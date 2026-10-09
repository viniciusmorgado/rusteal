use bindings::engine::{
    Actor, ActorExt, AnimInstance, AnimInstanceExt, AnimMontage, CapsuleComponentExt, Character,
    CharacterExt, CharacterMovementComponentExt, EAutoPossessAI, ECollisionChannel,
    ECollisionEnabled, EDrawDebugTrace, GameplayStatics, KismetMathLibrary, KismetSystemLibrary,
    PawnExt, PrimitiveComponentExt, SceneComponentExt, SkeletalMeshComponentExt,
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
    #[component(attach = "root_component")]
    life_bar: WidgetComponent,

    #[uproperty(EditAnywhere, default = model::ENEMY_MAX_HP, name = "MaxHP")]
    max_hp: f32,

    #[uproperty(VisibleAnywhere, BlueprintReadOnly, default = model::ENEMY_MAX_HP, name = "CurrentHP")]
    current_hp: f32,

    #[uproperty(EditAnywhere)]
    pelvis_bone_name: FName,

    #[uproperty(EditAnywhere)]
    life_bar_widget: UObjectRef<CombatLifeBar>,

    #[uproperty(EditAnywhere, default = model::ENEMY_MELEE_TRACE_DISTANCE)]
    melee_trace_distance: f32,

    #[uproperty(EditAnywhere, default = model::ENEMY_MELEE_TRACE_RADIUS)]
    melee_trace_radius: f32,

    #[uproperty(EditAnywhere, default = model::ENEMY_MELEE_DAMAGE)]
    melee_damage: f32,

    #[uproperty(EditAnywhere, default = model::ENEMY_MELEE_KNOCKBACK_IMPULSE)]
    melee_knockback_impulse: f32,

    #[uproperty(EditAnywhere, default = model::ENEMY_MELEE_LAUNCH_IMPULSE)]
    melee_launch_impulse: f32,

    #[uproperty(EditAnywhere)]
    combo_attack_montage: UObjectRef<AnimMontage>,

    #[uproperty(EditAnywhere)]
    combo_section_names: UeArray<FName>,

    #[uproperty(EditAnywhere)]
    charged_attack_montage: UObjectRef<AnimMontage>,

    #[uproperty(EditAnywhere)]
    charge_loop_section: FName,

    #[uproperty(EditAnywhere)]
    charge_attack_section: FName,

    #[uproperty(EditAnywhere, default = model::MIN_CHARGE_LOOPS)]
    min_charge_loops: i32,

    #[uproperty(EditAnywhere, default = model::MAX_CHARGE_LOOPS)]
    max_charge_loops: i32,

    #[uproperty(EditAnywhere, default = model::DEATH_REMOVAL_TIME)]
    death_removal_time: f32,

    state: EnemyState,

    attack_end_bound: bool,

    attack_completed_task: UObjectRef<StateTreeTaskBlueprintBase>,

    landed_task: UObjectRef<StateTreeTaskBlueprintBase>,

    spawner: UObjectRef<Actor>,
}

#[uclass_impl]
impl CombatEnemy {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        let me = self.as_ref().checked()?;

        me.set_ai_controller_class(SubclassOf::<CombatAIController>::base().upcast_to());

        me.set_auto_possess_ai(EAutoPossessAI::PlacedInWorldOrSpawned);

        me.set_use_controller_rotation_yaw(false);

        me.get_capsule_component().checked()?.set_capsule_size(
            model::CAPSULE_RADIUS,
            model::CAPSULE_HALF_HEIGHT,
            Some(true),
        );

        me.get_character_movement()
            .checked()?
            .set_use_controller_desired_rotation(true);

        Ok(())
    }

    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        self.set_current_hp(self.max_hp());

        let filled = self
            .life_bar()
            .and_then(|life_bar| {
                life_bar
                    .checked()?
                    .get_user_widget_object()
                    .cast::<CombatLifeBar>()
            })
            .and_then(|widget| {
                self.set_life_bar_widget(widget);

                CombatLifeBar::from_obj(widget)
            });

        match filled {
            Ok(widget) => widget.set_life_percentage(1.0),
            Err(e) => ulog!(LOG_WARNING, "[Combat] enemy life bar: {e}"),
        }
    }

    #[ufunction(Override)]
    fn receive_end_play(&mut self, _end_play_reason: u8) {
        KismetSystemLibrary::k2_clear_timer(self.as_ref().upcast_to(), "RemoveFromLevel");
    }

    #[ufunction(Override)]
    fn on_landed(&mut self, _hit: UStructRef<FHitResult>) {
        if self.current_hp() >= 0.0
            && let Ok(mesh) = self
                .as_ref()
                .checked()
                .and_then(|me| me.get_mesh().checked())
        {
            mesh.set_physics_blend_weight(0.0);
        }

        finish(self.landed_task());
    }

    #[ufunction(BlueprintCallable)]
    fn check_combo(&mut self) {
        let mut state = self.state();

        state.current_combo_attack += 1;
        self.set_state(state);

        if state.current_combo_attack >= state.target_combo_count {
            return;
        }

        let Ok(sections) = self.combo_section_names().to_vec() else {
            return;
        };

        if let (Some(section), Ok(anim_instance)) = (
            sections.get(state.current_combo_attack as usize),
            self.anim_instance().and_then(|a| a.checked()),
        ) {
            anim_instance
                .montage_jump_to_section(section.handle(), Some(self.combo_attack_montage()));
        }
    }

    #[ufunction(BlueprintCallable)]
    fn check_charged_attack(&mut self) {
        let mut state = self.state();

        state.current_charge_loop += 1;
        self.set_state(state);

        let section = if state.current_charge_loop >= state.target_charge_loops {
            self.charge_attack_section()
        } else {
            self.charge_loop_section()
        };

        if let Ok(anim_instance) = self.anim_instance().and_then(|a| a.checked()) {
            anim_instance
                .montage_jump_to_section(section.handle(), Some(self.charged_attack_montage()));
        }
    }

    #[ufunction(BlueprintCallable)]
    fn remove_from_level(&mut self) {
        if let Ok(me) = self.as_ref().checked() {
            me.k2_destroy_actor();
        }
    }

    #[ufunction(BlueprintImplementableEvent)]
    fn received_damage(
        &self,
        damage: f32,
        impact_point: &OwnedStruct<FVector>,
        damage_direction: &OwnedStruct<FVector>,
    ) {
    }
}

fn finish(task: UObjectRef<StateTreeTaskBlueprintBase>) {
    if let Ok(task) = task.checked() {
        task.finish_task(Some(true));
    }
}

impl CombatEnemy {
    pub fn do_ai_combo_attack(&mut self) {
        let mut state = self.state();

        if state.is_attacking {
            return;
        }

        state.is_attacking = true;

        let sections = self.combo_section_names().len().unwrap_or(0) as i32;
        state.target_combo_count = KismetMathLibrary::random_integer_in_range(1, sections - 1);

        state.current_combo_attack = 0;
        self.set_state(state);

        self.play_attack_montage(self.combo_attack_montage());
    }

    pub fn do_ai_charged_attack(&mut self) {
        let mut state = self.state();

        if state.is_attacking {
            return;
        }

        state.is_attacking = true;

        state.target_charge_loops = KismetMathLibrary::random_integer_in_range(
            self.min_charge_loops(),
            self.max_charge_loops(),
        );

        state.current_charge_loop = 0;
        self.set_state(state);

        self.play_attack_montage(self.charged_attack_montage());
    }

    pub fn set_attack_completed_listener(&mut self, task: UObjectRef<StateTreeTaskBlueprintBase>) {
        self.set_attack_completed_task(task);
    }

    pub fn set_landed_listener(&mut self, task: UObjectRef<StateTreeTaskBlueprintBase>) {
        self.set_landed_task(task);
    }

    pub fn set_death_listener(&mut self, spawner: UObjectRef<Actor>) {
        self.set_spawner(spawner);
    }

    pub fn last_danger_location(&self) -> DVec3 {
        self.state().last_danger_location
    }

    pub fn last_danger_time(&self) -> f64 {
        self.state().last_danger_time
    }

    fn play_attack_montage(&mut self, montage: UObjectRef<AnimMontage>) {
        let Ok(anim_instance) = self.anim_instance() else {
            return;
        };

        if let Err(e) = self.bind_attack_end(anim_instance) {
            ulog!(
                LOG_WARNING,
                "[Combat] cannot watch the enemy's attack montage: {e}"
            );
        }

        if let Ok(anim_instance) = anim_instance.checked() {
            anim_instance.montage_play(montage, Some(1.0), None, Some(0.0), Some(true));
        }
    }

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
                    && (montage == me.combo_attack_montage()
                        || montage == me.charged_attack_montage())
                {
                    me.attack_montage_ended();
                }
            })?
            .detach();

        self.set_attack_end_bound(true);

        Ok(())
    }

    fn attack_montage_ended(&mut self) {
        let mut state = self.state();
        state.is_attacking = false;
        self.set_state(state);

        finish(self.attack_completed_task());
    }

    fn take_damage(&mut self, damage: f32) -> RustealResult<f32> {
        let Some((hp, dead)) = model::take_damage(self.current_hp(), damage) else {
            return Ok(0.0);
        };

        self.set_current_hp(hp);

        if dead {
            self.handle_death();
        } else {
            CombatLifeBar::from_obj(self.life_bar_widget())?
                .set_life_percentage(hp / self.max_hp());

            let mesh = self.as_ref().checked()?.get_mesh().checked()?;
            mesh.set_physics_blend_weight(0.5);
            mesh.set_body_simulate_physics(self.pelvis_bone_name().handle(), false);
        }

        Ok(damage)
    }

    fn anim_instance(&self) -> RustealResult<UObjectRef<AnimInstance>> {
        Ok(self
            .as_ref()
            .checked()?
            .get_mesh()
            .checked()?
            .get_anim_instance())
    }
}

impl CombatAttacker for CombatEnemy {
    fn do_attack_trace(&mut self, damage_source_bone: &str) {
        let Ok(me) = self.as_ref().checked() else {
            return;
        };

        let Ok(mesh) = me.get_mesh().checked() else {
            return;
        };

        let start = mesh
            .get_socket_location(FName::new(damage_source_bone).handle())
            .to_dvec3();

        let end = start
            + me.get_actor_forward_vector().to_dvec3() * f64::from(self.melee_trace_distance());

        let object_types: Vec<_> = object_type_query(ECollisionChannel::ECC_Pawn)
            .into_iter()
            .collect();

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

            if !actor.checked().is_ok_and(|a| a.actor_has_tag(player_tag)) {
                continue;
            }

            if let Some(mut damageable) = interfaces::damageable(actor) {
                let impulse = model::knockback(
                    hit.get_impact_normal().to_dvec3(),
                    self.melee_knockback_impulse(),
                    self.melee_launch_impulse(),
                );

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
    fn apply_damage(
        &mut self,
        damage: f32,
        _damage_causer: UObjectRef<Actor>,
        damage_location: DVec3,
        damage_impulse: DVec3,
    ) {
        let Ok(actual_damage) = self.take_damage(damage) else {
            return;
        };

        if actual_damage <= 0.0 {
            return;
        }

        let Ok(me) = self.as_ref().checked() else {
            return;
        };

        if let Ok(movement) = me.get_character_movement().checked() {
            movement.add_impulse(&FVector::from_dvec3(damage_impulse), Some(true));
        }

        if let Ok(mesh) = me.get_mesh().checked() {
            if mesh.is_simulating_physics(None) {
                let impulse = damage_impulse * f64::from(mesh.get_mass());

                mesh.add_impulse_at_location(
                    &FVector::from_dvec3(impulse),
                    &FVector::from_dvec3(damage_location),
                    None,
                );
            }

            if let Ok(anim_instance) = mesh.get_anim_instance().checked() {
                anim_instance.montage_stop(0.1, Some(self.combo_attack_montage()));
                anim_instance.montage_stop(0.1, Some(self.charged_attack_montage()));
            }
        }

        self.received_damage(
            actual_damage,
            &FVector::from_dvec3(damage_location),
            &FVector::from_dvec3(model::safe_normal(damage_impulse)),
        );
    }

    fn handle_death(&mut self) {
        let Ok(me) = self.as_ref().checked() else {
            return;
        };

        if let Ok(life_bar) = self.life_bar().and_then(|l| l.checked()) {
            life_bar.set_hidden_in_game(true, None);
        }

        if let Ok(capsule) = me.get_capsule_component().checked() {
            capsule.set_collision_enabled(ECollisionEnabled::NoCollision);
        }

        if let Ok(movement) = me.get_character_movement().checked() {
            movement.disable_movement();
        }

        if let Ok(mesh) = me.get_mesh().checked() {
            mesh.set_simulate_physics(true);
        }

        if let Ok(mut spawner) = CombatEnemySpawner::from_obj(self.spawner()) {
            spawner.on_enemy_died();
        }

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

    fn apply_healing(&mut self, _healing: f32, _healer: UObjectRef<Actor>) {}

    fn notify_danger(&mut self, danger_location: DVec3, danger_source: UObjectRef<Actor>) {
        let player_tag = FName::new(model::PLAYER_TAG).handle();

        if !danger_source
            .checked()
            .is_ok_and(|source| source.actor_has_tag(player_tag))
        {
            return;
        }

        let mut state = self.state();
        state.last_danger_location = danger_location;
        state.last_danger_time = GameplayStatics::get_time_seconds(self.as_ref().upcast_to());
        self.set_state(state);
    }
}
