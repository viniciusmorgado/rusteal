use bindings::engine::{
    EAttachmentRule, EDetachmentRule, GameplayStatics, HapticFeedbackEffect_Base,
    PlayerControllerExt, PrimitiveComponent, PrimitiveComponentExt, SceneComponent,
    SceneComponentExt,
};
use bindings::head_mounted_display::MotionControllerComponent;
use bindings::head_mounted_display::MotionControllerComponentExt;
use bindings::input_core::EControllerHand;
use bindings::prelude::*;
use rusteal_runtime::runtime::{FName, LOG_ERROR, OwnedStruct, RustealResult, UObjectRef, ulog};
use rusteal_runtime::{uclass, uclass_impl, uenum};

use crate::model;

#[uenum]
pub enum EGrabType {
    None = 0,
    Snap = 1,
    Custom = 3,
    Free = 4,
}

#[uclass(parent = SceneComponent)]
pub struct GrabComponent {
    #[uproperty(BlueprintReadOnly, category = "Grab")]
    b_is_held: bool,

    #[uproperty(BlueprintReadOnly, category = "Grab")]
    motion_controller_ref: UObjectRef<MotionControllerComponent>,

    #[uproperty(BlueprintReadWrite, category = "Grab")]
    primary_grab_component: UObjectRef<GrabComponent>,

    #[uproperty(BlueprintReadWrite, category = "Grab")]
    primary_grab_relative_rotation: OwnedStruct<FRotator>,

    #[uproperty(BlueprintReadWrite, category = "Grab")]
    b_simulate_on_drop: bool,

    #[uproperty(EditAnywhere, BlueprintReadWrite, category = "Grab", default = EGrabType::Free)]
    grab_type: EGrabType,

    #[uproperty(EditAnywhere, BlueprintReadWrite, category = "Grab")]
    on_grab_haptic_effect: UObjectRef<HapticFeedbackEffect_Base>,
}

#[uclass_impl]
impl GrabComponent {
    #[udelegate]
    pub fn on_grabbed(&self) {}

    #[udelegate]
    pub fn on_dropped(&self) {}

    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        self.set_should_simulate_on_drop();

        if let Ok(parent) = self.attach_parent() {
            parent.set_collision_profile_name(FName::new("PhysicsActor").handle(), Some(true));
        }
    }

    #[ufunction(BlueprintCallable)]
    pub fn try_grab(&mut self, motion_controller: UObjectRef<MotionControllerComponent>) -> bool {
        match self.grab_type() {
            EGrabType::Free => {
                self.set_primitive_comp_physics(false);
                self.attach_parent_to_motion_controller(motion_controller);
                self.set_b_is_held(true);
            }
            EGrabType::Snap => {
                self.set_primitive_comp_physics(false);
                self.attach_parent_to_motion_controller(motion_controller);
                self.set_b_is_held(true);

                if let Err(e) = self.snap_to(motion_controller) {
                    ulog!(LOG_ERROR, "[VR] snap grab failed: {e}");
                }
            }
            EGrabType::Custom => self.set_b_is_held(true),
            EGrabType::None => {}
        }

        if !self.b_is_held() {
            return false;
        }

        self.set_motion_controller_ref(motion_controller);
        self.on_grabbed();

        let me = self.as_ref();
        let controller = GameplayStatics::get_player_controller(me.upcast_to(), 0);

        if let Ok(controller) = controller.checked() {
            controller.play_haptic_effect(
                self.on_grab_haptic_effect(),
                self.get_held_by_hand(),
                Some(1.0),
                Some(false),
            );
        }

        true
    }

    #[ufunction(BlueprintCallable)]
    pub fn try_release(&mut self) -> bool {
        match self.grab_type() {
            EGrabType::Free | EGrabType::Snap => {
                if self.b_simulate_on_drop() {
                    self.set_primitive_comp_physics(true);
                }

                if let Ok(parent) = self.attach_parent() {
                    parent.k2_detach_from_component(
                        Some(EDetachmentRule::KeepWorld),
                        Some(EDetachmentRule::KeepWorld),
                        Some(EDetachmentRule::KeepWorld),
                        Some(true),
                    );
                }

                self.set_b_is_held(false);
            }
            EGrabType::Custom => self.set_b_is_held(false),
            EGrabType::None => {}
        }

        if self.b_is_held() {
            return false;
        }

        self.on_dropped();
        true
    }

    #[ufunction(BlueprintPure)]
    pub fn get_held_by_hand(&self) -> EControllerHand {
        let source = self
            .motion_controller_ref()
            .checked()
            .map(|controller| FName(controller.get_motion_source()).to_string_lossy())
            .unwrap_or_default();

        match model::hand_of_motion_source(&source) {
            model::Hand::Left => EControllerHand::Left,
            model::Hand::Right => EControllerHand::Right,
        }
    }

    #[ufunction(BlueprintCallable)]
    pub fn set_should_simulate_on_drop(&mut self) {
        if let Ok(parent) = self.attach_parent()
            && parent.is_any_simulating_physics()
        {
            self.set_b_simulate_on_drop(true);
        }
    }

    #[ufunction(BlueprintCallable)]
    pub fn set_primitive_comp_physics(&self, b_simulate: bool) {
        match self.attach_parent() {
            Ok(parent) => parent.set_simulate_physics(b_simulate),
            Err(_) => ulog!(
                LOG_ERROR,
                "GrabComponent->SetSimulatingParent->Cast To PrimitiveComponent FAILED"
            ),
        }
    }
}

impl GrabComponent {
    fn scene(&self) -> RustealResult<rusteal_runtime::runtime::Checked<SceneComponent>> {
        self.as_ref().upcast_to::<SceneComponent>().checked()
    }

    fn attach_parent(
        &self,
    ) -> RustealResult<rusteal_runtime::runtime::Checked<PrimitiveComponent>> {
        self.scene()?
            .get_attach_parent()
            .cast::<PrimitiveComponent>()?
            .checked()
    }

    fn attach_parent_to_motion_controller(
        &self,
        motion_controller: UObjectRef<MotionControllerComponent>,
    ) {
        let Ok(parent) = self.scene().map(|scene| scene.get_attach_parent()) else {
            return;
        };

        let attached = parent.checked().is_ok_and(|parent| {
            parent.k2_attach_to_component(
                motion_controller.upcast_to(),
                FName::NONE.handle(),
                EAttachmentRule::KeepWorld,
                EAttachmentRule::KeepWorld,
                EAttachmentRule::KeepWorld,
                true,
            )
        });

        if !attached {
            ulog!(
                LOG_ERROR,
                "Attaching the grabbed object to its motion controller FAILED - object not grabbed"
            );
        }
    }

    fn snap_to(
        &self,
        motion_controller: UObjectRef<MotionControllerComponent>,
    ) -> RustealResult<()> {
        let me = self.scene()?;
        let parent = me.get_attach_parent().checked()?;
        let rotation = me.get_relative_rotation().to_rotator();

        parent.k2_set_relative_rotation(
            &FRotator::from_rotator(rusteal_runtime::runtime::Rotator::new(
                -rotation.pitch,
                -rotation.yaw,
                -rotation.roll,
            )),
            false,
            true,
        );

        let controller = motion_controller
            .upcast_to::<SceneComponent>()
            .checked()?
            .k2_get_component_location()
            .to_dvec3();
        let offset = me.k2_get_component_location().to_dvec3()
            - parent.k2_get_component_location().to_dvec3();

        parent.k2_set_world_location(&FVector::from_dvec3(controller - offset), false, true);
        Ok(())
    }
}
