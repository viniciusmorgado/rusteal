mod model;

use bindings::engine::{
    CameraComponent, CameraComponentExt, CapsuleComponentExt, Character, CharacterExt,
    CharacterMovementComponentExt, ControllerExt, PawnExt, SpringArmComponent,
    SpringArmComponentExt,
};
use bindings::enhanced_input::{ETriggerEvent, FInputActionValue, InputAction};
use bindings::prelude::*;
use rusteal_runtime::runtime::{
    LOG_DISPLAY, LOG_WARNING, Rotator, RustealResult, UObjectRef, UStructRef, ulog,
};
use rusteal_runtime::{uclass, uclass_impl};

use model::{LookInput, MoveInput};

#[uclass(parent = Character)]
pub struct ThirdPersonCharacter {
    #[component(attach = "root_component")]
    camera_boom: SpringArmComponent,

    #[component(attach = "camera_boom", socket = "SpringEndpoint")]
    follow_camera: CameraComponent,

    #[uproperty(EditAnywhere)]
    jump_action: UObjectRef<InputAction>,

    #[uproperty(EditAnywhere)]
    move_action: UObjectRef<InputAction>,

    #[uproperty(EditAnywhere)]
    look_action: UObjectRef<InputAction>,

    #[uproperty(EditAnywhere)]
    mouse_look_action: UObjectRef<InputAction>,
}

#[uclass_impl]
impl ThirdPersonCharacter {
    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        ulog!(LOG_DISPLAY, "[ThirdPerson] {} ready", self.name());
    }

    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        let me = self.as_ref().checked()?;

        me.get_capsule_component().checked()?.set_capsule_size(
            model::CAPSULE_RADIUS,
            model::CAPSULE_HALF_HEIGHT,
            Some(false),
        );

        me.set_use_controller_rotation_pitch(false);
        me.set_use_controller_rotation_yaw(false);
        me.set_use_controller_rotation_roll(false);

        let movement = me.get_character_movement().checked()?;
        movement.set_orient_rotation_to_movement(true);

        movement.set_rotation_rate(&FRotator::from_rotator(Rotator::new(
            0.0,
            model::ROTATION_RATE_YAW,
            0.0,
        )));

        movement.set_jump_z_velocity(model::JUMP_Z_VELOCITY);
        movement.set_air_control(model::AIR_CONTROL);
        movement.set_max_walk_speed(model::MAX_WALK_SPEED);
        movement.set_min_analog_walk_speed(model::MIN_ANALOG_WALK_SPEED);
        movement.set_braking_deceleration_walking(model::BRAKING_DECELERATION_WALKING);
        movement.set_braking_deceleration_falling(model::BRAKING_DECELERATION_FALLING);

        let boom = self.camera_boom()?.checked()?;
        boom.set_target_arm_length(model::CAMERA_BOOM_LENGTH);
        boom.set_use_pawn_control_rotation(true);

        self.follow_camera()?
            .checked()?
            .set_use_pawn_control_rotation(false);

        Ok(())
    }

    #[ufunction(Override)]
    fn receive_restarted(&mut self) {
        match self.bind_input() {
            Ok(true) => ulog!(LOG_DISPLAY, "[ThirdPerson] {} input bound", self.name()),
            Ok(false) => {}
            Err(e) => ulog!(
                LOG_WARNING,
                "[ThirdPerson] {} failed to bind its input: {e}",
                self.name()
            ),
        }
    }

    #[ufunction(BlueprintCallable)]
    fn r#move(&mut self, value: UStructRef<FInputActionValue>) {
        let movement = value.axis2d();
        self.do_move(movement.x as f32, movement.y as f32);
    }

    #[ufunction(BlueprintCallable)]
    fn look(&mut self, value: UStructRef<FInputActionValue>) {
        let look_axis = value.axis2d();
        self.do_look(look_axis.x as f32, look_axis.y as f32);
    }

    #[ufunction(BlueprintCallable)]
    fn do_move(&mut self, right: f32, forward: f32) {
        if let Err(e) = self.apply_move(MoveInput { right, forward }) {
            ulog!(LOG_WARNING, "[ThirdPerson] DoMove failed: {e}");
        }
    }

    #[ufunction(BlueprintCallable)]
    fn do_look(&mut self, yaw: f32, pitch: f32) {
        if let Err(e) = self.apply_look(LookInput { yaw, pitch }) {
            ulog!(LOG_WARNING, "[ThirdPerson] DoLook failed: {e}");
        }
    }

    #[ufunction(BlueprintCallable)]
    fn do_jump_start(&mut self) {
        if let Ok(me) = self.as_ref().checked() {
            me.jump();
        }
    }

    #[ufunction(BlueprintCallable)]
    fn do_jump_end(&mut self) {
        if let Ok(me) = self.as_ref().checked() {
            me.stop_jumping();
        }
    }
}

impl ThirdPersonCharacter {
    fn bind_input(&self) -> RustealResult<bool> {
        let me = self.as_ref();
        let pawn = me.checked()?;

        if !pawn.is_player_controlled() || !pawn.is_locally_controlled() {
            return Ok(false);
        }

        bind_action(&me, self.jump_action(), ETriggerEvent::Started, "Jump")?;

        bind_action(
            &me,
            self.jump_action(),
            ETriggerEvent::Completed,
            "StopJumping",
        )?;

        bind_action(&me, self.move_action(), ETriggerEvent::Triggered, "Move")?;

        bind_action(
            &me,
            self.mouse_look_action(),
            ETriggerEvent::Triggered,
            "Look",
        )?;

        bind_action(&me, self.look_action(), ETriggerEvent::Triggered, "Look")?;

        Ok(true)
    }

    fn apply_move(&self, input: MoveInput) -> RustealResult<()> {
        let me = self.as_ref().checked()?;
        let controller = me.get_controller();

        if !controller.is_valid() {
            return Ok(());
        }

        let yaw = controller
            .checked()?
            .get_control_rotation()
            .to_rotator()
            .yaw;

        let (forward, right) = model::ground_axes(yaw);

        me.add_movement_input(
            &FVector::from_dvec3(forward),
            Some(input.forward),
            Some(false),
        );

        me.add_movement_input(&FVector::from_dvec3(right), Some(input.right), Some(false));

        Ok(())
    }

    fn apply_look(&self, input: LookInput) -> RustealResult<()> {
        let me = self.as_ref().checked()?;

        if !me.get_controller().is_valid() {
            return Ok(());
        }

        me.add_controller_yaw_input(input.yaw);
        me.add_controller_pitch_input(input.pitch);

        Ok(())
    }

    fn name(&self) -> String {
        self.as_ref()
            .get_name()
            .unwrap_or_else(|_| "ThirdPersonCharacter".into())
    }
}
