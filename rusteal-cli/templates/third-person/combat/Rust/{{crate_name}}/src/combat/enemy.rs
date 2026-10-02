// CombatEnemy: the Combat variant's `ACombatEnemy` in Rust. An AI character
// with the player's combat moves, run by a StateTree (`CombatAIController`).
//
// The C++ class tells its StateTree tasks that an attack ended or that it
// landed through C++ delegates; here the task waiting for it registers with
// the enemy, which finishes it. And its `OnEnemyDied` delegate, which only its
// spawner listens to, is a call to that spawner.

use bindings::engine::{
    Actor, ActorExt, AnimInstance, AnimInstanceExt, AnimMontage, CapsuleComponentExt, Character,
    CharacterExt, CharacterMovementComponentExt, EAutoPossessAI, ECollisionChannel,
    ECollisionEnabled, EDrawDebugTrace, GameplayStatics, KismetMathLibrary, KismetSystemLibrary,
    PawnExt, PrimitiveComponentExt, SceneComponentExt,
    SkeletalMeshComponentExt,
};
use bindings::engine::{ActorComponentExt, FHitResult, FHitResultExt};
use bindings::prelude::*;
use bindings::state_tree::{StateTreeTaskBlueprintBase, StateTreeTaskBlueprintBaseExt};
use bindings::umg::{WidgetComponent, WidgetComponentExt};
use glam::DVec3;
use rusteal_runtime::runtime::{
    FName, LOG_WARNING, OwnedStruct, RustealResult, SubclassOf, UObjectRef, UStructRef, UeArray,
    ulog,
};
use rusteal_runtime::{uclass, uclass_impl};

use super::ai_controller::CombatAIController;
use super::enemy_spawner::CombatEnemySpawner;
use super::interfaces::{self, CombatAttacker, CombatDamageable};
use super::life_bar::CombatLifeBar;
use super::model::{self, EnemyState};

#[uclass(parent = Character)]
pub struct CombatEnemy {
    /// Overhead life bar widget component
    #[component(attach = "root_component")]
    life_bar: WidgetComponent,

    /// Max amount of HP the character will have on respawn
    #[uproperty(EditAnywhere, default = model::ENEMY_MAX_HP, name = "MaxHP")]
    max_hp: f32,

    /// Current amount of HP the character has
    #[uproperty(VisibleAnywhere, BlueprintReadOnly, default = model::ENEMY_MAX_HP, name = "CurrentHP")]
    current_hp: f32,

    /// Name of the pelvis bone, for damage ragdoll physics
    #[uproperty(EditAnywhere)]
    pelvis_bone_name: FName,

    /// Pointer to the life bar widget
    #[uproperty(EditAnywhere)]
    life_bar_widget: UObjectRef<CombatLifeBar>,

    /// Distance ahead of the character that melee attack sphere collision traces will extend
    #[uproperty(EditAnywhere, default = model::ENEMY_MELEE_TRACE_DISTANCE)]
    melee_trace_distance: f32,

    /// Radius of the sphere trace for melee attacks
    #[uproperty(EditAnywhere, default = model::ENEMY_MELEE_TRACE_RADIUS)]
    melee_trace_radius: f32,

    /// Amount of damage a melee attack will deal
    #[uproperty(EditAnywhere, default = model::ENEMY_MELEE_DAMAGE)]
    melee_damage: f32,

    /// Amount of knockback impulse a melee attack will apply
    #[uproperty(EditAnywhere, default = model::ENEMY_MELEE_KNOCKBACK_IMPULSE)]
    melee_knockback_impulse: f32,

    /// Amount of upwards impulse a melee attack will apply
    #[uproperty(EditAnywhere, default = model::ENEMY_MELEE_LAUNCH_IMPULSE)]
    melee_launch_impulse: f32,

    /// AnimMontage that will play for combo attacks
    #[uproperty(EditAnywhere)]
    combo_attack_montage: UObjectRef<AnimMontage>,

    /// Names of the AnimMontage sections that correspond to each stage of the combo attack
    #[uproperty(EditAnywhere)]
    combo_section_names: UeArray<FName>,

    /// AnimMontage that will play for charged attacks
    #[uproperty(EditAnywhere)]
    charged_attack_montage: UObjectRef<AnimMontage>,

    /// Name of the AnimMontage section that corresponds to the charge loop
    #[uproperty(EditAnywhere)]
    charge_loop_section: FName,

    /// Name of the AnimMontage section that corresponds to the attack
    #[uproperty(EditAnywhere)]
    charge_attack_section: FName,

    /// Minimum number of charge animation loops that will be played by the AI
    #[uproperty(EditAnywhere, default = model::MIN_CHARGE_LOOPS)]
    min_charge_loops: i32,

    /// Maximum number of charge animation loops that will be played by the AI
    #[uproperty(EditAnywhere, default = model::MAX_CHARGE_LOOPS)]
    max_charge_loops: i32,

    /// Time to wait before removing this character from the level after it dies
    #[uproperty(EditAnywhere, default = model::DEATH_REMOVAL_TIME)]
    death_removal_time: f32,

    /// attack counters and the last danger seen
    state: EnemyState,

    /// whether the attack montages' end is already routed to `attack_montage_ended`
    attack_end_bound: bool,

    /// the StateTree task waiting for the current attack to end (`OnAttackCompleted`)
    attack_completed_task: UObjectRef<StateTreeTaskBlueprintBase>,

    /// the StateTree task waiting for the enemy to land (`OnEnemyLanded`)
    landed_task: UObjectRef<StateTreeTaskBlueprintBase>,

    /// the spawner to tell of this enemy's death (`OnEnemyDied`)
    spawner: UObjectRef<Actor>,
}

#[uclass_impl]
impl CombatEnemy {
    /// Everything `ACombatEnemy::ACombatEnemy()` sets.
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        let me = self.as_ref().checked()?;
        // set the AI Controller class by default
        me.set_ai_controller_class(SubclassOf::<CombatAIController>::base().upcast_to());
        // use an AI Controller regardless of whether we're placed or spawned
        me.set_auto_possess_ai(EAutoPossessAI::PlacedInWorldOrSpawned);
        // ignore the controller's yaw rotation
        me.set_use_controller_rotation_yaw(false);
        // set the collision capsule size
        me.get_capsule_component().checked()?.set_capsule_size(
            model::CAPSULE_RADIUS,
            model::CAPSULE_HALF_HEIGHT,
            Some(true),
        );
        // set the character movement properties
        me.get_character_movement().checked()?.set_use_controller_desired_rotation(true);
        Ok(())
    }

    /// Gameplay initialization
    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        // reset HP to maximum
        self.set_current_hp(self.max_hp());
        // get the life bar widget from the widget comp, fill the life bar
        let filled = self
            .life_bar()
            .and_then(|life_bar| life_bar.checked()?.get_user_widget_object().cast::<CombatLifeBar>())
            .and_then(|widget| {
                self.set_life_bar_widget(widget);
                CombatLifeBar::from_obj(widget)
            });
        match filled {
            Ok(widget) => widget.set_life_percentage(1.0),
            Err(e) => ulog!(LOG_WARNING, "[Combat] enemy life bar: {e}"),
        }
    }

    /// Gameplay cleanup
    #[ufunction(Override)]
    fn receive_end_play(&mut self, _end_play_reason: u8) {
        // clear the death timer
        KismetSystemLibrary::k2_clear_timer(self.as_ref().upcast_to(), "RemoveFromLevel");
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
        // call the landed Delegate for StateTree
        finish(self.landed_task());
    }

    /// Performs the combo string check
    #[ufunction(BlueprintCallable)]
    fn check_combo(&mut self) {
        let mut state = self.state();
        // increase the combo counter
        state.current_combo_attack += 1;
        self.set_state(state);
        // do we still have attacks to play in this string?
        if state.current_combo_attack >= state.target_combo_count {
            return;
        }
        // jump to the next attack section
        let Ok(sections) = self.combo_section_names().to_vec() else {
            return;
        };
        if let (Some(section), Ok(anim_instance)) =
            (sections.get(state.current_combo_attack as usize), self.anim_instance().and_then(|a| a.checked()))
        {
            anim_instance.montage_jump_to_section(section.handle(), Some(self.combo_attack_montage()));
        }
    }

    /// Performs the charged attack hold check
    #[ufunction(BlueprintCallable)]
    fn check_charged_attack(&mut self) {
        let mut state = self.state();
        // increase the charge loop counter
        state.current_charge_loop += 1;
        self.set_state(state);
        // jump to either the loop or attack section of the montage depending on whether we hit the loop target
        let section = if state.current_charge_loop >= state.target_charge_loops {
            self.charge_attack_section()
        } else {
            self.charge_loop_section()
        };
        if let Ok(anim_instance) = self.anim_instance().and_then(|a| a.checked()) {
            anim_instance.montage_jump_to_section(section.handle(), Some(self.charged_attack_montage()));
        }
    }

    /// Removes this character from the level after it dies (the death timer's function)
    #[ufunction(BlueprintCallable)]
    fn remove_from_level(&mut self) {
        // destroy this actor
        if let Ok(me) = self.as_ref().checked() {
            me.k2_destroy_actor();
        }
    }

    /// Blueprint handler to play damage received effects
    #[ufunction(BlueprintImplementableEvent)]
    fn received_damage(&self, damage: f32, impact_point: &OwnedStruct<FVector>, damage_direction: &OwnedStruct<FVector>) {}
}

/// Finish a waiting StateTree task, if any: what the C++ delegates' lambdas do.
fn finish(task: UObjectRef<StateTreeTaskBlueprintBase>) {
    if let Ok(task) = task.checked() {
        task.finish_task(Some(true));
    }
}

impl CombatEnemy {
    /// Performs an AI-initiated combo attack. Number of hits will be decided by this character.
    pub fn do_ai_combo_attack(&mut self) {
        let mut state = self.state();
        // ignore if we're already playing an attack animation
        if state.is_attacking {
            return;
        }
        // raise the attacking flag
        state.is_attacking = true;
        // choose how many times we're going to attack
        let sections = self.combo_section_names().len().unwrap_or(0) as i32;
        state.target_combo_count = KismetMathLibrary::random_integer_in_range(1, sections - 1);
        // reset the attack counter
        state.current_combo_attack = 0;
        self.set_state(state);
        // play the attack montage
        self.play_attack_montage(self.combo_attack_montage());
    }

    /// Performs an AI-initiated charged attack. Charge time will be decided by this character.
    pub fn do_ai_charged_attack(&mut self) {
        let mut state = self.state();
        // ignore if we're already playing an attack animation
        if state.is_attacking {
            return;
        }
        // raise the attacking flag
        state.is_attacking = true;
        // choose how many loops are we going to charge for
        state.target_charge_loops =
            KismetMathLibrary::random_integer_in_range(self.min_charge_loops(), self.max_charge_loops());
        // reset the charge loop counter
        state.current_charge_loop = 0;
        self.set_state(state);
        // play the attack montage
        self.play_attack_montage(self.charged_attack_montage());
    }

    /// The StateTree task to finish when the current attack ends (`OnAttackCompleted`).
    pub fn set_attack_completed_listener(&mut self, task: UObjectRef<StateTreeTaskBlueprintBase>) {
        self.set_attack_completed_task(task);
    }

    /// The StateTree task to finish when the enemy lands (`OnEnemyLanded`).
    pub fn set_landed_listener(&mut self, task: UObjectRef<StateTreeTaskBlueprintBase>) {
        self.set_landed_task(task);
    }

    /// The spawner to tell of this enemy's death (`OnEnemyDied`).
    pub fn set_death_listener(&mut self, spawner: UObjectRef<Actor>) {
        self.set_spawner(spawner);
    }

    /// Returns the last recorded location we were attacked from
    pub fn last_danger_location(&self) -> DVec3 {
        self.state().last_danger_location
    }

    /// Returns the last game time we were attacked
    pub fn last_danger_time(&self) -> f64 {
        self.state().last_danger_time
    }

    /// Play an attack montage, its end routed to `attack_montage_ended`.
    fn play_attack_montage(&mut self, montage: UObjectRef<AnimMontage>) {
        let Ok(anim_instance) = self.anim_instance() else {
            return;
        };
        if let Err(e) = self.bind_attack_end(anim_instance) {
            ulog!(LOG_WARNING, "[Combat] cannot watch the enemy's attack montage: {e}");
        }
        if let Ok(anim_instance) = anim_instance.checked() {
            anim_instance.montage_play(montage, Some(1.0), None, Some(0.0), Some(true));
        }
    }

    /// Route the end of the attack montages to `attack_montage_ended`, once.
    fn bind_attack_end(&mut self, anim_instance: UObjectRef<AnimInstance>) -> RustealResult<()> {
        if self.attack_end_bound() {
            return Ok(());
        }
        let enemy: UObjectRef<Character> = self.as_ref();
        anim_instance
            .checked()?
            .on_montage_ended()
            .add(move |montage, _interrupted| {
                if let Ok(mut me) = CombatEnemy::from_obj(enemy)
                    && (montage == me.combo_attack_montage() || montage == me.charged_attack_montage())
                {
                    me.attack_montage_ended();
                }
            })?
            .detach();
        self.set_attack_end_bound(true);
        Ok(())
    }

    /// Called when the attack montage ends
    fn attack_montage_ended(&mut self) {
        // reset the attacking flag
        let mut state = self.state();
        state.is_attacking = false;
        self.set_state(state);
        // call the attack completed delegate so the StateTree can continue execution
        finish(self.attack_completed_task());
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

    fn anim_instance(&self) -> RustealResult<UObjectRef<AnimInstance>> {
        Ok(self.as_ref().checked()?.get_mesh().checked()?.get_anim_instance())
    }
}

impl CombatAttacker for CombatEnemy {
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
        // enemies only affect Pawn collision objects; they don't knock back boxes
        let object_types: Vec<_> = object_type_query(ECollisionChannel::ECC_Pawn).into_iter().collect();
        let (_, hits) = KismetSystemLibrary::sphere_trace_multi_for_objects(
            me.as_ref().upcast_to(),
            &FVector::from_dvec3(start),
            &FVector::from_dvec3(end),
            self.melee_trace_radius(),
            &object_types,
            false,
            &[],
            EDrawDebugTrace::None,
            true,
            &OwnedStruct::new(),
            &OwnedStruct::new(),
            None,
        );
        let player_tag = FName::new(model::PLAYER_TAG).handle();
        for hit in hits {
            let hit = hit.as_ref();
            let Ok(actor) = hit.get_component().checked().map(|c| c.get_owner()) else {
                continue;
            };
            // does the actor have the player tag?
            if !actor.checked().is_ok_and(|a| a.actor_has_tag(player_tag)) {
                continue;
            }
            // check if the actor is damageable
            if let Some(mut damageable) = interfaces::damageable(actor) {
                // knock upwards and away from the impact normal
                let impulse = model::knockback(
                    hit.get_impact_normal().to_dvec3(),
                    self.melee_knockback_impulse(),
                    self.melee_launch_impulse(),
                );
                // pass the damage event to the actor
                damageable.apply_damage(
                    self.melee_damage(),
                    me.as_ref().upcast_to(),
                    hit.get_impact_point().to_dvec3(),
                    impulse,
                );
            }
        }
    }

    fn check_combo(&mut self) {
        CombatEnemy::check_combo(self);
    }

    fn check_charged_attack(&mut self) {
        CombatEnemy::check_charged_attack(self);
    }
}

impl CombatDamageable for CombatEnemy {
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
        if let Ok(mesh) = me.get_mesh().checked() {
            // is the character ragdolling?
            if mesh.is_simulating_physics(None) {
                // apply an impulse to the ragdoll
                let impulse = damage_impulse * f64::from(mesh.get_mass());
                mesh.add_impulse_at_location(&FVector::from_dvec3(impulse), &FVector::from_dvec3(damage_location), None);
            }
            // stop the attack montages to interrupt the attack
            if let Ok(anim_instance) = mesh.get_anim_instance().checked() {
                anim_instance.montage_stop(0.1, Some(self.combo_attack_montage()));
                anim_instance.montage_stop(0.1, Some(self.charged_attack_montage()));
            }
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
        // hide the life bar
        if let Ok(life_bar) = self.life_bar().and_then(|l| l.checked()) {
            life_bar.set_hidden_in_game(true, None);
        }
        // disable the collision capsule to avoid being hit again while dead
        if let Ok(capsule) = me.get_capsule_component().checked() {
            capsule.set_collision_enabled(ECollisionEnabled::NoCollision);
        }
        // disable character movement
        if let Ok(movement) = me.get_character_movement().checked() {
            movement.disable_movement();
        }
        // enable full ragdoll physics
        if let Ok(mesh) = me.get_mesh().checked() {
            mesh.set_simulate_physics(true);
        }
        // call the died delegate to notify any subscribers
        if let Ok(mut spawner) = CombatEnemySpawner::from_obj(self.spawner()) {
            spawner.on_enemy_died();
        }
        // set up the death timer
        KismetSystemLibrary::k2_set_timer(
            self.as_ref().upcast_to(),
            "RemoveFromLevel",
            self.death_removal_time(),
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
    fn notify_danger(&mut self, danger_location: DVec3, danger_source: UObjectRef<Actor>) {
        // ensure we're being attacked by the player
        let player_tag = FName::new(model::PLAYER_TAG).handle();
        if !danger_source.checked().is_ok_and(|source| source.actor_has_tag(player_tag)) {
            return;
        }
        // save the danger location and game time
        let mut state = self.state();
        state.last_danger_location = danger_location;
        state.last_danger_time = GameplayStatics::get_time_seconds(self.as_ref().upcast_to());
        self.set_state(state);
    }
}
