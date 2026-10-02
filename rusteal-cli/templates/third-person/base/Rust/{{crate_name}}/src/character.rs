// ThirdPersonCharacter: the Third Person template's `ATP_ThirdPersonCharacter` in Rust.
//
// What the C++ constructor does happens in two places: the components and
// their attachment are declared on the struct, and the values it sets on
// inherited properties are written by `#[class_defaults]` on the class default
// object, which every instance and Blueprint child starts from. What
// `SetupPlayerInputComponent` does happens in `ReceiveRestarted`, which a pawn
// gets right after its player input component is set up. The mannequin, its
// animation and the input actions come from its Blueprint child
// `BP_ThirdPersonCharacter`, as they do for the C++ class.

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
    /// Camera boom positioning the camera behind the character.
    #[component(attach = "root_component")]
    camera_boom: SpringArmComponent,

    /// Follow camera, at the end of the boom (`USpringArmComponent::SocketName`).
    #[component(attach = "camera_boom", socket = "SpringEndpoint")]
    follow_camera: CameraComponent,

    /// Jump input action, assigned in the Blueprint child.
    #[uproperty(EditAnywhere)]
    jump_action: UObjectRef<InputAction>,

    /// Move input action.
    #[uproperty(EditAnywhere)]
    move_action: UObjectRef<InputAction>,

    /// Look input action.
    #[uproperty(EditAnywhere)]
    look_action: UObjectRef<InputAction>,

    /// Mouse look input action.
    #[uproperty(EditAnywhere)]
    mouse_look_action: UObjectRef<InputAction>,
}

#[uclass_impl]
impl ThirdPersonCharacter {
    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        ulog!(LOG_DISPLAY, "[ThirdPerson] {} ready", self.name());
    }

    /// Everything `ATP_ThirdPersonCharacter::ATP_ThirdPersonCharacter()` sets on
    /// inherited properties and on the components, written once on the class
    /// default object.
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        let me = self.as_ref().checked()?;

        // Set size for collision capsule
        me.get_capsule_component().checked()?.set_capsule_size(
            model::CAPSULE_RADIUS,
            model::CAPSULE_HALF_HEIGHT,
            Some(false),
        );

        // Don't rotate when the controller rotates. Let that just affect the camera.
        me.set_use_controller_rotation_pitch(false);
        me.set_use_controller_rotation_yaw(false);
        me.set_use_controller_rotation_roll(false);

        // Configure character movement
        let movement = me.get_character_movement().checked()?;
        movement.set_orient_rotation_to_movement(true);
        movement.set_rotation_rate(&FRotator::from_rotator(Rotator::new(
            0.0,
            model::ROTATION_RATE_YAW,
            0.0,
        )));

        // Note: For faster iteration times these variables, and many more, can be tweaked in the Character Blueprint
        // instead of recompiling to adjust them
        movement.set_jump_z_velocity(model::JUMP_Z_VELOCITY);
        movement.set_air_control(model::AIR_CONTROL);
        movement.set_max_walk_speed(model::MAX_WALK_SPEED);
        movement.set_min_analog_walk_speed(model::MIN_ANALOG_WALK_SPEED);
        movement.set_braking_deceleration_walking(model::BRAKING_DECELERATION_WALKING);
        movement.set_braking_deceleration_falling(model::BRAKING_DECELERATION_FALLING);

        // Create a camera boom (pulls in towards the player if there is a collision)
        let boom = self.camera_boom()?.checked()?;
        boom.set_target_arm_length(model::CAMERA_BOOM_LENGTH); // The camera follows at this distance behind the character
        boom.set_use_pawn_control_rotation(true); // Rotate the arm based on the controller

        // Create a follow camera
        self.follow_camera()?.checked()?.set_use_pawn_control_rotation(false); // Camera does not rotate relative to arm

        Ok(())
    }

    /// The pawn's player input component is set up: bind the input actions.
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

    /// Called for movement input.
    #[ufunction(BlueprintCallable)]
    fn r#move(&mut self, value: UStructRef<FInputActionValue>) {
        // input is a Vector2D
        let movement = value.axis2d();
        self.do_move(movement.x as f32, movement.y as f32);
    }

    /// Called for looking input.
    #[ufunction(BlueprintCallable)]
    fn look(&mut self, value: UStructRef<FInputActionValue>) {
        // input is a Vector2D
        let look_axis = value.axis2d();
        self.do_look(look_axis.x as f32, look_axis.y as f32);
    }

    /// Handles move inputs from either controls or UI interfaces.
    #[ufunction(BlueprintCallable)]
    fn do_move(&mut self, right: f32, forward: f32) {
        if let Err(e) = self.apply_move(MoveInput { right, forward }) {
            ulog!(LOG_WARNING, "[ThirdPerson] DoMove failed: {e}");
        }
    }

    /// Handles look inputs from either controls or UI interfaces.
    #[ufunction(BlueprintCallable)]
    fn do_look(&mut self, yaw: f32, pitch: f32) {
        if let Err(e) = self.apply_look(LookInput { yaw, pitch }) {
            ulog!(LOG_WARNING, "[ThirdPerson] DoLook failed: {e}");
        }
    }

    /// Handles jump pressed inputs from either controls or UI interfaces.
    #[ufunction(BlueprintCallable)]
    fn do_jump_start(&mut self) {
        if let Ok(me) = self.as_ref().checked() {
            me.jump();
        }
    }

    /// Handles jump released inputs from either controls or UI interfaces.
    #[ufunction(BlueprintCallable)]
    fn do_jump_end(&mut self) {
        if let Ok(me) = self.as_ref().checked() {
            me.stop_jumping();
        }
    }
}

impl ThirdPersonCharacter {
    /// Everything `ATP_ThirdPersonCharacter::SetupPlayerInputComponent` does. Only
    /// a locally controlled player pawn has an input component; `false` for
    /// any other.
    fn bind_input(&self) -> RustealResult<bool> {
        let me = self.as_ref();
        let pawn = me.checked()?;
        if !pawn.is_player_controlled() || !pawn.is_locally_controlled() {
            return Ok(false);
        }

        // Jumping
        bind_action(&me, self.jump_action(), ETriggerEvent::Started, "Jump")?;
        bind_action(&me, self.jump_action(), ETriggerEvent::Completed, "StopJumping")?;

        // Moving
        bind_action(&me, self.move_action(), ETriggerEvent::Triggered, "Move")?;
        bind_action(&me, self.mouse_look_action(), ETriggerEvent::Triggered, "Look")?;

        // Looking
        bind_action(&me, self.look_action(), ETriggerEvent::Triggered, "Look")?;

        Ok(true)
    }

    /// `DoMove`: move along the controller's yaw, ignoring its pitch.
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

    /// `DoLook`: add yaw and pitch input to the controller.
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
