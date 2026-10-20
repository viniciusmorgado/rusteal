use bindings::engine::{
    Actor, ActorComponentExt, ActorExt, GameplayStatics, HapticFeedbackEffect_Base,
    PlayerControllerExt, PrimitiveComponentExt, SceneComponent, SceneComponentExt,
    StaticMeshComponent,
};
use bindings::enhanced_input::{
    ETriggerEvent, EnhancedInputLocalPlayerSubsystemExt, InputAction, InputMappingContext,
};
use bindings::input_core::EControllerHand;
use bindings::prelude::*;
use glam::DVec3;
use rusteal_runtime::runtime::{
    DynamicCall, FName, OwnedStruct, Rotator, RustealResult, SubclassOf, UObjectRef,
};
use rusteal_runtime::{uclass, uclass_impl};

use crate::grab_component::{EGrabType, GrabComponent};
use crate::projectile::Projectile;

#[uclass(parent = Actor)]
pub struct Pistol {
    #[component(root, name = "SM_Pistol")]
    sm_pistol: StaticMeshComponent,

    #[component(attach = "sm_pistol")]
    grab_component_snap: GrabComponent,

    #[component(attach = "sm_pistol")]
    muzzle_location: SceneComponent,

    #[uproperty(EditAnywhere, category = "Pistol")]
    pistol_fire_haptic_effect: UObjectRef<HapticFeedbackEffect_Base>,

    #[uproperty(EditAnywhere, category = "Pistol")]
    projectile_class: SubclassOf<Projectile>,

    #[uproperty(EditAnywhere, category = "Input")]
    weapon_left_mapping_context: UObjectRef<InputMappingContext>,

    #[uproperty(EditAnywhere, category = "Input")]
    weapon_right_mapping_context: UObjectRef<InputMappingContext>,

    #[uproperty(EditAnywhere, category = "Input")]
    shoot_left_action: UObjectRef<InputAction>,

    #[uproperty(EditAnywhere, category = "Input")]
    shoot_right_action: UObjectRef<InputAction>,

    input_bound: bool,
}

#[uclass_impl]
impl Pistol {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        let mesh = self.sm_pistol()?.checked()?;
        mesh.set_simulate_physics(true);
        mesh.set_collision_profile_name(FName::new("PhysicsActor").handle(), Some(true));

        self.muzzle_location()?
            .checked()?
            .k2_set_relative_location_and_rotation(
                &FVector::from_dvec3(DVec3::new(0.000086, 19.223918, 8.321923)),
                &FRotator::from_rotator(Rotator::new(0.0, 90.000137, 0.0)),
                false,
                false,
            );

        let grab = self.grab_component_snap()?;
        GrabComponent::from_obj(grab)?.set_grab_type(EGrabType::Snap);

        grab.upcast_to::<SceneComponent>()
            .checked()?
            .k2_set_relative_location_and_rotation(
                &FVector::from_dvec3(DVec3::new(0.000006, 0.400918, -0.813376)),
                &FRotator::from_rotator(Rotator::new(85.000183, 90.000035, -0.000127)),
                false,
                false,
            );

        Ok(())
    }

    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        let Ok(grab) = self.grab_component_snap().and_then(GrabComponent::from_obj) else {
            return;
        };

        let me = self.as_ref();

        let grabbed = grab.on_grabbed_add(move || {
            if let Ok(mut pistol) = Pistol::from_obj(me)
                && let Err(e) = pistol.grabbed()
            {
                log::warn!("[VR] pistol grab failed: {e}");
            }
        });

        let dropped = grab.on_dropped_add(move || {
            if let Ok(pistol) = Pistol::from_obj(me)
                && let Err(e) = pistol.dropped()
            {
                log::warn!("[VR] pistol drop failed: {e}");
            }
        });

        for binding in [grabbed, dropped] {
            match binding {
                Ok(binding) => binding.detach(),
                Err(e) => log::warn!("[VR] pistol cannot watch its grab component: {e}"),
            }
        }
    }

    #[ufunction]
    fn shoot_left(&mut self) {
        self.shoot_with(EControllerHand::Left);
    }

    #[ufunction]
    fn shoot_right(&mut self) {
        self.shoot_with(EControllerHand::Right);
    }

    #[ufunction(BlueprintCallable)]
    pub fn fire(&mut self) {
        if let Err(e) = self.spawn_projectile() {
            log::warn!("[VR] the pistol did not fire: {e}");
        }
    }
}

impl Pistol {
    fn grab(&self) -> RustealResult<GrabComponent> {
        GrabComponent::from_obj(self.grab_component_snap()?)
    }

    fn held_by_right(&self) -> RustealResult<bool> {
        Ok(self.grab()?.get_held_by_hand() == EControllerHand::Right)
    }

    fn weapon_context(&self) -> RustealResult<UObjectRef<InputMappingContext>> {
        Ok(if self.held_by_right()? {
            self.weapon_right_mapping_context()
        } else {
            self.weapon_left_mapping_context()
        })
    }

    fn grabbed(&mut self) -> RustealResult<()> {
        let me = self.as_ref();
        let controller = GameplayStatics::get_player_controller(me.upcast_to(), 0);

        me.checked()?.enable_input(controller);

        if !self.input_bound() {
            bind_action_by_name(
                &me,
                self.shoot_left_action(),
                ETriggerEvent::Started,
                "ShootLeft",
            )?;
            bind_action_by_name(
                &me,
                self.shoot_right_action(),
                ETriggerEvent::Started,
                "ShootRight",
            )?;
            self.set_input_bound(true);
        }

        self.hide_hand(true)?;

        let subsystem = enhanced_input_subsystem(controller)?.checked()?;
        subsystem.add_mapping_context(self.weapon_context()?, 1, &OwnedStruct::new());
        Ok(())
    }

    fn dropped(&self) -> RustealResult<()> {
        let me = self.as_ref();
        let controller = GameplayStatics::get_player_controller(me.upcast_to(), 0);

        me.checked()?.disable_input(controller);
        self.hide_hand(false)?;

        let subsystem = enhanced_input_subsystem(controller)?.checked()?;
        subsystem.remove_mapping_context(self.weapon_context()?, &OwnedStruct::new());
        Ok(())
    }

    fn hide_hand(&self, hide: bool) -> RustealResult<()> {
        let grab = self.grab()?;
        let owner = grab.motion_controller_ref().checked()?.get_owner();

        let mut call = DynamicCall::new(&owner, "HideUnhideHand")?;
        call.set("bHide", hide)?;
        call.set("bRightHand", self.held_by_right()?)?;
        call.call()?;
        Ok(())
    }

    fn shoot_with(&mut self, hand: EControllerHand) {
        if self
            .grab()
            .is_ok_and(|grab| grab.get_held_by_hand() == hand)
        {
            self.fire();
        }
    }

    fn spawn_projectile(&self) -> RustealResult<()> {
        let me = self.as_ref();
        let muzzle = self.muzzle_location()?.checked()?;

        let transform = bindings::engine::KismetMathLibrary::make_transform(
            &muzzle.k2_get_component_location(),
            &muzzle.k2_get_component_rotation(),
            &FVector::from_dvec3(DVec3::ONE),
        );

        let pawn = GameplayStatics::get_player_pawn(me.upcast_to(), 0);
        let world = me.get_world()?;

        let params = SpawnParams {
            owner: Some(me.upcast_to()),
            instigator: Some(pawn),
            ..SpawnParams::default()
        };

        let projectile =
            world.spawn_actor_deferred_of_class(self.projectile_class(), &transform, params)?;

        world.finish_spawning(&projectile.upcast_to(), &transform)?;

        let controller = GameplayStatics::get_player_controller(me.upcast_to(), 0);

        if let Ok(controller) = controller.checked() {
            controller.play_haptic_effect(
                self.pistol_fire_haptic_effect(),
                self.grab()?.get_held_by_hand(),
                Some(1.0),
                Some(false),
            );
        }

        Ok(())
    }
}
