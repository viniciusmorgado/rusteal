use bindings::engine::{
    Actor, ActorComponentExt, ActorExt, CameraComponent, CapsuleComponentExt, Character,
    CharacterExt, CharacterMovementComponentExt, ECollisionChannel, ECollisionResponse,
    EComponentMobility, EDrawDebugTrace, ETraceTypeQuery, FHitResult, FHitResultExt,
    GameplayStatics, KismetSystemLibrary, MovementComponentExt, NavMovementComponentExt, PawnExt,
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

const MOVE_FALLING: u8 = 3;

#[uclass(parent = Character)]
pub struct SideScrollingCharacter {
    #[component(attach = "root_component")]
    camera: CameraComponent,

    #[uproperty(EditAnywhere)]
    move_action: UObjectRef<InputAction>,

    #[uproperty(EditAnywhere)]
    jump_action: UObjectRef<InputAction>,

    #[uproperty(EditAnywhere)]
    drop_action: UObjectRef<InputAction>,

    #[uproperty(EditAnywhere)]
    interact_action: UObjectRef<InputAction>,

    #[uproperty(EditAnywhere, default = model::JUMP_PUSH_IMPULSE)]
    jump_push_impulse: f32,

    #[uproperty(EditAnywhere, default = model::INTERACTION_RADIUS)]
    interaction_radius: f32,

    #[uproperty(EditAnywhere, default = model::DELAY_BETWEEN_WALL_JUMPS)]
    delay_between_wall_jumps: f32,

    #[uproperty(EditAnywhere, default = model::WALL_JUMP_TRACE_DISTANCE)]
    wall_jump_trace_distance: f32,

    #[uproperty(EditAnywhere, default = model::WALL_JUMP_HORIZONTAL_IMPULSE)]
    wall_jump_horizontal_impulse: f32,

    #[uproperty(EditAnywhere, default = model::WALL_JUMP_VERTICAL_MULTIPLIER)]
    wall_jump_vertical_multiplier: f32,

    #[uproperty(EditAnywhere)]
    soft_collision_object_type: ECollisionChannel,

    #[uproperty(EditAnywhere, default = model::SOFT_COLLISION_TRACE_DISTANCE)]
    soft_collision_trace_distance: f32,

    #[uproperty(EditAnywhere, default = model::MAX_COYOTE_TIME)]
    max_coyote_time: f32,

    state: MoveState,
}

#[uclass_impl]
impl SideScrollingCharacter {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        let me = self.as_ref().checked()?;

        self.camera()?
            .checked()?
            .k2_set_relative_location_and_rotation(
                &FVector::from_dvec3(model::CAMERA_LOCATION),
                &FRotator::from_rotator(Rotator::new(0.0, model::CAMERA_YAW, 0.0)),
                false,
                false,
            );

        me.get_capsule_component().checked()?.set_capsule_size(
            model::CAPSULE_RADIUS,
            model::CAPSULE_HALF_HEIGHT,
            Some(true),
        );

        me.set_use_controller_rotation_yaw(false);

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

        me.set_jump_max_count(model::JUMP_MAX_COUNT);

        Ok(())
    }

    #[ufunction(Override)]
    fn receive_restarted(&mut self) {
        match self.bind_input() {
            Ok(true) => ulog!(LOG_DISPLAY, "[SideScrolling] {} input bound", self.name()),
            Ok(false) => {}
            Err(e) => ulog!(
                LOG_WARNING,
                "[SideScrolling] {} failed to bind its input: {e}",
                self.name()
            ),
        }
    }

    #[ufunction(Override)]
    fn receive_end_play(&mut self, _end_play_reason: u8) {
        KismetSystemLibrary::k2_clear_timer(self.as_ref().upcast_to(), "ResetWallJump");
    }

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

    #[ufunction(Override)]
    fn on_landed(&mut self, _hit: UStructRef<FHitResult>) {
        let mut state = self.state();
        state.has_double_jumped = false;
        self.set_state(state);
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

    #[ufunction(BlueprintCallable)]
    fn r#move(&mut self, value: UStructRef<FInputActionValue>) {
        let move_vector = value.axis2d();

        self.do_move(move_vector.y as f32);
    }

    #[ufunction(BlueprintCallable)]
    fn drop(&mut self, value: UStructRef<FInputActionValue>) {
        self.do_drop(value.axis1d() as f32);
    }

    #[ufunction(BlueprintCallable)]
    fn drop_released(&mut self, _value: UStructRef<FInputActionValue>) {
        self.do_drop(0.0);
    }

    #[ufunction(BlueprintCallable)]
    fn do_move(&mut self, forward: f32) {
        let mut state = self.state();

        if state.has_wall_jumped {
            return;
        }

        state.action_value_y = forward;
        self.set_state(state);

        if let Ok(me) = self.as_ref().checked() {
            me.add_movement_input(
                &FVector::from_dvec3(model::move_direction(forward)),
                Some(forward),
                Some(false),
            );
        }
    }

    #[ufunction(BlueprintCallable)]
    fn do_drop(&mut self, value: f32) {
        let mut state = self.state();
        state.drop_value = value;
        self.set_state(state);
    }

    #[ufunction(BlueprintCallable)]
    fn do_jump_start(&mut self) {
        if let Err(e) = self.multi_jump() {
            ulog!(LOG_WARNING, "[SideScrolling] jump failed: {e}");
        }
    }

    #[ufunction(BlueprintCallable)]
    fn do_jump_end(&mut self) {
        if let Ok(me) = self.as_ref().checked() {
            me.stop_jumping();
        }
    }

    #[ufunction(BlueprintCallable)]
    fn do_interact(&mut self) {
        if let Err(e) = self.interact() {
            ulog!(LOG_WARNING, "[SideScrolling] interact failed: {e}");
        }
    }

    #[ufunction(BlueprintCallable)]
    fn reset_wall_jump(&mut self) {
        let mut state = self.state();
        state.has_wall_jumped = false;
        self.set_state(state);
    }

    #[ufunction(BlueprintPure)]
    fn has_double_jumped(&self) -> bool {
        self.state().has_double_jumped
    }

    #[ufunction(BlueprintPure)]
    fn has_wall_jumped(&self) -> bool {
        self.state().has_wall_jumped
    }
}

impl SideScrollingCharacter {
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
            self.interact_action(),
            ETriggerEvent::Triggered,
            "DoInteract",
        )?;

        bind_action(&me, self.move_action(), ETriggerEvent::Triggered, "Move")?;

        bind_action(&me, self.drop_action(), ETriggerEvent::Triggered, "Drop")?;

        bind_action(
            &me,
            self.drop_action(),
            ETriggerEvent::Completed,
            "DropReleased",
        )?;

        Ok(true)
    }

    fn push(&self, other_comp: UObjectRef<PrimitiveComponent>) -> RustealResult<()> {
        let me = self.as_ref().checked()?;

        if !me.get_character_movement().checked()?.is_falling() {
            return Ok(());
        }

        let Ok(other) = other_comp.checked() else {
            return Ok(());
        };

        if other.get_mobility() == EComponentMobility::Movable && other.is_simulating_physics(None)
        {
            let push_dir = glam::DVec3::new(model::facing_x(self.state().action_value_y), 0.0, 0.0);

            other.add_impulse(
                &FVector::from_dvec3(push_dir * f64::from(self.jump_push_impulse())),
                None,
                Some(true),
            );
        }

        Ok(())
    }

    fn interact(&self) -> RustealResult<()> {
        let me = self.as_ref().checked()?;

        let start = me.k2_get_actor_location().to_dvec3();
        let end = start + glam::DVec3::new(model::INTERACTION_REACH, 0.0, 0.0);

        let object_types: Vec<_> = [
            ECollisionChannel::ECC_Pawn,
            ECollisionChannel::ECC_WorldDynamic,
        ]
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

        let target = out_hit.as_ref().get_component().checked()?.get_owner();
        interactable::interact(target, self.as_ref().upcast_to());

        Ok(())
    }

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

                me.jump();
            }
            AirJump::Double => {
                state.has_double_jumped = true;

                me.jump();
            }
            AirJump::None => {}
        }

        self.set_state(state);

        Ok(())
    }

    fn wall_jump(&self, state: &mut MoveState) -> RustealResult<bool> {
        let me = self.as_ref().checked()?;

        let start = me.k2_get_actor_location().to_dvec3();

        let end = start
            + glam::DVec3::new(model::facing_x(state.action_value_y), 0.0, 0.0)
                * f64::from(self.wall_jump_trace_distance());

        let (_, out_hit) = KismetSystemLibrary::line_trace_single(
            self.as_ref().upcast_to(),
            &FVector::from_dvec3(start),
            &FVector::from_dvec3(end),
            ETraceTypeQuery::TraceTypeQuery1,
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

        let bounce = Rotator::new(0.0, model::yaw_of(impact_normal), 0.0);
        me.k2_set_actor_rotation(&FRotator::from_rotator(bounce), false);

        let movement = me.get_character_movement().checked()?;

        let impulse = model::wall_jump_impulse(
            impact_normal,
            self.wall_jump_horizontal_impulse(),
            movement.get_jump_z_velocity(),
            self.wall_jump_vertical_multiplier(),
        );

        me.launch_character(&FVector::from_dvec3(impulse), true, true);

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

    fn check_for_soft_collision(&mut self) -> RustealResult<()> {
        let mut state = self.state();
        state.drop_value = 0.0;
        self.set_state(state);

        let me = self.as_ref().checked()?;
        let start = me.k2_get_actor_location().to_dvec3();
        let end = start - glam::DVec3::Z * f64::from(self.soft_collision_trace_distance());

        let object_types: Vec<_> = object_type_query(self.soft_collision_object_type())
            .into_iter()
            .collect();

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

        if hit {
            self.set_soft_collision(true)?;
        }

        Ok(())
    }

    pub fn set_soft_collision(&self, b_enabled: bool) -> RustealResult<()> {
        let response = if b_enabled {
            ECollisionResponse::ECR_Ignore
        } else {
            ECollisionResponse::ECR_Block
        };

        self.as_ref()
            .checked()?
            .get_capsule_component()
            .checked()?
            .set_collision_response_to_channel(self.soft_collision_object_type(), response);

        Ok(())
    }

    fn name(&self) -> String {
        self.as_ref()
            .get_name()
            .unwrap_or_else(|_| "SideScrollingCharacter".into())
    }
}
