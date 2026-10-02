// CombatCharacter: the Combat variant's `ACombatCharacter` in Rust. A third
// person character with melee combat: a combo attack string, a press and hold
// charged attack, damage and reactions, death and respawn.
//
// The C++ virtuals it overrides are their Blueprint events here:
// `BeginPlay` is `ReceiveBeginPlay`, `EndPlay` is `ReceiveEndPlay`, `Landed`
// is `OnLanded`, `NotifyControllerChanged` is `ReceiveControllerChanged` and
// `SetupPlayerInputComponent` is `ReceiveRestarted`. `TakeDamage` is only
// called by the class itself, so it is a Rust method. The Blueprint child
// `BP_CombatCharacter` holds the mannequin, the input actions, the attack
// montages and the life bar widget, and plays the hit effects
// (`DealtDamage`, `ReceivedDamage`) and the camera toggle (`BP_ToggleCamera`).

use bindings::engine::{
    Actor, ActorExt, AnimInstanceExt, AnimMontage, CameraComponent, CameraComponentExt,
    CapsuleComponentExt, Character, CharacterExt, CharacterMovementComponentExt, Controller,
    ControllerExt, ECollisionChannel, EDrawDebugTrace, GameplayStatics, KismetSystemLibrary,
    PawnExt, PrimitiveComponentExt, SceneComponentExt, SkeletalMeshComponentExt,
    SpringArmComponent, SpringArmComponentExt,
};
use bindings::engine::{ActorComponentExt, FHitResult, FHitResultExt};
use bindings::enhanced_input::{ETriggerEvent, FInputActionValue, InputAction};
use bindings::prelude::*;
use bindings::umg::{WidgetComponent, WidgetComponentExt};
use glam::DVec3;
use rusteal_runtime::runtime::{
    FName, LOG_DISPLAY, LOG_WARNING, OwnedStruct, RustealResult, UObjectRef, UStructRef, UeArray,
    ulog,
};
use rusteal_runtime::{uclass, uclass_impl};

use super::interfaces::{self, CombatAttacker, CombatDamageable};
use super::life_bar::CombatLifeBar;
use super::model::{self, AttackState};
use super::player_controller::CombatPlayerController;

#[uclass(parent = Character)]
pub struct CombatCharacter {
    /// Camera boom positioning the camera behind the character
    #[component(attach = "root_component")]
    camera_boom: SpringArmComponent,

    /// Follow camera
    #[component(attach = "camera_boom", socket = "SpringEndpoint")]
    follow_camera: CameraComponent,

    /// Life bar widget component
    #[component(attach = "root_component")]
    life_bar: WidgetComponent,

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

    /// Combo Attack Input Action
    #[uproperty(EditAnywhere)]
    combo_attack_action: UObjectRef<InputAction>,

    /// Charged Attack Input Action
    #[uproperty(EditAnywhere)]
    charged_attack_action: UObjectRef<InputAction>,

    /// Toggle Camera Side Input Action
    #[uproperty(EditAnywhere)]
    toggle_camera_action: UObjectRef<InputAction>,

    /// Max amount of HP the character will have on respawn
    #[uproperty(EditAnywhere, default = model::MAX_HP, name = "MaxHP")]
    max_hp: f32,

    /// Current amount of HP the character has
    #[uproperty(VisibleAnywhere, default = 0.0, name = "CurrentHP")]
    current_hp: f32,

    /// Life bar widget fill color
    #[uproperty(EditAnywhere)]
    life_bar_color: OwnedStruct<FLinearColor>,

    /// Name of the pelvis bone, for damage ragdoll physics
    #[uproperty(EditAnywhere)]
    pelvis_bone_name: FName,

    /// Pointer to the life bar widget
    #[uproperty(EditAnywhere)]
    life_bar_widget: UObjectRef<CombatLifeBar>,

    /// Max amount of time that may elapse for a non-combo attack input to not be considered stale
    #[uproperty(EditAnywhere, default = model::ATTACK_INPUT_CACHE_TIME_TOLERANCE)]
    attack_input_cache_time_tolerance: f32,

    /// Distance ahead of the character that melee attack sphere collision traces will extend
    #[uproperty(EditAnywhere, default = model::MELEE_TRACE_DISTANCE)]
    melee_trace_distance: f32,

    /// Radius of the sphere trace for melee attacks
    #[uproperty(EditAnywhere, default = model::MELEE_TRACE_RADIUS)]
    melee_trace_radius: f32,

    /// Distance ahead of the character that enemies will be notified of incoming attacks
    #[uproperty(EditAnywhere, default = model::DANGER_TRACE_DISTANCE)]
    danger_trace_distance: f32,

    /// Radius of the sphere trace to notify enemies of incoming attacks
    #[uproperty(EditAnywhere, default = model::DANGER_TRACE_RADIUS)]
    danger_trace_radius: f32,

    /// Amount of damage a melee attack will deal
    #[uproperty(EditAnywhere, default = model::MELEE_DAMAGE)]
    melee_damage: f32,

    /// Amount of knockback impulse a melee attack will apply
    #[uproperty(EditAnywhere, default = model::MELEE_KNOCKBACK_IMPULSE)]
    melee_knockback_impulse: f32,

    /// Amount of upwards impulse a melee attack will apply
    #[uproperty(EditAnywhere, default = model::MELEE_LAUNCH_IMPULSE)]
    melee_launch_impulse: f32,

    /// AnimMontage that will play for combo attacks
    #[uproperty(EditAnywhere)]
    combo_attack_montage: UObjectRef<AnimMontage>,

    /// Names of the AnimMontage sections that correspond to each stage of the combo attack
    #[uproperty(EditAnywhere)]
    combo_section_names: UeArray<FName>,

    /// Max amount of time that may elapse for a combo attack input to not be considered stale
    #[uproperty(EditAnywhere, default = model::COMBO_INPUT_CACHE_TIME_TOLERANCE)]
    combo_input_cache_time_tolerance: f32,

    /// AnimMontage that will play for charged attacks
    #[uproperty(EditAnywhere)]
    charged_attack_montage: UObjectRef<AnimMontage>,

    /// Name of the AnimMontage section that corresponds to the charge loop
    #[uproperty(EditAnywhere)]
    charge_loop_section: FName,

    /// Name of the AnimMontage section that corresponds to the attack
    #[uproperty(EditAnywhere)]
    charge_attack_section: FName,

    /// Camera boom length while the character is dead
    #[uproperty(EditAnywhere, default = model::DEATH_CAMERA_DISTANCE)]
    death_camera_distance: f32,

    /// Camera boom length when the character respawns
    #[uproperty(EditAnywhere, default = model::DEFAULT_CAMERA_DISTANCE)]
    default_camera_distance: f32,

    /// Time to wait before respawning the character
    #[uproperty(EditAnywhere, default = model::RESPAWN_TIME)]
    respawn_time: f32,

    /// attack flags, counters and the last cached attack input
    attack: AttackState,

    /// whether the attack montages' end is already routed to `attack_montage_ended`
    attack_end_bound: bool,
}

#[uclass_impl]
impl CombatCharacter {
    /// Everything `ACombatCharacter::ACombatCharacter()` sets.
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        let me = self.as_ref().checked()?;

        // Set size for collision capsule
        me.get_capsule_component().checked()?.set_capsule_size(
            model::CAPSULE_RADIUS,
            model::CAPSULE_HALF_HEIGHT,
            Some(false),
        );

        // Configure character movement
        me.get_character_movement().checked()?.set_max_walk_speed(model::MAX_WALK_SPEED);

        // create the camera boom
        let boom = self.camera_boom()?.checked()?;
        boom.set_target_arm_length(self.default_camera_distance());
        boom.set_use_pawn_control_rotation(true);
        boom.set_enable_camera_lag(true);
        boom.set_enable_camera_rotation_lag(true);

        // create the orbiting camera
        self.follow_camera()?.checked()?.set_use_pawn_control_rotation(false);

        // set the player tag
        me.tags().push(&FName::new(model::PLAYER_TAG).handle())?;
        Ok(())
    }

    /// Gameplay initialization
    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        if let Err(e) = self.begin_play() {
            ulog!(LOG_WARNING, "[Combat] {} BeginPlay failed: {e}", self.name());
        }
    }

    /// Gameplay cleanup
    #[ufunction(Override)]
    fn receive_end_play(&mut self, _end_play_reason: u8) {
        // clear the respawn timer
        KismetSystemLibrary::k2_clear_timer(self.as_ref().upcast_to(), "RespawnCharacter");
    }

    /// The pawn's player input component is set up: bind the input actions.
    #[ufunction(Override)]
    fn receive_restarted(&mut self) {
        match self.bind_input() {
            Ok(true) => ulog!(LOG_DISPLAY, "[Combat] {} input bound", self.name()),
            Ok(false) => {}
            Err(e) => ulog!(LOG_WARNING, "[Combat] {} failed to bind its input: {e}", self.name()),
        }
    }

    /// Handles possessed controller changes
    #[ufunction(Override)]
    fn receive_controller_changed(&mut self, _old_controller: UObjectRef<Controller>, new_controller: UObjectRef<Controller>) {
        // update the respawn transform on the Player Controller
        if let (Ok(controller), Ok(me)) = (CombatPlayerController::from_obj(new_controller), self.as_ref().checked()) {
            controller.set_respawn_transform(&me.get_transform());
        }
    }

    /// Handles landing events
    #[ufunction(Override)]
    fn on_landed(&mut self, _hit: UStructRef<FHitResult>) {
        // is the character still alive?
        if self.current_hp() >= 0.0
            && let Ok(mesh) = self.as_ref().checked().and_then(|me| me.get_mesh().checked())
        {
            // disable ragdoll physics
            mesh.set_physics_blend_weight(0.0);
        }
    }

    /// Called for movement input
    #[ufunction(BlueprintCallable)]
    fn r#move(&mut self, value: UStructRef<FInputActionValue>) {
        // input is a Vector2D
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

    /// Called for combo attack input
    #[ufunction(BlueprintCallable)]
    fn combo_attack_pressed(&mut self) {
        // route the input
        self.do_combo_attack_start();
    }

    /// Called for combo attack input pressed
    #[ufunction(BlueprintCallable)]
    fn charged_attack_pressed(&mut self) {
        // route the input
        self.do_charged_attack_start();
    }

    /// Called for combo attack input released
    #[ufunction(BlueprintCallable)]
    fn charged_attack_released(&mut self) {
        // route the input
        self.do_charged_attack_end();
    }

    /// Called for toggle camera side input
    #[ufunction(BlueprintCallable)]
    fn toggle_camera(&mut self) {
        // call the BP hook
        self.bp_toggle_camera();
    }

    /// BP hook to animate the camera side switch
    #[ufunction(BlueprintImplementableEvent, name = "BP_ToggleCamera")]
    fn bp_toggle_camera(&self) {}

    /// Handles move inputs from either controls or UI interfaces
    #[ufunction(BlueprintCallable)]
    fn do_move(&mut self, right: f32, forward: f32) {
        if let Err(e) = self.apply_move(right, forward) {
            ulog!(LOG_WARNING, "[Combat] DoMove failed: {e}");
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

    /// Handles combo attack pressed from either controls or UI interfaces
    #[ufunction(BlueprintCallable)]
    fn do_combo_attack_start(&mut self) {
        let mut attack = self.attack();
        // are we already playing an attack animation?
        if attack.is_attacking {
            // cache the input time so we can check it later
            attack.cached_attack_input_time = self.now();
            self.set_attack(attack);
            return;
        }
        // perform a combo attack
        self.combo_attack();
    }

    /// Handles combo attack released from either controls or UI interfaces
    #[ufunction(BlueprintCallable)]
    fn do_combo_attack_end(&mut self) {
        // stub
    }

    /// Handles charged attack pressed from either controls or UI interfaces
    #[ufunction(BlueprintCallable)]
    fn do_charged_attack_start(&mut self) {
        let mut attack = self.attack();
        // raise the charging attack flag
        attack.is_charging_attack = true;
        if attack.is_attacking {
            // do not attack if the charge animation hasn't looped at least once
            if !attack.has_looped_charged_attack {
                attack.has_released_charged_attack = false;
            }
            // cache the input time so we can check it later
            attack.cached_attack_input_time = self.now();
            self.set_attack(attack);
            return;
        }
        self.set_attack(attack);
        self.charged_attack();
    }

    /// Handles charged attack released from either controls or UI interfaces
    #[ufunction(BlueprintCallable)]
    fn do_charged_attack_end(&mut self) {
        let mut attack = self.attack();
        // lower the charging attack flag
        attack.is_charging_attack = false;
        // have we done the charge loop at least once and haven't released the button yet?
        let resolve = attack.has_looped_charged_attack && !attack.has_released_charged_attack;
        if resolve {
            // release the charge and resolve the attack
            attack.has_released_charged_attack = true;
        }
        self.set_attack(attack);
        if resolve {
            self.loop_or_resolve_charged_attack();
        }
    }

    /// Called after death to respawn the character (the respawn timer's function)
    #[ufunction(BlueprintCallable)]
    fn respawn_character(&mut self) {
        // destroy the character and let it be respawned by the Player Controller
        if let Ok(me) = self.as_ref().checked() {
            me.k2_destroy_actor();
        }
    }

    /// Blueprint handler to play damage dealt effects
    #[ufunction(BlueprintImplementableEvent)]
    fn dealt_damage(&self, damage: f32, impact_point: &OwnedStruct<FVector>) {}

    /// Blueprint handler to play damage received effects
    #[ufunction(BlueprintImplementableEvent)]
    fn received_damage(&self, damage: f32, impact_point: &OwnedStruct<FVector>, damage_direction: &OwnedStruct<FVector>) {}
}

impl CombatCharacter {
    /// `BeginPlay`: the life bar, the camera, the mesh, full HP.
    fn begin_play(&mut self) -> RustealResult<()> {
        // get the life bar from the widget component
        let life_bar_widget = self.life_bar()?.checked()?.get_user_widget_object().cast::<CombatLifeBar>()?;
        self.set_life_bar_widget(life_bar_widget);

        // initialize the camera
        self.camera_boom()?.checked()?.set_target_arm_length(self.default_camera_distance());

        // set the life bar color
        CombatLifeBar::from_obj(life_bar_widget)?.set_bar_color(&self.life_bar_color());

        // reset HP to maximum
        self.reset_hp()
    }

    /// Everything `ACombatCharacter::SetupPlayerInputComponent` does; `false`
    /// for a pawn that is not a local player's.
    fn bind_input(&self) -> RustealResult<bool> {
        let me = self.as_ref();
        let pawn = me.checked()?;
        if !pawn.is_player_controlled() || !pawn.is_locally_controlled() {
            return Ok(false);
        }

        // Moving
        bind_action(&me, self.move_action(), ETriggerEvent::Triggered, "Move")?;

        // Looking
        bind_action(&me, self.look_action(), ETriggerEvent::Triggered, "Look")?;
        bind_action(&me, self.mouse_look_action(), ETriggerEvent::Triggered, "Look")?;

        // Combo Attack
        bind_action(&me, self.combo_attack_action(), ETriggerEvent::Started, "ComboAttackPressed")?;

        // Charged Attack
        bind_action(&me, self.charged_attack_action(), ETriggerEvent::Started, "ChargedAttackPressed")?;
        bind_action(&me, self.charged_attack_action(), ETriggerEvent::Completed, "ChargedAttackReleased")?;

        // Camera Side Toggle
        bind_action(&me, self.toggle_camera_action(), ETriggerEvent::Triggered, "ToggleCamera")?;

        Ok(true)
    }

    /// `DoMove`: move along the controller's yaw.
    fn apply_move(&self, right: f32, forward: f32) -> RustealResult<()> {
        let me = self.as_ref().checked()?;
        let controller = me.get_controller();
        if !controller.is_valid() {
            return Ok(());
        }
        // find out which way is forward
        let yaw = controller.checked()?.get_control_rotation().to_rotator().yaw;
        let (sin, cos) = yaw.to_radians().sin_cos();
        let forward_direction = DVec3::new(cos, sin, 0.0);
        let right_direction = DVec3::new(-sin, cos, 0.0);
        // add movement
        me.add_movement_input(&FVector::from_dvec3(forward_direction), Some(forward), Some(false));
        me.add_movement_input(&FVector::from_dvec3(right_direction), Some(right), Some(false));
        Ok(())
    }

    /// Resets the character's current HP to maximum
    fn reset_hp(&mut self) -> RustealResult<()> {
        // reset the current HP total
        self.set_current_hp(self.max_hp());
        // update the life bar
        CombatLifeBar::from_obj(self.life_bar_widget())?.set_life_percentage(1.0);
        Ok(())
    }

    /// Performs a combo attack
    fn combo_attack(&mut self) {
        let mut attack = self.attack();
        // raise the attacking flag
        attack.is_attacking = true;
        // reset the combo count
        attack.combo_count = 0;
        self.set_attack(attack);
        // notify enemies they are about to be attacked
        self.notify_enemies_of_incoming_attack();
        // play the attack montage
        self.play_attack_montage(self.combo_attack_montage());
    }

    /// Performs a charged attack
    fn charged_attack(&mut self) {
        let mut attack = self.attack();
        // raise the attacking flag
        attack.is_attacking = true;
        // reset the charge loop flag
        attack.has_looped_charged_attack = false;
        // reset the charge release flag
        attack.has_released_charged_attack = false;
        self.set_attack(attack);
        // notify enemies they are about to be attacked
        self.notify_enemies_of_incoming_attack();
        // play the charged attack montage
        self.play_attack_montage(self.charged_attack_montage());
    }

    /// Play an attack montage, its end routed to `attack_montage_ended`.
    fn play_attack_montage(&mut self, montage: UObjectRef<AnimMontage>) {
        let Ok(anim_instance) = self.anim_instance() else {
            return;
        };
        if let Err(e) = self.bind_attack_end(anim_instance) {
            ulog!(LOG_WARNING, "[Combat] cannot watch the attack montage: {e}");
        }
        if let Ok(anim_instance) = anim_instance.checked() {
            anim_instance.montage_play(montage, Some(1.0), None, Some(0.0), Some(true));
        }
    }

    /// Route the end of the attack montages to `attack_montage_ended`, once:
    /// the C++ class sets the montage's end delegate on every attack.
    fn bind_attack_end(&mut self, anim_instance: UObjectRef<bindings::engine::AnimInstance>) -> RustealResult<()> {
        if self.attack_end_bound() {
            return Ok(());
        }
        let character: UObjectRef<Character> = self.as_ref();
        anim_instance
            .checked()?
            .on_montage_ended()
            .add(move |montage, interrupted| {
                if let Ok(mut me) = CombatCharacter::from_obj(character)
                    && (montage == me.combo_attack_montage() || montage == me.charged_attack_montage())
                {
                    me.attack_montage_ended(interrupted);
                }
            })?
            .detach();
        self.set_attack_end_bound(true);
        Ok(())
    }

    /// Called when the attack montage ends
    fn attack_montage_ended(&mut self, _interrupted: bool) {
        let mut attack = self.attack();
        // reset the attacking flag
        attack.is_attacking = false;
        self.set_attack(attack);
        // check if we have a non-stale cached input
        if model::input_is_fresh(self.now(), attack.cached_attack_input_time, self.attack_input_cache_time_tolerance()) {
            // are we holding the charged attack button?
            if attack.is_charging_attack {
                // do a charged attack
                self.charged_attack();
            } else {
                // do a regular attack
                self.combo_attack();
            }
        }
    }

    /// Resolves the charged attack: loop the charge or release the attack
    fn loop_or_resolve_charged_attack(&self) {
        // jump to either the loop or the attack section depending on whether we've released the charge
        let section = if self.attack().has_released_charged_attack {
            self.charge_attack_section()
        } else {
            self.charge_loop_section()
        };
        if let Ok(anim_instance) = self.anim_instance().and_then(|a| a.checked()) {
            anim_instance.montage_jump_to_section(section.handle(), Some(self.charged_attack_montage()));
        }
    }

    /// Notifies nearby enemies that an attack is coming so they can react
    fn notify_enemies_of_incoming_attack(&self) {
        let Ok(me) = self.as_ref().checked() else {
            return;
        };
        // sweep for objects in front of the character to be hit by the attack
        let start = me.k2_get_actor_location().to_dvec3();
        let end = start + me.get_actor_forward_vector().to_dvec3() * f64::from(self.danger_trace_distance());
        // check for pawn object types only
        for hit in self.sweep(start, end, self.danger_trace_radius(), &[ECollisionChannel::ECC_Pawn]) {
            let Ok(actor) = hit.as_ref().get_component().checked().map(|c| c.get_owner()) else {
                continue;
            };
            // check if we've hit a damageable actor
            if let Some(mut damageable) = interfaces::damageable(actor) {
                // notify the enemy
                damageable.notify_danger(start, me.as_ref().upcast_to());
            }
        }
    }

    /// Takes damage: HP, death, partial ragdoll; the damage taken.
    fn take_damage(&mut self, damage: f32) -> RustealResult<f32> {
        // only process damage if the character is still alive
        let Some((hp, dead)) = model::take_damage(self.current_hp(), damage) else {
            return Ok(0.0);
        };
        // reduce the current HP
        self.set_current_hp(hp);
        // have we run out of HP?
        if dead {
            // die
            self.handle_death();
        } else {
            // update the life bar
            CombatLifeBar::from_obj(self.life_bar_widget())?.set_life_percentage(hp / self.max_hp());
            // enable partial ragdoll physics, but keep the pelvis vertical
            let mesh = self.as_ref().checked()?.get_mesh().checked()?;
            mesh.set_physics_blend_weight(0.5);
            mesh.set_body_simulate_physics(self.pelvis_bone_name().handle(), false);
        }
        // return the received damage amount
        Ok(damage)
    }

    /// A sphere sweep for objects of these channels' types, ignoring self.
    pub(super) fn sweep(&self, start: DVec3, end: DVec3, radius: f32, channels: &[ECollisionChannel]) -> Vec<OwnedStruct<FHitResult>> {
        let object_types: Vec<_> = channels.iter().copied().filter_map(object_type_query).collect();
        KismetSystemLibrary::sphere_trace_multi_for_objects(
            self.as_ref().upcast_to(),
            &FVector::from_dvec3(start),
            &FVector::from_dvec3(end),
            radius,
            &object_types,
            false,
            &[],
            EDrawDebugTrace::None,
            true,
            &OwnedStruct::new(),
            &OwnedStruct::new(),
            None,
        )
        .1
    }

    fn anim_instance(&self) -> RustealResult<UObjectRef<bindings::engine::AnimInstance>> {
        Ok(self.as_ref().checked()?.get_mesh().checked()?.get_anim_instance())
    }

    fn now(&self) -> f64 {
        GameplayStatics::get_time_seconds(self.as_ref().upcast_to())
    }

    fn name(&self) -> String {
        self.as_ref().get_name().unwrap_or_else(|_| "CombatCharacter".into())
    }
}

impl CombatAttacker for CombatCharacter {
    /// Performs the collision check for an attack
    fn do_attack_trace(&mut self, damage_source_bone: &str) {
        let Ok(me) = self.as_ref().checked() else {
            return;
        };
        let Ok(mesh) = me.get_mesh().checked() else {
            return;
        };
        // start at the provided socket location, sweep forward
        let start = mesh.get_socket_location(FName::new(damage_source_bone).handle()).to_dvec3();
        let end = start + me.get_actor_forward_vector().to_dvec3() * f64::from(self.melee_trace_distance());
        // check for pawn and world dynamic collision object types
        let channels = [ECollisionChannel::ECC_Pawn, ECollisionChannel::ECC_WorldDynamic];
        for hit in self.sweep(start, end, self.melee_trace_radius(), &channels) {
            let hit = hit.as_ref();
            let Ok(actor) = hit.get_component().checked().map(|c| c.get_owner()) else {
                continue;
            };
            // check if we've hit a damageable actor
            let Some(mut damageable) = interfaces::damageable(actor) else {
                continue;
            };
            // knock upwards and away from the impact normal
            let impulse = model::knockback(
                hit.get_impact_normal().to_dvec3(),
                self.melee_knockback_impulse(),
                self.melee_launch_impulse(),
            );
            let impact_point = hit.get_impact_point().to_dvec3();
            // pass the damage event to the actor
            damageable.apply_damage(self.melee_damage(), me.as_ref().upcast_to(), impact_point, impulse);
            // call the BP handler to play effects, etc.
            self.dealt_damage(self.melee_damage(), &FVector::from_dvec3(impact_point));
        }
    }

    /// Performs the combo string check
    fn check_combo(&mut self) {
        let mut attack = self.attack();
        // are we playing a non-charge attack animation?
        if !attack.is_attacking || attack.is_charging_attack {
            return;
        }
        // is the last attack input not stale?
        if !model::input_is_fresh(self.now(), attack.cached_attack_input_time, self.combo_input_cache_time_tolerance()) {
            return;
        }
        // consume the attack input so we don't accidentally trigger it twice
        attack.cached_attack_input_time = 0.0;
        // increase the combo counter
        attack.combo_count += 1;
        self.set_attack(attack);

        // do we still have a combo section to play?
        let Ok(sections) = self.combo_section_names().to_vec() else {
            return;
        };
        let Some(section) = sections.get(attack.combo_count as usize).copied() else {
            return;
        };
        // notify enemies they are about to be attacked
        self.notify_enemies_of_incoming_attack();
        // jump to the next combo section
        if let Ok(anim_instance) = self.anim_instance().and_then(|a| a.checked()) {
            anim_instance.montage_jump_to_section(section.handle(), Some(self.combo_attack_montage()));
        }
    }

    /// Performs the charged attack hold check
    fn check_charged_attack(&mut self) {
        let mut attack = self.attack();
        // raise the looped charged attack flag
        attack.has_looped_charged_attack = true;
        // set the input release flag from the input. This will determine if we loop or resolve
        attack.has_released_charged_attack = !attack.is_charging_attack;
        self.set_attack(attack);
        // resolve the charge loop
        self.loop_or_resolve_charged_attack();
    }
}

impl CombatDamageable for CombatCharacter {
    /// Handles damage and knockback events
    fn apply_damage(&mut self, damage: f32, _damage_causer: UObjectRef<Actor>, damage_location: DVec3, damage_impulse: DVec3) {
        // pass the damage event to the actor
        let Ok(actual_damage) = self.take_damage(damage) else {
            return;
        };
        // only process knockback and effects if we received nonzero damage
        if actual_damage <= 0.0 {
            return;
        }
        let Ok(me) = self.as_ref().checked() else {
            return;
        };
        // apply the knockback impulse
        if let Ok(movement) = me.get_character_movement().checked() {
            movement.add_impulse(&FVector::from_dvec3(damage_impulse), Some(true));
        }
        // is the character ragdolling?
        if let Ok(mesh) = me.get_mesh().checked()
            && mesh.is_simulating_physics(None)
        {
            // apply an impulse to the ragdoll
            let impulse = damage_impulse * f64::from(mesh.get_mass());
            mesh.add_impulse_at_location(&FVector::from_dvec3(impulse), &FVector::from_dvec3(damage_location), None);
        }
        // pass control to BP to play effects, etc.
        self.received_damage(
            actual_damage,
            &FVector::from_dvec3(damage_location),
            &FVector::from_dvec3(model::safe_normal(damage_impulse)),
        );
    }

    /// Handles death events
    fn handle_death(&mut self) {
        let Ok(me) = self.as_ref().checked() else {
            return;
        };
        // disable movement while we're dead
        if let Ok(movement) = me.get_character_movement().checked() {
            movement.disable_movement();
        }
        // enable full ragdoll physics
        if let Ok(mesh) = me.get_mesh().checked() {
            mesh.set_simulate_physics(true);
        }
        // hide the life bar
        if let Ok(life_bar) = self.life_bar().and_then(|l| l.checked()) {
            life_bar.set_hidden_in_game(true, None);
        }
        // pull back the camera
        if let Ok(boom) = self.camera_boom().and_then(|b| b.checked()) {
            boom.set_target_arm_length(self.death_camera_distance());
        }
        // schedule respawning
        KismetSystemLibrary::k2_set_timer(
            self.as_ref().upcast_to(),
            "RespawnCharacter",
            self.respawn_time(),
            false,
            None,
            None,
            None,
        );
    }

    /// Handles healing events
    fn apply_healing(&mut self, _healing: f32, _healer: UObjectRef<Actor>) {
        // stub
    }

    /// Notifies the actor of impending danger
    fn notify_danger(&mut self, _danger_location: DVec3, _danger_source: UObjectRef<Actor>) {
        // stub
    }
}
