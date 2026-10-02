// FirstPersonCharacter: the First Person template's `ATP_FirstPersonCharacter` in Rust.
//
// What the C++ constructor does happens in two places: the components and
// their attachment are declared on the struct, and the values it sets on
// inherited properties are written by `#[class_defaults]` on the class default
// object, which every instance and Blueprint child starts from. What
// `SetupPlayerInputComponent` does happens in `ReceiveRestarted`, which a pawn
// gets right after its player input component is set up. The arms, the
// mannequin, their animation and the input actions come from its Blueprint
// child `BP_FirstPersonCharacter`, as they do for the C++ class.

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
    /// Pawn mesh: first person view (arms; seen only by self)
    #[component(attach = "mesh", name = "First Person Mesh")]
    first_person_mesh: SkeletalMeshComponent,

    /// First person camera
    #[component(attach = "first_person_mesh", socket = "head", name = "First Person Camera")]
    first_person_camera_component: CameraComponent,

    /// Jump Input Action
    #[uproperty(EditAnywhere, category = "Input")]
    jump_action: UObjectRef<InputAction>,

    /// Move Input Action
    #[uproperty(EditAnywhere, category = "Input")]
    move_action: UObjectRef<InputAction>,

    /// Look Input Action
    #[uproperty(EditAnywhere, category = "Input")]
    look_action: UObjectRef<InputAction>,

    /// Mouse Look Input Action
    #[uproperty(EditAnywhere, category = "Input")]
    mouse_look_action: UObjectRef<InputAction>,
}

#[uclass_impl]
impl FirstPersonCharacter {
    #[ufunction(Override)]
    pub(crate) fn receive_begin_play(&mut self) {
        ulog!(LOG_DISPLAY, "[FirstPerson] {} ready", self.name());
    }

    /// Everything `ATP_FirstPersonCharacter::ATP_FirstPersonCharacter()` sets on
    /// inherited properties and on the components, written once on the class
    /// default object.
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        let me = self.as_ref().checked()?;

        // Create the first person mesh that will be viewed only by this character's owner
        let first_person_mesh = self.first_person_mesh()?.checked()?;
        first_person_mesh.set_only_owner_see(true);
        first_person_mesh.set_first_person_primitive_type(EFirstPersonPrimitiveType::FirstPerson);
        first_person_mesh.set_collision_profile_name(FName::new("NoCollision").handle(), Some(true));

        // Create the Camera Component
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

        // configure the character comps
        let mesh = me.get_mesh().checked()?;
        mesh.set_owner_no_see(true);
        mesh.set_first_person_primitive_type(EFirstPersonPrimitiveType::WorldSpaceRepresentation);

        me.get_capsule_component().checked()?.set_capsule_size(
            model::CAPSULE_RADIUS,
            model::CAPSULE_HALF_HEIGHT,
            Some(false),
        );

        // Configure character movement
        let movement = me.get_character_movement().checked()?;
        movement.set_braking_deceleration_falling(model::BRAKING_DECELERATION_FALLING);
        movement.set_air_control(model::AIR_CONTROL);

        Ok(())
    }

    /// The pawn's player input component is set up: bind the input actions.
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

    /// Called from Input Actions for movement input
    #[ufunction]
    fn move_input(&mut self, value: UStructRef<FInputActionValue>) {
        // get the Vector2D move axis
        let movement_vector = value.axis2d();

        // pass the axis values to the move input
        self.do_move(movement_vector.x as f32, movement_vector.y as f32);
    }

    /// Called from Input Actions for looking input
    #[ufunction]
    fn look_input(&mut self, value: UStructRef<FInputActionValue>) {
        // get the Vector2D look axis
        let look_axis_vector = value.axis2d();

        // pass the axis values to the aim input
        self.do_aim(look_axis_vector.x as f32, look_axis_vector.y as f32);
    }

    /// Handles aim inputs from either controls or UI interfaces
    #[ufunction(BlueprintCallable)]
    fn do_aim(&mut self, yaw: f32, pitch: f32) {
        let Ok(me) = self.as_ref().checked() else {
            return;
        };
        if me.get_controller().is_valid() {
            // pass the rotation inputs
            me.add_controller_yaw_input(yaw);
            me.add_controller_pitch_input(pitch);
        }
    }

    /// Handles move inputs from either controls or UI interfaces
    #[ufunction(BlueprintCallable)]
    fn do_move(&mut self, right: f32, forward: f32) {
        let Ok(me) = self.as_ref().checked() else {
            return;
        };
        if me.get_controller().is_valid() {
            // pass the move inputs
            me.add_movement_input(&me.get_actor_right_vector(), Some(right), Some(false));
            me.add_movement_input(&me.get_actor_forward_vector(), Some(forward), Some(false));
        }
    }

    /// Handles jump start inputs from either controls or UI interfaces
    #[ufunction(BlueprintCallable)]
    fn do_jump_start(&mut self) {
        // pass Jump to the character
        if let Ok(me) = self.as_ref().checked() {
            me.jump();
        }
    }

    /// Handles jump end inputs from either controls or UI interfaces
    #[ufunction(BlueprintCallable)]
    fn do_jump_end(&mut self) {
        // pass StopJumping to the character
        if let Ok(me) = self.as_ref().checked() {
            me.stop_jumping();
        }
    }
}

impl FirstPersonCharacter {
    /// Everything `ATP_FirstPersonCharacter::SetupPlayerInputComponent` does. Only
    /// a locally controlled player pawn has an input component; `false` for
    /// any other.
    fn bind_input(&self) -> RustealResult<bool> {
        let me = self.as_ref();
        let pawn = me.checked()?;
        if !pawn.is_player_controlled() || !pawn.is_locally_controlled() {
            return Ok(false);
        }

        // Jumping
        bind_action(&me, self.jump_action(), ETriggerEvent::Started, "DoJumpStart")?;
        bind_action(&me, self.jump_action(), ETriggerEvent::Completed, "DoJumpEnd")?;

        // Moving
        bind_action(&me, self.move_action(), ETriggerEvent::Triggered, "MoveInput")?;

        // Looking/Aiming
        bind_action(&me, self.look_action(), ETriggerEvent::Triggered, "LookInput")?;
        bind_action(&me, self.mouse_look_action(), ETriggerEvent::Triggered, "LookInput")?;

        Ok(true)
    }

    fn name(&self) -> String {
        self.as_ref()
            .get_name()
            .unwrap_or_else(|_| "FirstPersonCharacter".into())
    }
}
