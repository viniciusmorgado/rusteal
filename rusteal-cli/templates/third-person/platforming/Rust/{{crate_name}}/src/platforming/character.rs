// PlatformingCharacter: the Platforming variant's `APlatformingCharacter` in
// Rust. A third person character with platforming movement physics: press
// and hold jump, double jump, wall jump, coyote time and a dash.
//
// What the C++ constructor sets on inherited properties is written by
// `#[class_defaults]`; the C++ virtuals it overrides are their Blueprint
// events here: `Landed` is `OnLanded`, `OnMovementModeChanged` is
// `K2_OnMovementModeChanged`, `EndPlay` is `ReceiveEndPlay`, and
// `SetupPlayerInputComponent` is `ReceiveRestarted`. The Blueprint child
// `BP_PlatformingCharacter` holds the mannequin, the input actions, the dash
// montage and the jump trails (`SetJumpTrailState`), and has its own
// `BeginPlay`, so this class leaves that event to it.

use bindings::engine::{
    AnimInstanceExt, AnimMontage, CameraComponent, CameraComponentExt, CapsuleComponentExt,
    Character, CharacterExt, CharacterMovementComponentExt, ControllerExt, EDrawDebugTrace,
    ETraceTypeQuery, FHitResult, FHitResultExt, GameplayStatics, KismetSystemLibrary,
    NavMovementComponentExt, PawnExt, SkeletalMeshComponentExt, SpringArmComponent,
    SpringArmComponentExt,
};
use bindings::engine::{FNavAgentPropertiesExt, MovementComponentExt};
use bindings::enhanced_input::{ETriggerEvent, FInputActionValue, InputAction};
use bindings::prelude::*;
use rusteal_runtime::runtime::{
    LOG_DISPLAY, LOG_WARNING, OwnedStruct, Rotator, RustealResult, UObjectRef, UStructRef, ulog,
};
use rusteal_runtime::{uclass, uclass_impl};

use super::model::{self, AirJump, JumpPress, MoveState, ground_axes};

/// `EMovementMode::MOVE_Falling`.
const MOVE_FALLING: u8 = 3;

#[uclass(parent = Character)]
pub struct PlatformingCharacter {
    /// Camera boom positioning the camera behind the character
    #[component(attach = "root_component")]
    camera_boom: SpringArmComponent,

    /// Follow camera
    #[component(attach = "camera_boom", socket = "SpringEndpoint")]
    follow_camera: CameraComponent,

    /// Jump Input Action
    #[uproperty(EditAnywhere)]
    jump_action: UObjectRef<InputAction>,

    /// Move Input Action
    #[uproperty(EditAnywhere)]
    move_action: UObjectRef<InputAction>,

    /// Look Input Action
    #[uproperty(EditAnywhere)]
    look_action: UObjectRef<InputAction>,

    /// Mouse Look Input Action
    #[uproperty(EditAnywhere)]
    mouse_look_action: UObjectRef<InputAction>,

    /// Dash Input Action
    #[uproperty(EditAnywhere)]
    dash_action: UObjectRef<InputAction>,

    /// Distance to trace ahead of the character to look for walls to jump from
    #[uproperty(EditAnywhere, default = model::WALL_JUMP_TRACE_DISTANCE)]
    wall_jump_trace_distance: f32,

    /// Radius of the wall jump sphere trace check
    #[uproperty(EditAnywhere, default = model::WALL_JUMP_TRACE_RADIUS)]
    wall_jump_trace_radius: f32,

    /// Impulse to apply away from the wall when wall jumping
    #[uproperty(EditAnywhere, default = model::WALL_JUMP_BOUNCE_IMPULSE)]
    wall_jump_bounce_impulse: f32,

    /// Vertical impulse to apply when wall jumping
    #[uproperty(EditAnywhere, default = model::WALL_JUMP_VERTICAL_IMPULSE)]
    wall_jump_vertical_impulse: f32,

    /// Time to ignore jump inputs after a wall jump
    #[uproperty(EditAnywhere, default = model::DELAY_BETWEEN_WALL_JUMPS)]
    delay_between_wall_jumps: f32,

    /// AnimMontage to use for the Dash action
    #[uproperty(EditAnywhere)]
    dash_montage: UObjectRef<AnimMontage>,

    /// Max amount of time that can pass since we started falling when we allow a regular jump
    #[uproperty(EditAnywhere, default = model::MAX_COYOTE_TIME)]
    max_coyote_time: f32,

    /// movement state flags and the last time this character started falling
    state: MoveState,

    /// whether the dash montage's end is already routed to `DashMontageEnded`
    dash_end_bound: bool,
}

#[uclass_impl]
impl PlatformingCharacter {
    /// Everything `APlatformingCharacter::APlatformingCharacter()` sets on
    /// inherited properties and on the components.
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        let me = self.as_ref().checked()?;

        // enable press and hold jump
        me.set_jump_max_hold_time(model::JUMP_MAX_HOLD_TIME);

        // set the jump max count to 3 so we can double jump and check for coyote time jumps
        me.set_jump_max_count(model::JUMP_MAX_COUNT);

        // Set size for collision capsule
        me.get_capsule_component().checked()?.set_capsule_size(
            model::CAPSULE_RADIUS,
            model::CAPSULE_HALF_HEIGHT,
            Some(false),
        );

        // don't rotate the mesh when the controller rotates
        me.set_use_controller_rotation_yaw(false);

        // Configure character movement
        let movement = me.get_character_movement().checked()?;
        movement.set_gravity_scale(model::GRAVITY_SCALE);
        movement.set_max_acceleration(model::MAX_ACCELERATION);
        movement.set_braking_friction_factor(model::BRAKING_FRICTION_FACTOR);
        movement.set_use_separate_braking_friction(true);
        movement.set_ground_friction(model::GROUND_FRICTION);
        movement.set_max_walk_speed(model::MAX_WALK_SPEED);
        movement.set_min_analog_walk_speed(model::MIN_ANALOG_WALK_SPEED);
        movement.set_braking_deceleration_walking(model::BRAKING_DECELERATION_WALKING);
        movement.set_perch_radius_threshold(model::PERCH_RADIUS_THRESHOLD);
        movement.set_jump_z_velocity(model::JUMP_Z_VELOCITY);
        movement.set_braking_deceleration_falling(model::BRAKING_DECELERATION_FALLING);
        movement.set_air_control(model::AIR_CONTROL);
        movement.set_rotation_rate(&FRotator::from_rotator(Rotator::new(
            0.0,
            model::ROTATION_RATE_YAW,
            0.0,
        )));
        movement.set_orient_rotation_to_movement(true);
        let nav = movement.get_nav_agent_props();
        nav.as_ref().set_agent_radius(model::NAV_AGENT_RADIUS);
        nav.as_ref().set_agent_height(model::NAV_AGENT_HEIGHT);
        movement.set_nav_agent_props(&nav);

        // create the camera boom
        let boom = self.camera_boom()?.checked()?;
        boom.set_target_arm_length(model::CAMERA_BOOM_LENGTH);
        boom.set_use_pawn_control_rotation(true);
        boom.set_enable_camera_lag(true);
        boom.set_camera_lag_speed(model::CAMERA_LAG_SPEED);
        boom.set_enable_camera_rotation_lag(true);
        boom.set_camera_rotation_lag_speed(model::CAMERA_ROTATION_LAG_SPEED);

        // create the orbiting camera
        self.follow_camera()?.checked()?.set_use_pawn_control_rotation(false);

        Ok(())
    }

    /// The pawn's player input component is set up: bind the input actions.
    #[ufunction(Override)]
    fn receive_restarted(&mut self) {
        match self.bind_input() {
            Ok(true) => ulog!(LOG_DISPLAY, "[Platforming] {} input bound", self.name()),
            Ok(false) => {}
            Err(e) => ulog!(LOG_WARNING, "[Platforming] {} failed to bind its input: {e}", self.name()),
        }
    }

    /// Called for movement input
    #[ufunction(BlueprintCallable)]
    fn r#move(&mut self, value: UStructRef<FInputActionValue>) {
        let movement_vector = value.axis2d();
        // route the input
        self.do_move(movement_vector.x as f32, movement_vector.y as f32);
    }

    /// Called for looking input
    #[ufunction(BlueprintCallable)]
    fn look(&mut self, value: UStructRef<FInputActionValue>) {
        let look_axis_vector = value.axis2d();
        // route the input
        self.do_look(look_axis_vector.x as f32, look_axis_vector.y as f32);
    }

    /// Called for dash input
    #[ufunction(BlueprintCallable)]
    fn dash(&mut self) {
        // route the input
        self.do_dash();
    }

    /// Handles move inputs from either controls or UI interfaces
    #[ufunction(BlueprintCallable)]
    fn do_move(&mut self, right: f32, forward: f32) {
        if let Err(e) = self.apply_move(right, forward) {
            ulog!(LOG_WARNING, "[Platforming] DoMove failed: {e}");
        }
    }

    /// Handles look inputs from either controls or UI interfaces
    #[ufunction(BlueprintCallable)]
    fn do_look(&mut self, yaw: f32, pitch: f32) {
        let Ok(me) = self.as_ref().checked() else {
            return;
        };
        if me.get_controller().is_valid() {
            // add yaw and pitch input to controller
            me.add_controller_yaw_input(yaw);
            me.add_controller_pitch_input(pitch);
        }
    }

    /// Handles dash inputs from either controls or UI interfaces
    #[ufunction(BlueprintCallable)]
    fn do_dash(&mut self) {
        if let Err(e) = self.start_dash() {
            ulog!(LOG_WARNING, "[Platforming] DoDash failed: {e}");
        }
    }

    /// Handles jump pressed inputs from either controls or UI interfaces
    #[ufunction(BlueprintCallable)]
    fn do_jump_start(&mut self) {
        // handle special jump cases
        if let Err(e) = self.multi_jump() {
            ulog!(LOG_WARNING, "[Platforming] jump failed: {e}");
        }
    }

    /// Handles jump pressed inputs from either controls or UI interfaces
    #[ufunction(BlueprintCallable)]
    fn do_jump_end(&mut self) {
        // stop jumping
        if let Ok(me) = self.as_ref().checked() {
            me.stop_jumping();
        }
    }

    /// Resets the wall jump input lock (the wall jump timer's function)
    #[ufunction(BlueprintCallable)]
    fn reset_wall_jump(&mut self) {
        // reset the wall jump input lock
        let mut state = self.state();
        state.has_wall_jumped = false;
        self.set_state(state);
    }

    /// Passes control to Blueprint to enable or disable jump trails
    #[ufunction(BlueprintImplementableEvent)]
    fn set_jump_trail_state(&self, b_enabled: bool) {}

    /// Returns true if the character has just double jumped
    #[ufunction(BlueprintPure)]
    fn has_double_jumped(&self) -> bool {
        self.state().has_double_jumped
    }

    /// Returns true if the character has just wall jumped
    #[ufunction(BlueprintPure)]
    fn has_wall_jumped(&self) -> bool {
        self.state().has_wall_jumped
    }

    /// EndPlay cleanup
    #[ufunction(Override)]
    fn receive_end_play(&mut self, _end_play_reason: u8) {
        // clear the wall jump reset timer
        KismetSystemLibrary::k2_clear_timer(self.as_ref().upcast_to(), "ResetWallJump");
    }

    /// Handle landings to reset dash and advanced jump state
    #[ufunction(Override)]
    fn on_landed(&mut self, _hit: UStructRef<FHitResult>) {
        // reset the double jump and dash flags
        let mut state = self.state();
        state.has_double_jumped = false;
        state.has_dashed = false;
        self.set_state(state);

        // deactivate the jump trail
        self.set_jump_trail_state(false);
    }

    /// Handle movement mode changes to keep track of coyote time jumps
    #[ufunction(Override, name = "K2_OnMovementModeChanged")]
    fn on_movement_mode_changed(
        &mut self,
        _prev_movement_mode: u8,
        new_movement_mode: u8,
        _prev_custom_mode: u8,
        _new_custom_mode: u8,
    ) {
        // are we falling?
        if new_movement_mode == MOVE_FALLING {
            // save the game time when we started falling, so we can check it later for coyote time jumps
            let mut state = self.state();
            state.last_fall_time = GameplayStatics::get_time_seconds(self.as_ref().upcast_to());
            self.set_state(state);
        }
    }
}

impl PlatformingCharacter {
    /// Everything `APlatformingCharacter::SetupPlayerInputComponent` does. Only
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
        bind_action(&me, self.move_action(), ETriggerEvent::Triggered, "Move")?;
        bind_action(&me, self.mouse_look_action(), ETriggerEvent::Triggered, "Look")?;

        // Looking
        bind_action(&me, self.look_action(), ETriggerEvent::Triggered, "Look")?;

        // Dashing
        bind_action(&me, self.dash_action(), ETriggerEvent::Triggered, "Dash")?;

        Ok(true)
    }

    /// `DoMove`: move along the controller's yaw, unless just wall jumped.
    fn apply_move(&self, right: f32, forward: f32) -> RustealResult<()> {
        let me = self.as_ref().checked()?;
        let controller = me.get_controller();
        // momentarily disable movement inputs if we've just wall jumped
        if !controller.is_valid() || self.state().has_wall_jumped {
            return Ok(());
        }
        // find out which way is forward
        let yaw = controller.checked()?.get_control_rotation().to_rotator().yaw;
        let (forward_direction, right_direction) = ground_axes(yaw);
        // add movement
        me.add_movement_input(&FVector::from_dvec3(forward_direction), Some(forward), Some(false));
        me.add_movement_input(&FVector::from_dvec3(right_direction), Some(right), Some(false));
        Ok(())
    }

    /// `MultiJump`: a regular jump on the ground; in the air a wall jump, a
    /// coyote time jump or a double jump, in that order.
    fn multi_jump(&mut self) -> RustealResult<()> {
        let me = self.as_ref().checked()?;
        let mut state = self.state();
        let falling = me.get_character_movement().checked()?.is_falling();
        match model::jump_press(&state, falling) {
            JumpPress::Ignore => {}
            JumpPress::Ground => {
                // we're grounded so just do a regular jump
                me.jump();
                // activate the jump trail
                self.set_jump_trail_state(true);
            }
            JumpPress::TryWallJump => {
                if self.wall_jump(&mut state)? {
                    self.set_state(state);
                    return Ok(());
                }
                // no wall jump, try a double jump next
                let now = GameplayStatics::get_time_seconds(self.as_ref().upcast_to());
                match model::air_jump(&state, now, self.max_coyote_time()) {
                    AirJump::Coyote => {
                        ulog!(LOG_WARNING, "Coyote Jump");
                        // use the built-in CMC functionality to do the jump
                        me.jump();
                        // enable the jump trail
                        self.set_jump_trail_state(true);
                    }
                    AirJump::Double => {
                        state.has_double_jumped = true;
                        // use the built-in CMC functionality to do the double jump
                        me.jump();
                        // enable the jump trail
                        self.set_jump_trail_state(true);
                    }
                    AirJump::None => {}
                }
            }
        }
        self.set_state(state);
        Ok(())
    }

    /// Sweep for a wall in front of the character and jump off it; `false`
    /// when there is none.
    fn wall_jump(&self, state: &mut MoveState) -> RustealResult<bool> {
        let me = self.as_ref().checked()?;
        // run a sphere sweep to check if we're in front of a wall
        let trace_start = me.k2_get_actor_location().to_dvec3();
        let trace_end = trace_start
            + me.get_actor_forward_vector().to_dvec3() * f64::from(self.wall_jump_trace_distance());
        let (hit, out_hit) = KismetSystemLibrary::sphere_trace_single(
            self.as_ref().upcast_to(),
            &FVector::from_dvec3(trace_start),
            &FVector::from_dvec3(trace_end),
            self.wall_jump_trace_radius(),
            ETraceTypeQuery::TraceTypeQuery1, // ECC_Visibility
            false,
            &[],
            EDrawDebugTrace::None,
            true,
            &OwnedStruct::new(),
            &OwnedStruct::new(),
            None,
        );
        if !hit {
            return Ok(false);
        }
        let impact_normal = out_hit.as_ref().get_impact_normal().to_dvec3();

        // rotate the character to face away from the wall, so we're correctly oriented for the next wall jump
        let wall_orientation = Rotator::new(0.0, model::yaw_facing(impact_normal), 0.0);
        me.k2_set_actor_rotation(&FRotator::from_rotator(wall_orientation), false);

        // apply a launch impulse to the character to perform the actual wall jump
        let wall_jump_impulse = model::wall_jump_impulse(
            impact_normal,
            self.wall_jump_bounce_impulse(),
            self.wall_jump_vertical_impulse(),
        );
        me.launch_character(&FVector::from_dvec3(wall_jump_impulse), true, true);

        // enable the jump trail
        self.set_jump_trail_state(true);

        // raise the wall jump flag to prevent an immediate second wall jump
        state.has_wall_jumped = true;
        KismetSystemLibrary::k2_set_timer(
            self.as_ref().upcast_to(),
            "ResetWallJump",
            self.delay_between_wall_jumps(),
            false,
            None,
            None,
            None,
        );
        Ok(true)
    }

    /// `DoDash`: dash forward without gravity, for the length of the dash
    /// montage.
    fn start_dash(&mut self) -> RustealResult<()> {
        let mut state = self.state();
        // ignore the input if we've already dashed and have yet to reset
        if state.has_dashed {
            return Ok(());
        }

        // raise the dash flags
        state.is_dashing = true;
        state.has_dashed = true;
        self.set_state(state);

        let me = self.as_ref().checked()?;
        let movement = me.get_character_movement().checked()?;
        // disable gravity while dashing
        movement.set_gravity_scale(0.0);
        // reset the character velocity so we don't carry momentum into the dash
        movement.set_velocity(&FVector::from_dvec3(glam::DVec3::ZERO));

        // enable the jump trails
        self.set_jump_trail_state(true);

        // play the dash montage
        let anim_instance = me.get_mesh().checked()?.get_anim_instance();
        if let Ok(anim_instance) = anim_instance.checked() {
            self.bind_dash_end(&anim_instance.as_ref())?;
            anim_instance.montage_play(self.dash_montage(), Some(1.0), None, Some(0.0), Some(true));
        }
        Ok(())
    }

    /// Route the end of the dash montage to `dash_montage_ended`, once: the
    /// C++ class sets the montage's end delegate on every dash.
    fn bind_dash_end(&mut self, anim_instance: &UObjectRef<bindings::engine::AnimInstance>) -> RustealResult<()> {
        if self.dash_end_bound() {
            return Ok(());
        }
        let character: UObjectRef<Character> = self.as_ref();
        anim_instance
            .checked()?
            .on_montage_ended()
            .add(move |montage, interrupted| {
                if let Ok(mut me) = PlatformingCharacter::from_obj(character)
                    && montage == me.dash_montage()
                {
                    me.dash_montage_ended(interrupted);
                }
            })?
            .detach();
        self.set_dash_end_bound(true);
        Ok(())
    }

    /// Called when the dash montage ends
    fn dash_montage_ended(&mut self, interrupted: bool) {
        // Avoid resetting the dash if the previous ground dash interrupted the montage.
        if interrupted && self.state().is_dashing {
            return;
        }
        // end the dash
        self.end_dash();
    }

    /// Ends the dash state
    pub fn end_dash(&mut self) {
        let Ok(movement) = self.as_ref().checked().and_then(|me| me.get_character_movement().checked()) else {
            return;
        };
        // restore gravity
        movement.set_gravity_scale(model::GRAVITY_SCALE);

        // reset the dashing flag
        let mut state = self.state();
        state.is_dashing = false;

        // are we grounded after the dash?
        if movement.is_moving_on_ground() {
            // reset the dash usage flag, since we won't receive a landed event
            state.has_dashed = false;
            // deactivate the jump trails
            self.set_jump_trail_state(false);
        }
        self.set_state(state);
    }

    fn name(&self) -> String {
        self.as_ref().get_name().unwrap_or_else(|_| "PlatformingCharacter".into())
    }
}
