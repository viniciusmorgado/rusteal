// TwinStickCharacter: the TwinStick variant's `ATwinStickCharacter` in Rust,
// a player character for a twin stick shooter: it moves with one stick, aims
// with the other (or the mouse) and turns to face its aim, shoots
// projectiles, dashes, and spends items on area attacks.
//
// `SetupPlayerInputComponent`, `NotifyControllerChanged` and `Tick` are C++
// virtuals; here they are the `ReceiveRestarted`, `ReceiveControllerChanged`
// and `ReceiveTick` events.

use bindings::engine::{
    ActorExt, Character, CharacterExt, CharacterMovementComponentExt, Controller,
    ESpawnActorCollisionHandlingMethod, ETraceTypeQuery, FHitResultExt, GameplayStatics,
    KismetMathLibrary, KismetSystemLibrary, PawnExt, PlayerController, PlayerControllerExt,
    SceneComponentExt, SpringArmComponent, SpringArmComponentExt, CameraComponent,
    CameraComponentExt, MovementComponentExt,
};
use bindings::enhanced_input::{ETriggerEvent, FInputActionValue, InputAction};
use bindings::prelude::*;
use glam::DVec2;
use rusteal_runtime::runtime::{
    LOG_WARNING, Rotator, RustealResult, SubclassOf, UObjectRef, UStructRef, ulog,
};
use rusteal_runtime::{uclass, uclass_impl};

use super::aoe_attack::TwinStickAoEAttack;
use super::game_mode::TwinStickGameMode;
use super::model;
use super::projectile::TwinStickProjectile;

/// The camera and movement setup `ATwinStickCharacter::ATwinStickCharacter()`
/// gives the character.
const SPRING_ARM_PITCH: f64 = -50.0;
const SPRING_ARM_LENGTH: f32 = 2200.0;
const CAMERA_LAG_SPEED: f32 = 0.5;
const CAMERA_FIELD_OF_VIEW: f32 = 75.0;
const GRAVITY_SCALE: f32 = 1.5;
const MAX_ACCELERATION: f32 = 1000.0;
const BRAKING_FRICTION_FACTOR: f32 = 1.0;
const ROTATION_RATE_YAW: f64 = 640.0;

#[uclass(parent = Character)]
pub struct TwinStickCharacter {
    /// Camera boom spring arm
    #[component(attach = "root_component", name = "Spring Arm")]
    spring_arm: SpringArmComponent,

    /// Player Camera
    #[component(attach = "spring_arm")]
    camera: CameraComponent,

    /// Movement input action
    #[uproperty(EditAnywhere, category = "Input")]
    move_action: UObjectRef<InputAction>,

    /// Gamepad aim input action
    #[uproperty(EditAnywhere, category = "Input")]
    stick_aim_action: UObjectRef<InputAction>,

    /// Mouse aim input action
    #[uproperty(EditAnywhere, category = "Input")]
    mouse_aim_action: UObjectRef<InputAction>,

    /// Dash input action
    #[uproperty(EditAnywhere, category = "Input")]
    dash_action: UObjectRef<InputAction>,

    /// Shooting input action
    #[uproperty(EditAnywhere, category = "Input")]
    shoot_action: UObjectRef<InputAction>,

    /// AoE attack input action
    #[uproperty(EditAnywhere, category = "Input")]
    ao_e_action: UObjectRef<InputAction>,

    /// Trace channel to use for mouse aim
    #[uproperty(EditAnywhere, category = "Input", default = ETraceTypeQuery::TraceTypeQuery1)]
    mouse_aim_trace_channel: ETraceTypeQuery,

    /// Impulse to apply to the character when dashing
    #[uproperty(EditAnywhere, category = "Dash", default = 2500.0)]
    dash_impulse: f32,

    /// Type of projectile to spawn when shooting
    #[uproperty(EditAnywhere, category = "Projectile")]
    projectile_class: SubclassOf<TwinStickProjectile>,

    /// Distance ahead of the character that the projectile will be spawned at
    #[uproperty(EditAnywhere, category = "Projectile", default = 100.0)]
    projectile_offset: f32,

    /// Type of AoE attack actor to spawn
    #[uproperty(EditAnywhere, category = "AoE")]
    ao_e_attack_class: SubclassOf<TwinStickAoEAttack>,

    /// Number of starting AoE attack items
    #[uproperty(EditAnywhere, category = "AoE", default = 1)]
    items: i32,

    /// Knockback impulse to apply to the character when they're damaged
    #[uproperty(EditAnywhere, category = "Damage", default = 2500.0)]
    knockback_strength: f32,

    /// Time to disallow AoE attacks after one is performed
    #[uproperty(EditAnywhere, category = "AoE", default = 1.0)]
    ao_e_cooldown_time: f32,

    /// Speed to blend between our current rotation and the target aim rotation when stick aiming
    #[uproperty(EditAnywhere, category = "Aim", default = 10.0)]
    aim_rotation_interp_speed: f32,

    /// Time to wait between autofire attempts
    #[uproperty(EditAnywhere, category = "Aim", default = 0.2)]
    auto_fire_delay: f32,

    /// Pointer to the player controller assigned to this character
    #[uproperty]
    player_controller: UObjectRef<PlayerController>,

    /// Game time of the last AoE attack
    last_ao_e_time: f32,

    /// Aim Yaw Angle in degrees
    aim_angle: f32,

    /// If true, the player is using mouse aim
    using_mouse: bool,

    /// Last held move input
    last_move_input: DVec2,

    /// If true, the player is auto firing while stick aiming
    auto_fire_active: bool,
}

#[uclass_impl]
impl TwinStickCharacter {
    /// Everything `ATwinStickCharacter::ATwinStickCharacter()` sets.
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        // create the spring arm
        let spring_arm = self.spring_arm()?.checked()?;
        spring_arm.k2_set_relative_rotation(
            &FRotator::from_rotator(Rotator::new(SPRING_ARM_PITCH, 0.0, 0.0)),
            false,
            false,
        );
        spring_arm.set_target_arm_length(SPRING_ARM_LENGTH);
        spring_arm.set_do_collision_test(false);
        spring_arm.set_inherit_yaw(false);
        spring_arm.set_enable_camera_lag(true);
        spring_arm.set_camera_lag_speed(CAMERA_LAG_SPEED);

        // create the camera
        self.camera()?.checked()?.set_field_of_view(CAMERA_FIELD_OF_VIEW);

        // configure the character movement
        let movement = self.as_ref().checked()?.get_character_movement().checked()?;
        movement.set_gravity_scale(GRAVITY_SCALE);
        movement.set_max_acceleration(MAX_ACCELERATION);
        movement.set_braking_friction_factor(BRAKING_FRICTION_FACTOR);
        movement.set_can_walk_off_ledges(false);
        movement.set_rotation_rate(&FRotator::from_rotator(Rotator::new(0.0, ROTATION_RATE_YAW, 0.0)));
        movement.set_plane_constraint_enabled(true);
        movement.set_snap_to_plane_at_start(true);
        Ok(())
    }

    /// Gameplay Initialization
    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        // update the items count
        self.update_items();
    }

    /// Gameplay cleanup
    #[ufunction(Override)]
    fn receive_end_play(&mut self, _end_play_reason: u8) {
        // Clear the autofire timer
        KismetSystemLibrary::k2_clear_timer(self.as_ref().upcast_to(), "ResetAutoFire");
    }

    /// Possessed by controller initialization
    #[ufunction(Override)]
    fn receive_controller_changed(&mut self, _old_controller: UObjectRef<Controller>, new_controller: UObjectRef<Controller>) {
        // set the player controller reference
        self.set_player_controller(new_controller.cast().unwrap_or_default());
    }

    /// Updates the character's rotation to face the aim direction
    #[ufunction(Override)]
    fn receive_tick(&mut self, _delta_seconds: f32) {
        let Ok(me) = self.as_ref().checked() else {
            return;
        };

        // get the current rotation
        let old_rotation = me.k2_get_actor_rotation().to_rotator();

        // are we aiming with the mouse?
        if self.using_mouse() {
            let Ok(player_controller) = self.player_controller().checked() else {
                return;
            };

            // get the cursor world location
            let (_, out_hit) = player_controller.get_hit_result_under_cursor_by_channel(self.mouse_aim_trace_channel(), true);

            // find the aim rotation and save the aim angle
            let cursor = FVector::from_dvec3(out_hit.as_ref().get_location().to_dvec3());
            let aim_rotation = KismetMathLibrary::find_look_at_rotation(&me.k2_get_actor_location(), &cursor);
            self.set_aim_angle(aim_rotation.to_rotator().yaw as f32);
        }

        // update the yaw, reuse the pitch and roll
        let target = Rotator::new(old_rotation.pitch, f64::from(self.aim_angle()), old_rotation.roll);
        me.k2_set_actor_rotation(&FRotator::from_rotator(target), false);
    }

    /// Adds input bindings
    #[ufunction(Override)]
    fn receive_restarted(&mut self) {
        if let Err(e) = self.bind_input() {
            ulog!(LOG_WARNING, "[TwinStick] failed to bind the input: {e}");
        }
    }

    /// Handles movement inputs
    #[ufunction]
    fn r#move(&mut self, value: UStructRef<FInputActionValue>) {
        // save the input vector and route the input
        let input = value.axis2d();
        self.do_move(input.x as f32, input.y as f32);
    }

    /// Handles joypad aim
    #[ufunction]
    fn stick_aim(&mut self, value: UStructRef<FInputActionValue>) {
        // get the input vector and route the input
        let input = value.axis2d();
        self.do_aim(input.x as f32, input.y as f32);
    }

    /// Handles mouse aim
    #[ufunction]
    fn mouse_aim(&mut self, _value: UStructRef<FInputActionValue>) {
        // raise the mouse controls flag
        self.set_using_mouse(true);

        // show the mouse cursor
        if let Ok(player_controller) = self.player_controller().checked() {
            player_controller.set_show_mouse_cursor(true);
        }
    }

    /// Performs a dash
    #[ufunction]
    fn dash(&mut self, _value: UStructRef<FInputActionValue>) {
        self.do_dash();
    }

    /// Shoots projectiles
    #[ufunction]
    fn shoot(&mut self, _value: UStructRef<FInputActionValue>) {
        self.do_shoot();
    }

    /// Performs an AoE Attack
    #[ufunction(name = "AoEAttack")]
    fn ao_e_attack(&mut self, _value: UStructRef<FInputActionValue>) {
        self.do_ao_e_attack();
    }

    /// Handles move inputs from both input actions and touch interface
    #[ufunction(BlueprintCallable)]
    fn do_move(&mut self, axis_x: f32, axis_y: f32) {
        // save the input
        self.set_last_move_input(DVec2::new(f64::from(axis_x), f64::from(axis_y)));
        let Ok(me) = self.as_ref().checked() else {
            return;
        };

        // calculate the forward component of the input
        let control = me.get_control_rotation().to_rotator();
        let flat = FRotator::from_rotator(Rotator::new(0.0, control.yaw, control.roll));

        // apply the forward input
        let forward = KismetMathLibrary::get_forward_vector(&flat);
        me.add_movement_input(&forward, Some(axis_x), Some(false));

        // apply the right input
        let right = KismetMathLibrary::get_right_vector(&flat);
        me.add_movement_input(&right, Some(axis_y), Some(false));
    }

    /// Handles aim inputs from both input actions and touch interface
    #[ufunction(BlueprintCallable)]
    fn do_aim(&mut self, axis_x: f32, axis_y: f32) {
        // calculate the aim angle from the inputs
        self.set_aim_angle(model::aim_angle(axis_x, axis_y));

        // lower the mouse controls flag
        self.set_using_mouse(false);

        // hide the mouse cursor
        if let Ok(player_controller) = self.player_controller().checked() {
            player_controller.set_show_mouse_cursor(false);
        }

        // are we on autofire cooldown?
        if !self.auto_fire_active() {
            // set ourselves on cooldown
            self.set_auto_fire_active(true);

            // fire a projectile
            self.do_shoot();

            // schedule autofire cooldown reset
            KismetSystemLibrary::k2_set_timer(
                self.as_ref().upcast_to(),
                "ResetAutoFire",
                self.auto_fire_delay(),
                false,
                None,
                None,
                None,
            );
        }
    }

    /// Handles dash inputs from both input actions and touch interface
    #[ufunction(BlueprintCallable)]
    fn do_dash(&mut self) {
        // calculate the launch impulse vector based on the last move input
        let launch_dir = model::dash_direction(self.last_move_input());

        // launch the character in the chosen direction
        if let Ok(me) = self.as_ref().checked() {
            me.launch_character(&FVector::from_dvec3(launch_dir * f64::from(self.dash_impulse())), true, true);
        }
    }

    /// Handles shoot inputs from both input actions and touch interface
    #[ufunction(BlueprintCallable)]
    fn do_shoot(&mut self) {
        if let Err(e) = self.spawn_projectile() {
            ulog!(LOG_WARNING, "[TwinStick] shooting failed: {e}");
        }
    }

    /// Handles aoe attack inputs from both input actions and touch interface
    #[ufunction(BlueprintCallable, name = "DoAoEAttack")]
    fn do_ao_e_attack(&mut self) {
        // do we have enough items to do an AoE attack?
        if self.items() <= 0 {
            return;
        }
        let Ok(me) = self.as_ref().checked() else {
            return;
        };

        // get the game time; are we off AoE cooldown?
        let game_time = GameplayStatics::get_time_seconds(me.as_ref().upcast_to()) as f32;
        if game_time - self.last_ao_e_time() <= self.ao_e_cooldown_time() {
            return;
        }

        // save the new AoE time
        self.set_last_ao_e_time(game_time);

        // spawn the AoE
        let transform = me.get_transform();
        let spawned = GameplayStatics::begin_deferred_actor_spawn_from_class(
            me.as_ref().upcast_to(),
            self.ao_e_attack_class().upcast_to(),
            &transform,
            None,
            None,
            None,
        );
        GameplayStatics::finish_spawning_actor(spawned, &transform, None);

        // decrease the number of items and update the items count
        self.set_items(self.items() - 1);
        self.update_items();
    }

    /// Allows Blueprint code to react to damage
    #[ufunction(BlueprintImplementableEvent, name = "BP_Damaged")]
    fn bp_damaged(&self) {}

    /// Resets stick the aim autofire flag after the autofire timer has expired
    #[ufunction]
    fn reset_auto_fire(&mut self) {
        // reset the autofire flag
        self.set_auto_fire_active(false);
    }
}

impl TwinStickCharacter {
    /// What `ATwinStickCharacter::SetupPlayerInputComponent` binds.
    fn bind_input(&self) -> RustealResult<()> {
        let me = self.as_ref();
        let pawn = me.checked()?;
        if !pawn.is_player_controlled() || !pawn.is_locally_controlled() {
            return Ok(());
        }

        // set up the enhanced input action bindings
        bind_action(&me, self.move_action(), ETriggerEvent::Triggered, "Move")?;
        bind_action(&me, self.stick_aim_action(), ETriggerEvent::Triggered, "StickAim")?;
        bind_action(&me, self.mouse_aim_action(), ETriggerEvent::Triggered, "MouseAim")?;
        bind_action(&me, self.dash_action(), ETriggerEvent::Triggered, "Dash")?;
        bind_action(&me, self.shoot_action(), ETriggerEvent::Triggered, "Shoot")?;
        bind_action(&me, self.ao_e_action(), ETriggerEvent::Triggered, "AoEAttack")?;
        Ok(())
    }

    /// `DoShoot`: a projectile ahead of the character, unless that spot
    /// collides with a wall or other nearby obstacle.
    fn spawn_projectile(&self) -> RustealResult<()> {
        let me = self.as_ref().checked()?;

        // get the actor transform and apply the projectile spawn offset
        let location = me.k2_get_actor_location().to_dvec3()
            + me.get_actor_forward_vector().to_dvec3() * f64::from(self.projectile_offset());
        let transform = KismetMathLibrary::make_transform(
            &FVector::from_dvec3(location),
            &me.k2_get_actor_rotation(),
            &me.get_actor_scale3_d(),
        );

        // ensure we don't spawn a projectile if it collides with a wall or other nearby obstacle
        let spawned = GameplayStatics::begin_deferred_actor_spawn_from_class(
            me.as_ref().upcast_to(),
            self.projectile_class().upcast_to(),
            &transform,
            Some(ESpawnActorCollisionHandlingMethod::DontSpawnIfColliding),
            None,
            None,
        );
        if spawned.is_valid() {
            GameplayStatics::finish_spawning_actor(spawned, &transform, None);
        }
        Ok(())
    }

    /// Applies collision impact to the player
    pub fn handle_damage(&self, _damage: f32, damage_direction: glam::DVec3) {
        let Ok(me) = self.as_ref().checked() else {
            return;
        };

        // calculate the knockback vector
        let launch = glam::DVec3::new(damage_direction.x, damage_direction.y, 0.0);

        // apply knockback to the character
        me.launch_character(&FVector::from_dvec3(launch * f64::from(self.knockback_strength())), true, true);

        // pass control to BP
        self.bp_damaged();
    }

    /// Gives the player a pickup item
    pub fn add_pickup(&self) {
        // increase the item count and update the items counter
        self.set_items(self.items() + 1);
        self.update_items();
    }

    /// Updates the items counter on the Game Mode
    fn update_items(&self) {
        let world = self.as_ref().upcast_to();
        if let Ok(mut game_mode) = TwinStickGameMode::from_obj(GameplayStatics::get_game_mode(world)) {
            game_mode.item_used(self.items());
        }
    }
}
