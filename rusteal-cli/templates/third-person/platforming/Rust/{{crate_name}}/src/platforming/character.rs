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

const MOVE_FALLING: u8 = 3;

#[uclass(parent = Character)]
pub struct PlatformingCharacter {
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

    #[uproperty(EditAnywhere)]
    dash_action: UObjectRef<InputAction>,

    #[uproperty(EditAnywhere, default = model::WALL_JUMP_TRACE_DISTANCE)]
    wall_jump_trace_distance: f32,

    #[uproperty(EditAnywhere, default = model::WALL_JUMP_TRACE_RADIUS)]
    wall_jump_trace_radius: f32,

    #[uproperty(EditAnywhere, default = model::WALL_JUMP_BOUNCE_IMPULSE)]
    wall_jump_bounce_impulse: f32,

    #[uproperty(EditAnywhere, default = model::WALL_JUMP_VERTICAL_IMPULSE)]
    wall_jump_vertical_impulse: f32,

    #[uproperty(EditAnywhere, default = model::DELAY_BETWEEN_WALL_JUMPS)]
    delay_between_wall_jumps: f32,

    #[uproperty(EditAnywhere)]
    dash_montage: UObjectRef<AnimMontage>,

    #[uproperty(EditAnywhere, default = model::MAX_COYOTE_TIME)]
    max_coyote_time: f32,

    state: MoveState,

    dash_end_bound: bool,
}

#[uclass_impl]
impl PlatformingCharacter {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        let me = self.as_ref().checked()?;

        me.set_jump_max_hold_time(model::JUMP_MAX_HOLD_TIME);

        me.set_jump_max_count(model::JUMP_MAX_COUNT);

        me.get_capsule_component().checked()?.set_capsule_size(
            model::CAPSULE_RADIUS,
            model::CAPSULE_HALF_HEIGHT,
            Some(false),
        );

        me.set_use_controller_rotation_yaw(false);

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

        let boom = self.camera_boom()?.checked()?;
        boom.set_target_arm_length(model::CAMERA_BOOM_LENGTH);
        boom.set_use_pawn_control_rotation(true);
        boom.set_enable_camera_lag(true);
        boom.set_camera_lag_speed(model::CAMERA_LAG_SPEED);
        boom.set_enable_camera_rotation_lag(true);
        boom.set_camera_rotation_lag_speed(model::CAMERA_ROTATION_LAG_SPEED);

        self.follow_camera()?
            .checked()?
            .set_use_pawn_control_rotation(false);

        Ok(())
    }

    #[ufunction(Override)]
    fn receive_restarted(&mut self) {
        match self.bind_input() {
            Ok(true) => ulog!(LOG_DISPLAY, "[Platforming] {} input bound", self.name()),
            Ok(false) => {}
            Err(e) => ulog!(
                LOG_WARNING,
                "[Platforming] {} failed to bind its input: {e}",
                self.name()
            ),
        }
    }

    #[ufunction(BlueprintCallable)]
    fn r#move(&mut self, value: UStructRef<FInputActionValue>) {
        let movement_vector = value.axis2d();

        self.do_move(movement_vector.x as f32, movement_vector.y as f32);
    }

    #[ufunction(BlueprintCallable)]
    fn look(&mut self, value: UStructRef<FInputActionValue>) {
        let look_axis_vector = value.axis2d();

        self.do_look(look_axis_vector.x as f32, look_axis_vector.y as f32);
    }

    #[ufunction(BlueprintCallable)]
    fn dash(&mut self) {
        self.do_dash();
    }

    #[ufunction(BlueprintCallable)]
    fn do_move(&mut self, right: f32, forward: f32) {
        if let Err(e) = self.apply_move(right, forward) {
            ulog!(LOG_WARNING, "[Platforming] DoMove failed: {e}");
        }
    }

    #[ufunction(BlueprintCallable)]
    fn do_look(&mut self, yaw: f32, pitch: f32) {
        let Ok(me) = self.as_ref().checked() else {
            return;
        };

        if me.get_controller().is_valid() {
            me.add_controller_yaw_input(yaw);
            me.add_controller_pitch_input(pitch);
        }
    }

    #[ufunction(BlueprintCallable)]
    fn do_dash(&mut self) {
        if let Err(e) = self.start_dash() {
            ulog!(LOG_WARNING, "[Platforming] DoDash failed: {e}");
        }
    }

    #[ufunction(BlueprintCallable)]
    fn do_jump_start(&mut self) {
        if let Err(e) = self.multi_jump() {
            ulog!(LOG_WARNING, "[Platforming] jump failed: {e}");
        }
    }

    #[ufunction(BlueprintCallable)]
    fn do_jump_end(&mut self) {
        if let Ok(me) = self.as_ref().checked() {
            me.stop_jumping();
        }
    }

    #[ufunction(BlueprintCallable)]
    fn reset_wall_jump(&mut self) {
        let mut state = self.state();
        state.has_wall_jumped = false;
        self.set_state(state);
    }

    #[ufunction(BlueprintImplementableEvent)]
    fn set_jump_trail_state(&self, b_enabled: bool) {}

    #[ufunction(BlueprintPure)]
    fn has_double_jumped(&self) -> bool {
        self.state().has_double_jumped
    }

    #[ufunction(BlueprintPure)]
    fn has_wall_jumped(&self) -> bool {
        self.state().has_wall_jumped
    }

    #[ufunction(Override)]
    fn receive_end_play(&mut self, _end_play_reason: u8) {
        KismetSystemLibrary::k2_clear_timer(self.as_ref().upcast_to(), "ResetWallJump");
    }

    #[ufunction(Override)]
    fn on_landed(&mut self, _hit: UStructRef<FHitResult>) {
        let mut state = self.state();
        state.has_double_jumped = false;
        state.has_dashed = false;
        self.set_state(state);

        self.set_jump_trail_state(false);
    }

    #[ufunction(Override, name = "K2_OnMovementModeChanged")]
    fn on_movement_mode_changed(
        &mut self,
        _prev_movement_mode: u8,
        new_movement_mode: u8,
        _prev_custom_mode: u8,
        _new_custom_mode: u8,
    ) {
        if new_movement_mode == MOVE_FALLING {
            let mut state = self.state();
            state.last_fall_time = GameplayStatics::get_time_seconds(self.as_ref().upcast_to());
            self.set_state(state);
        }
    }
}

impl PlatformingCharacter {
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

        bind_action(&me, self.move_action(), ETriggerEvent::Triggered, "Move")?;

        bind_action(
            &me,
            self.mouse_look_action(),
            ETriggerEvent::Triggered,
            "Look",
        )?;

        bind_action(&me, self.look_action(), ETriggerEvent::Triggered, "Look")?;

        bind_action(&me, self.dash_action(), ETriggerEvent::Triggered, "Dash")?;

        Ok(true)
    }

    fn apply_move(&self, right: f32, forward: f32) -> RustealResult<()> {
        let me = self.as_ref().checked()?;
        let controller = me.get_controller();

        if !controller.is_valid() || self.state().has_wall_jumped {
            return Ok(());
        }

        let yaw = controller
            .checked()?
            .get_control_rotation()
            .to_rotator()
            .yaw;

        let (forward_direction, right_direction) = ground_axes(yaw);

        me.add_movement_input(
            &FVector::from_dvec3(forward_direction),
            Some(forward),
            Some(false),
        );

        me.add_movement_input(
            &FVector::from_dvec3(right_direction),
            Some(right),
            Some(false),
        );

        Ok(())
    }

    fn multi_jump(&mut self) -> RustealResult<()> {
        let me = self.as_ref().checked()?;
        let mut state = self.state();
        let falling = me.get_character_movement().checked()?.is_falling();

        match model::jump_press(&state, falling) {
            JumpPress::Ignore => {}
            JumpPress::Ground => {
                me.jump();

                self.set_jump_trail_state(true);
            }
            JumpPress::TryWallJump => {
                if self.wall_jump(&mut state)? {
                    self.set_state(state);
                    return Ok(());
                }

                let now = GameplayStatics::get_time_seconds(self.as_ref().upcast_to());

                match model::air_jump(&state, now, self.max_coyote_time()) {
                    AirJump::Coyote => {
                        ulog!(LOG_WARNING, "Coyote Jump");

                        me.jump();

                        self.set_jump_trail_state(true);
                    }
                    AirJump::Double => {
                        state.has_double_jumped = true;

                        me.jump();

                        self.set_jump_trail_state(true);
                    }
                    AirJump::None => {}
                }
            }
        }

        self.set_state(state);

        Ok(())
    }

    fn wall_jump(&self, state: &mut MoveState) -> RustealResult<bool> {
        let me = self.as_ref().checked()?;

        let trace_start = me.k2_get_actor_location().to_dvec3();

        let trace_end = trace_start
            + me.get_actor_forward_vector().to_dvec3() * f64::from(self.wall_jump_trace_distance());

        let (hit, out_hit) = KismetSystemLibrary::sphere_trace_single(
            self.as_ref().upcast_to(),
            &FVector::from_dvec3(trace_start),
            &FVector::from_dvec3(trace_end),
            self.wall_jump_trace_radius(),
            ETraceTypeQuery::TraceTypeQuery1,
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

        let wall_orientation = Rotator::new(0.0, model::yaw_facing(impact_normal), 0.0);
        me.k2_set_actor_rotation(&FRotator::from_rotator(wall_orientation), false);

        let wall_jump_impulse = model::wall_jump_impulse(
            impact_normal,
            self.wall_jump_bounce_impulse(),
            self.wall_jump_vertical_impulse(),
        );

        me.launch_character(&FVector::from_dvec3(wall_jump_impulse), true, true);

        self.set_jump_trail_state(true);

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

    fn start_dash(&mut self) -> RustealResult<()> {
        let mut state = self.state();

        if state.has_dashed {
            return Ok(());
        }

        state.is_dashing = true;
        state.has_dashed = true;
        self.set_state(state);

        let me = self.as_ref().checked()?;
        let movement = me.get_character_movement().checked()?;

        movement.set_gravity_scale(0.0);

        movement.set_velocity(&FVector::from_dvec3(glam::DVec3::ZERO));

        self.set_jump_trail_state(true);

        let anim_instance = me.get_mesh().checked()?.get_anim_instance();

        if let Ok(anim_instance) = anim_instance.checked() {
            self.bind_dash_end(&anim_instance.as_ref())?;
            anim_instance.montage_play(self.dash_montage(), Some(1.0), None, Some(0.0), Some(true));
        }

        Ok(())
    }

    fn bind_dash_end(
        &mut self,
        anim_instance: &UObjectRef<bindings::engine::AnimInstance>,
    ) -> RustealResult<()> {
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

    fn dash_montage_ended(&mut self, interrupted: bool) {
        if interrupted && self.state().is_dashing {
            return;
        }

        self.end_dash();
    }

    pub fn end_dash(&mut self) {
        let Ok(movement) = self
            .as_ref()
            .checked()
            .and_then(|me| me.get_character_movement().checked())
        else {
            return;
        };

        movement.set_gravity_scale(model::GRAVITY_SCALE);

        let mut state = self.state();
        state.is_dashing = false;

        if movement.is_moving_on_ground() {
            state.has_dashed = false;

            self.set_jump_trail_state(false);
        }

        self.set_state(state);
    }

    fn name(&self) -> String {
        self.as_ref()
            .get_name()
            .unwrap_or_else(|_| "PlatformingCharacter".into())
    }
}
