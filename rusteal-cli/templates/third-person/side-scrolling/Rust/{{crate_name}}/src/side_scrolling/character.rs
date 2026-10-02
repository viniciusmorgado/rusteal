// SideScrollingCharacter: the SideScrolling variant's `ASideScrollingCharacter`
// in Rust. A player character for a side scrolling game: moves along X, double
// jumps, wall jumps, drops through soft platforms, pushes physics objects
// while in the air and interacts with what is in front of it.
//
// The C++ virtuals it overrides are their Blueprint events here: `NotifyHit`
// is `ReceiveHit`, `Landed` is `OnLanded`, `OnMovementModeChanged` is
// `K2_OnMovementModeChanged`, `EndPlay` is `ReceiveEndPlay` and
// `SetupPlayerInputComponent` is `ReceiveRestarted`. The Blueprint child
// `BP_SideScrollingCharacter` holds the mannequin, the input actions and the
// soft collision object type.

use bindings::engine::{
    Actor, ActorComponentExt, ActorExt, CameraComponent, CapsuleComponentExt, Character, CharacterExt,
    CharacterMovementComponentExt, EComponentMobility, ECollisionChannel, ECollisionResponse,
    EDrawDebugTrace, ETraceTypeQuery, FHitResult, FHitResultExt, GameplayStatics,
    KismetSystemLibrary, MovementComponentExt, NavMovementComponentExt, PawnExt,
    PrimitiveComponent, PrimitiveComponentExt, SceneComponentExt,
};
use bindings::enhanced_input::{ETriggerEvent, FInputActionValue, InputAction};
use bindings::prelude::*;
use rusteal_runtime::runtime::{
    LOG_DISPLAY, LOG_WARNING, OwnedStruct, Rotator, RustealResult, UObjectRef, UStructRef, ulog,
};
use rusteal_runtime::{uclass, uclass_impl};

use super::interactable;
use super::model::{self, AirJump, JumpPress, MoveState};

/// `EMovementMode::MOVE_Falling`.
const MOVE_FALLING: u8 = 3;

#[uclass(parent = Character)]
pub struct SideScrollingCharacter {
    /// Player camera
    #[component(attach = "root_component")]
    camera: CameraComponent,

    /// Move Input Action
    #[uproperty(EditAnywhere)]
    move_action: UObjectRef<InputAction>,

    /// Jump Input Action
    #[uproperty(EditAnywhere)]
    jump_action: UObjectRef<InputAction>,

    /// Drop from Platform Action
    #[uproperty(EditAnywhere)]
    drop_action: UObjectRef<InputAction>,

    /// Interact Input Action
    #[uproperty(EditAnywhere)]
    interact_action: UObjectRef<InputAction>,

    /// Impulse to manually push physics objects while we're in midair
    #[uproperty(EditAnywhere, default = model::JUMP_PUSH_IMPULSE)]
    jump_push_impulse: f32,

    /// Max distance that interactive objects can be triggered
    #[uproperty(EditAnywhere, default = model::INTERACTION_RADIUS)]
    interaction_radius: f32,

    /// Time to disable input after a wall jump to preserve momentum
    #[uproperty(EditAnywhere, default = model::DELAY_BETWEEN_WALL_JUMPS)]
    delay_between_wall_jumps: f32,

    /// Distance to trace ahead of the character for wall jumps
    #[uproperty(EditAnywhere, default = model::WALL_JUMP_TRACE_DISTANCE)]
    wall_jump_trace_distance: f32,

    /// Horizontal impulse to apply to the character during wall jumps
    #[uproperty(EditAnywhere, default = model::WALL_JUMP_HORIZONTAL_IMPULSE)]
    wall_jump_horizontal_impulse: f32,

    /// Multiplies the jump Z velocity for wall jumps.
    #[uproperty(EditAnywhere, default = model::WALL_JUMP_VERTICAL_MULTIPLIER)]
    wall_jump_vertical_multiplier: f32,

    /// Collision object type to use for soft collision traces (dropping down floors)
    #[uproperty(EditAnywhere)]
    soft_collision_object_type: ECollisionChannel,

    /// Distance to trace down during soft collision checks
    #[uproperty(EditAnywhere, default = model::SOFT_COLLISION_TRACE_DISTANCE)]
    soft_collision_trace_distance: f32,

    /// Max amount of time that can pass since we started falling when we allow a regular jump
    #[uproperty(EditAnywhere, default = model::MAX_COYOTE_TIME)]
    max_coyote_time: f32,

    /// movement flags and the last captured inputs
    state: MoveState,
}

#[uclass_impl]
impl SideScrollingCharacter {
    /// Everything `ASideScrollingCharacter::ASideScrollingCharacter()` sets.
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        let me = self.as_ref().checked()?;

        // create the camera component
        self.camera()?.checked()?.k2_set_relative_location_and_rotation(
            &FVector::from_dvec3(model::CAMERA_LOCATION),
            &FRotator::from_rotator(Rotator::new(0.0, model::CAMERA_YAW, 0.0)),
            false,
            false,
        );

        // configure the collision capsule
        me.get_capsule_component().checked()?.set_capsule_size(
            model::CAPSULE_RADIUS,
            model::CAPSULE_HALF_HEIGHT,
            Some(true),
        );

        // configure the Pawn properties
        me.set_use_controller_rotation_yaw(false);

        // configure the character movement component
        let movement = me.get_character_movement().checked()?;
        movement.set_gravity_scale(model::GRAVITY_SCALE);
        movement.set_max_acceleration(model::MAX_ACCELERATION);
        movement.set_braking_friction_factor(model::BRAKING_FRICTION_FACTOR);
        movement.set_use_separate_braking_friction(true);
        movement.set_mass(model::MASS);
        movement.set_walkable_floor_angle(model::WALKABLE_FLOOR_ANGLE);
        movement.set_max_walk_speed(model::MAX_WALK_SPEED);
        movement.set_min_analog_walk_speed(model::MIN_ANALOG_WALK_SPEED);
        movement.set_braking_deceleration_walking(model::BRAKING_DECELERATION_WALKING);
        movement.set_ignore_base_rotation(true);
        movement.set_perch_radius_threshold(model::PERCH_RADIUS_THRESHOLD);
        movement.set_ledge_check_threshold(model::LEDGE_CHECK_THRESHOLD);
        movement.set_jump_z_velocity(model::JUMP_Z_VELOCITY);
        movement.set_air_control(model::AIR_CONTROL);
        movement.set_rotation_rate(&FRotator::from_rotator(Rotator::new(
            0.0,
            model::ROTATION_RATE_YAW,
            0.0,
        )));
        movement.set_orient_rotation_to_movement(true);
        movement.set_plane_constraint_normal(&FVector::from_dvec3(model::PLANE_CONSTRAINT_NORMAL));
        movement.set_constrain_to_plane(true);

        // enable double jump and coyote time
        me.set_jump_max_count(model::JUMP_MAX_COUNT);
        Ok(())
    }

    /// The pawn's player input component is set up: bind the input actions.
    #[ufunction(Override)]
    fn receive_restarted(&mut self) {
        match self.bind_input() {
            Ok(true) => ulog!(LOG_DISPLAY, "[SideScrolling] {} input bound", self.name()),
            Ok(false) => {}
            Err(e) => ulog!(LOG_WARNING, "[SideScrolling] {} failed to bind its input: {e}", self.name()),
        }
    }

    /// Gameplay cleanup
    #[ufunction(Override)]
    fn receive_end_play(&mut self, _end_play_reason: u8) {
        // clear the wall jump timer
        KismetSystemLibrary::k2_clear_timer(self.as_ref().upcast_to(), "ResetWallJump");
    }

    /// Collision handling
    #[ufunction(Override)]
    #[allow(clippy::too_many_arguments)]
    fn receive_hit(
        &mut self,
        _my_comp: UObjectRef<PrimitiveComponent>,
        _other: UObjectRef<Actor>,
        other_comp: UObjectRef<PrimitiveComponent>,
        _b_self_moved: bool,
        _hit_location: UStructRef<FVector>,
        _hit_normal: UStructRef<FVector>,
        _normal_impulse: UStructRef<FVector>,
        _hit: UStructRef<FHitResult>,
    ) {
        if let Err(e) = self.push(other_comp) {
            ulog!(LOG_WARNING, "[SideScrolling] push failed: {e}");
        }
    }

    /// Landing handling
    #[ufunction(Override)]
    fn on_landed(&mut self, _hit: UStructRef<FHitResult>) {
        // reset the double jump
        let mut state = self.state();
        state.has_double_jumped = false;
        self.set_state(state);
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

    /// Called for movement input
    #[ufunction(BlueprintCallable)]
    fn r#move(&mut self, value: UStructRef<FInputActionValue>) {
        let move_vector = value.axis2d();
        // route the input
        self.do_move(move_vector.y as f32);
    }

    /// Called for drop from platform input
    #[ufunction(BlueprintCallable)]
    fn drop(&mut self, value: UStructRef<FInputActionValue>) {
        // route the input
        self.do_drop(value.axis1d() as f32);
    }

    /// Called for drop from platform input release
    #[ufunction(BlueprintCallable)]
    fn drop_released(&mut self, _value: UStructRef<FInputActionValue>) {
        // reset the input
        self.do_drop(0.0);
    }

    /// Handles move inputs from either controls or UI interfaces
    #[ufunction(BlueprintCallable)]
    fn do_move(&mut self, forward: f32) {
        let mut state = self.state();
        // is movement temporarily disabled after wall jumping?
        if state.has_wall_jumped {
            return;
        }
        // save the movement values
        state.action_value_y = forward;
        self.set_state(state);
        // apply the movement input
        if let Ok(me) = self.as_ref().checked() {
            me.add_movement_input(&FVector::from_dvec3(model::move_direction(forward)), Some(forward), Some(false));
        }
    }

    /// Handles drop inputs from either controls or UI interfaces
    #[ufunction(BlueprintCallable)]
    fn do_drop(&mut self, value: f32) {
        // save the movement value
        let mut state = self.state();
        state.drop_value = value;
        self.set_state(state);
    }

    /// Handles jump pressed inputs from either controls or UI interfaces
    #[ufunction(BlueprintCallable)]
    fn do_jump_start(&mut self) {
        // handle advanced jump behaviors
        if let Err(e) = self.multi_jump() {
            ulog!(LOG_WARNING, "[SideScrolling] jump failed: {e}");
        }
    }

    /// Handles jump pressed inputs from either controls or UI interfaces
    #[ufunction(BlueprintCallable)]
    fn do_jump_end(&mut self) {
        if let Ok(me) = self.as_ref().checked() {
            me.stop_jumping();
        }
    }

    /// Handles interact inputs from either controls or UI interfaces
    #[ufunction(BlueprintCallable)]
    fn do_interact(&mut self) {
        if let Err(e) = self.interact() {
            ulog!(LOG_WARNING, "[SideScrolling] interact failed: {e}");
        }
    }

    /// Resets wall jump lockout. Called from timer after a wall jump
    #[ufunction(BlueprintCallable)]
    fn reset_wall_jump(&mut self) {
        // reset the wall jump flag
        let mut state = self.state();
        state.has_wall_jumped = false;
        self.set_state(state);
    }

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
}

impl SideScrollingCharacter {
    /// Everything `ASideScrollingCharacter::SetupPlayerInputComponent` does;
    /// `false` for a pawn that is not a local player's.
    fn bind_input(&self) -> RustealResult<bool> {
        let me = self.as_ref();
        let pawn = me.checked()?;
        if !pawn.is_player_controlled() || !pawn.is_locally_controlled() {
            return Ok(false);
        }

        // Jumping
        bind_action(&me, self.jump_action(), ETriggerEvent::Started, "DoJumpStart")?;
        bind_action(&me, self.jump_action(), ETriggerEvent::Completed, "DoJumpEnd")?;

        // Interacting
        bind_action(&me, self.interact_action(), ETriggerEvent::Triggered, "DoInteract")?;

        // Moving
        bind_action(&me, self.move_action(), ETriggerEvent::Triggered, "Move")?;

        // Dropping from platform
        bind_action(&me, self.drop_action(), ETriggerEvent::Triggered, "Drop")?;
        bind_action(&me, self.drop_action(), ETriggerEvent::Completed, "DropReleased")?;

        Ok(true)
    }

    /// `NotifyHit`: while falling, push a movable physics object away.
    fn push(&self, other_comp: UObjectRef<PrimitiveComponent>) -> RustealResult<()> {
        let me = self.as_ref().checked()?;
        // only apply push impulse if we're falling
        if !me.get_character_movement().checked()?.is_falling() {
            return Ok(());
        }
        // ensure the colliding component is valid
        let Ok(other) = other_comp.checked() else {
            return Ok(());
        };
        // ensure the component is movable and simulating physics
        if other.get_mobility() == EComponentMobility::Movable && other.is_simulating_physics(None) {
            let push_dir = glam::DVec3::new(model::facing_x(self.state().action_value_y), 0.0, 0.0);
            // push the component away
            other.add_impulse(
                &FVector::from_dvec3(push_dir * f64::from(self.jump_push_impulse())),
                None,
                Some(true),
            );
        }
        Ok(())
    }

    /// `DoInteract`: sweep ahead for something to interact with.
    fn interact(&self) -> RustealResult<()> {
        let me = self.as_ref().checked()?;
        // do a sphere trace to look for interactive objects
        let start = me.k2_get_actor_location().to_dvec3();
        let end = start + glam::DVec3::new(model::INTERACTION_REACH, 0.0, 0.0);
        let object_types: Vec<_> = [ECollisionChannel::ECC_Pawn, ECollisionChannel::ECC_WorldDynamic]
            .into_iter()
            .filter_map(object_type_query)
            .collect();
        let (hit, out_hit) = KismetSystemLibrary::sphere_trace_single_for_objects(
            self.as_ref().upcast_to(),
            &FVector::from_dvec3(start),
            &FVector::from_dvec3(end),
            self.interaction_radius(),
            &object_types,
            false,
            &[],
            EDrawDebugTrace::None,
            true,
            &OwnedStruct::new(),
            &OwnedStruct::new(),
            None,
        );
        if !hit {
            return Ok(());
        }
        // have we hit an interactable?
        let target = out_hit.as_ref().get_component().checked()?.get_owner();
        interactable::interact(target, self.as_ref().upcast_to());
        Ok(())
    }

    /// `MultiJump`: drop, regular jump, wall jump, coyote time jump or double
    /// jump, in that order.
    fn multi_jump(&mut self) -> RustealResult<()> {
        let me = self.as_ref().checked()?;
        let mut state = self.state();
        let falling = me.get_character_movement().checked()?.is_falling();
        match model::jump_press(&state, falling) {
            JumpPress::Drop => {
                self.check_for_soft_collision()?;
                return Ok(());
            }
            JumpPress::Ground => {
                // reset the drop value
                state.drop_value = 0.0;
                self.set_state(state);
                me.jump();
                return Ok(());
            }
            JumpPress::TryWallJump => {
                state.drop_value = 0.0;
                if self.wall_jump(&mut state)? {
                    self.set_state(state);
                    return Ok(());
                }
            }
            JumpPress::Air => state.drop_value = 0.0,
        }
        let now = GameplayStatics::get_time_seconds(self.as_ref().upcast_to());
        match model::air_jump(&state, now, self.max_coyote_time()) {
            AirJump::Coyote => {
                ulog!(LOG_WARNING, "Coyote Jump");
                // use the built-in CMC functionality to do the jump
                me.jump();
            }
            AirJump::Double => {
                // raise the double jump flag
                state.has_double_jumped = true;
                // let the CMC handle jump
                me.jump();
            }
            AirJump::None => {}
        }
        self.set_state(state);
        Ok(())
    }

    /// Trace ahead for a wall and jump off it; `false` when there is none.
    fn wall_jump(&self, state: &mut MoveState) -> RustealResult<bool> {
        let me = self.as_ref().checked()?;
        // trace ahead of the character for walls
        let start = me.k2_get_actor_location().to_dvec3();
        let end = start
            + glam::DVec3::new(model::facing_x(state.action_value_y), 0.0, 0.0)
                * f64::from(self.wall_jump_trace_distance());
        let (_, out_hit) = KismetSystemLibrary::line_trace_single(
            self.as_ref().upcast_to(),
            &FVector::from_dvec3(start),
            &FVector::from_dvec3(end),
            ETraceTypeQuery::TraceTypeQuery1, // ECC_Visibility
            false,
            &[],
            EDrawDebugTrace::None,
            true,
            &OwnedStruct::new(),
            &OwnedStruct::new(),
            None,
        );
        if !out_hit.as_ref().get_blocking_hit() {
            return Ok(false);
        }
        let impact_normal = out_hit.as_ref().get_impact_normal().to_dvec3();

        // rotate to the bounce direction
        let bounce = Rotator::new(0.0, model::yaw_of(impact_normal), 0.0);
        me.k2_set_actor_rotation(&FRotator::from_rotator(bounce), false);

        // calculate the impulse vector
        let movement = me.get_character_movement().checked()?;
        let impulse = model::wall_jump_impulse(
            impact_normal,
            self.wall_jump_horizontal_impulse(),
            movement.get_jump_z_velocity(),
            self.wall_jump_vertical_multiplier(),
        );
        // launch the character away from the wall
        me.launch_character(&FVector::from_dvec3(impulse), true, true);

        // enable wall jump lockout for a bit
        state.has_wall_jumped = true;

        // schedule wall jump lockout reset
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

    /// Checks for soft collision with platforms
    fn check_for_soft_collision(&mut self) -> RustealResult<()> {
        // reset the drop value
        let mut state = self.state();
        state.drop_value = 0.0;
        self.set_state(state);

        // trace down
        let me = self.as_ref().checked()?;
        let start = me.k2_get_actor_location().to_dvec3();
        let end = start - glam::DVec3::Z * f64::from(self.soft_collision_trace_distance());
        let object_types: Vec<_> = object_type_query(self.soft_collision_object_type()).into_iter().collect();
        let (hit, _) = KismetSystemLibrary::line_trace_single_for_objects(
            self.as_ref().upcast_to(),
            &FVector::from_dvec3(start),
            &FVector::from_dvec3(end),
            &object_types,
            false,
            &[],
            EDrawDebugTrace::None,
            true,
            &OwnedStruct::new(),
            &OwnedStruct::new(),
            None,
        );

        // did we hit a soft floor?
        if hit {
            // drop through the floor
            self.set_soft_collision(true)?;
        }
        Ok(())
    }

    /// Sets the soft collision response. True passes, False blocks
    pub fn set_soft_collision(&self, b_enabled: bool) -> RustealResult<()> {
        // enable or disable collision response to the soft collision channel
        let response = if b_enabled { ECollisionResponse::ECR_Ignore } else { ECollisionResponse::ECR_Block };
        self.as_ref()
            .checked()?
            .get_capsule_component()
            .checked()?
            .set_collision_response_to_channel(self.soft_collision_object_type(), response);
        Ok(())
    }

    fn name(&self) -> String {
        self.as_ref().get_name().unwrap_or_else(|_| "SideScrollingCharacter".into())
    }
}
