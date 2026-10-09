mod model;

use bindings::engine::{
    CameraComponent, CameraComponentExt, CapsuleComponentExt, Character, CharacterExt,
    CharacterMovementComponentExt, EFirstPersonPrimitiveType, PawnExt, PrimitiveComponentExt,
    SceneComponentExt, SkeletalMeshComponent,
};
use bindings::enhanced_input::{ETriggerEvent, FInputActionValue, InputAction};
use bindings::prelude::*;
use rusteal_runtime::runtime::{
    FName, LOG_DISPLAY, LOG_WARNING, Rotator, RustealResult, UObjectRef, UStructRef, ulog,
};
use rusteal_runtime::{uclass, uclass_impl};

#[uclass(parent = Character)]
pub struct FirstPersonCharacter {
    #[component(attach = "mesh", name = "First Person Mesh")]
    first_person_mesh: SkeletalMeshComponent,

    #[component(
        attach = "first_person_mesh",
        socket = "head",
        name = "First Person Camera"
    )]
    first_person_camera_component: CameraComponent,

    #[uproperty(EditAnywhere, category = "Input")]
    jump_action: UObjectRef<InputAction>,

    #[uproperty(EditAnywhere, category = "Input")]
    move_action: UObjectRef<InputAction>,

    #[uproperty(EditAnywhere, category = "Input")]
    look_action: UObjectRef<InputAction>,

    #[uproperty(EditAnywhere, category = "Input")]
    mouse_look_action: UObjectRef<InputAction>,
}

#[uclass_impl]
impl FirstPersonCharacter {
    #[ufunction(Override)]
    pub(crate) fn receive_begin_play(&mut self) {
        ulog!(LOG_DISPLAY, "[FirstPerson] {} ready", self.name());
    }

    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        let me = self.as_ref().checked()?;

        let first_person_mesh = self.first_person_mesh()?.checked()?;
        first_person_mesh.set_only_owner_see(true);
        first_person_mesh.set_first_person_primitive_type(EFirstPersonPrimitiveType::FirstPerson);

        first_person_mesh
            .set_collision_profile_name(FName::new("NoCollision").handle(), Some(true));

        let camera = self.first_person_camera_component()?.checked()?;

        camera.k2_set_relative_location_and_rotation(
            &FVector::from_dvec3(model::CAMERA_LOCATION),
            &FRotator::from_rotator(Rotator::new(
                model::CAMERA_PITCH,
                model::CAMERA_YAW,
                model::CAMERA_ROLL,
            )),
            false,
            false,
        );

        camera.set_use_pawn_control_rotation(true);
        camera.set_enable_first_person_field_of_view(true);
        camera.set_enable_first_person_scale(true);
        camera.set_first_person_field_of_view(model::FIRST_PERSON_FIELD_OF_VIEW);
        camera.set_first_person_scale(model::FIRST_PERSON_SCALE);

        let mesh = me.get_mesh().checked()?;
        mesh.set_owner_no_see(true);
        mesh.set_first_person_primitive_type(EFirstPersonPrimitiveType::WorldSpaceRepresentation);

        me.get_capsule_component().checked()?.set_capsule_size(
            model::CAPSULE_RADIUS,
            model::CAPSULE_HALF_HEIGHT,
            Some(false),
        );

        let movement = me.get_character_movement().checked()?;
        movement.set_braking_deceleration_falling(model::BRAKING_DECELERATION_FALLING);
        movement.set_air_control(model::AIR_CONTROL);

        Ok(())
    }

    #[ufunction(Override)]
    pub(crate) fn receive_restarted(&mut self) {
        match self.bind_input() {
            Ok(true) => ulog!(LOG_DISPLAY, "[FirstPerson] {} input bound", self.name()),
            Ok(false) => {}
            Err(e) => ulog!(
                LOG_WARNING,
                "[FirstPerson] {} failed to bind its input: {e}",
                self.name()
            ),
        }
    }

    #[ufunction]
    fn move_input(&mut self, value: UStructRef<FInputActionValue>) {
        let movement_vector = value.axis2d();

        self.do_move(movement_vector.x as f32, movement_vector.y as f32);
    }

    #[ufunction]
    fn look_input(&mut self, value: UStructRef<FInputActionValue>) {
        let look_axis_vector = value.axis2d();

        self.do_aim(look_axis_vector.x as f32, look_axis_vector.y as f32);
    }

    #[ufunction(BlueprintCallable)]
    pub(crate) fn do_aim(&mut self, yaw: f32, pitch: f32) {
        let Ok(me) = self.as_ref().checked() else {
            return;
        };

        if me.get_controller().is_valid() {
            me.add_controller_yaw_input(yaw);
            me.add_controller_pitch_input(pitch);
        }
    }

    #[ufunction(BlueprintCallable)]
    pub(crate) fn do_move(&mut self, right: f32, forward: f32) {
        let Ok(me) = self.as_ref().checked() else {
            return;
        };

        if me.get_controller().is_valid() {
            me.add_movement_input(&me.get_actor_right_vector(), Some(right), Some(false));
            me.add_movement_input(&me.get_actor_forward_vector(), Some(forward), Some(false));
        }
    }

    #[ufunction(BlueprintCallable)]
    pub(crate) fn do_jump_start(&mut self) {
        if let Ok(me) = self.as_ref().checked() {
            me.jump();
        }
    }

    #[ufunction(BlueprintCallable)]
    pub(crate) fn do_jump_end(&mut self) {
        if let Ok(me) = self.as_ref().checked() {
            me.stop_jumping();
        }
    }
}

impl FirstPersonCharacter {
    fn bind_input(&self) -> RustealResult<bool> {
        let me = self.as_ref();
        let pawn = me.checked()?;

        if !pawn.is_player_controlled() || !pawn.is_locally_controlled() {
            return Ok(false);
        }

        bind_action(
            &me,
            self.jump_action(),
            ETriggerEvent::Started,
            "DoJumpStart",
        )?;

        bind_action(
            &me,
            self.jump_action(),
            ETriggerEvent::Completed,
            "DoJumpEnd",
        )?;

        bind_action(
            &me,
            self.move_action(),
            ETriggerEvent::Triggered,
            "MoveInput",
        )?;

        bind_action(
            &me,
            self.look_action(),
            ETriggerEvent::Triggered,
            "LookInput",
        )?;

        bind_action(
            &me,
            self.mouse_look_action(),
            ETriggerEvent::Triggered,
            "LookInput",
        )?;

        Ok(true)
    }

    fn name(&self) -> String {
        self.as_ref()
            .get_name()
            .unwrap_or_else(|_| "FirstPersonCharacter".into())
    }
}
