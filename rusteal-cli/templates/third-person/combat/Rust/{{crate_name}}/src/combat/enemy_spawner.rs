use std::time::Duration;

use bindings::engine::{
    Actor, ArrowComponent, CapsuleComponent, CapsuleComponentExt,
    ESpawnActorCollisionHandlingMethod, GameplayStatics, PrimitiveComponentExt, SceneComponent,
    SceneComponentExt,
};
use bindings::prelude::*;
use rusteal_runtime::runtime::timer::{self, TimerHandle};
use rusteal_runtime::runtime::{FName, RustealResult, SubclassOf, UObjectRef, UeArray, method};
use rusteal_runtime::{uclass, uclass_impl};

use super::enemy::CombatEnemy;
use super::interfaces::{self, CombatActivatable};
use super::model;

#[uclass(parent = Actor)]
pub struct CombatEnemySpawner {
    #[component(root)]
    root: SceneComponent,

    #[component(attach = "root", name = "Spawn Capsule")]
    spawn_capsule: CapsuleComponent,

    #[component(attach = "root", name = "Spawn Direction")]
    spawn_direction: ArrowComponent,

    #[uproperty(EditAnywhere, BlueprintReadOnly)]
    enemy_class: SubclassOf<CombatEnemy>,

    #[uproperty(EditAnywhere, BlueprintReadOnly, default = true)]
    b_should_spawn_enemies_immediately: bool,

    #[uproperty(EditAnywhere, BlueprintReadOnly, default = model::INITIAL_SPAWN_DELAY)]
    initial_spawn_delay: f32,

    #[uproperty(EditAnywhere, BlueprintReadOnly, default = model::SPAWN_COUNT)]
    spawn_count: i32,

    #[uproperty(EditAnywhere, BlueprintReadOnly, default = model::RESPAWN_DELAY)]
    respawn_delay: f32,

    #[uproperty(EditAnywhere, BlueprintReadOnly, default = model::ACTIVATION_DELAY)]
    activation_delay: f32,

    #[uproperty(EditAnywhere, BlueprintReadOnly)]
    actors_to_activate_when_depleted: UeArray<UObjectRef<Actor>>,

    has_been_activated: bool,

    pending_timer: Option<TimerHandle>,
}

#[uclass_impl]
impl CombatEnemySpawner {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        let capsule = self.spawn_capsule()?.checked()?;

        capsule.k2_set_relative_location(
            &FVector::from_dvec3(model::SPAWN_CAPSULE_LOCATION),
            false,
            false,
        );

        capsule.set_capsule_size(
            model::CAPSULE_RADIUS,
            model::CAPSULE_HALF_HEIGHT,
            Some(true),
        );

        capsule.set_collision_profile_name(FName::new("NoCollision").handle(), Some(true));

        Ok(())
    }

    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        if self.b_should_spawn_enemies_immediately() {
            self.set_timer(self.initial_spawn_delay(), method(self, Self::spawn_enemy));
        }
    }

    #[ufunction(Override)]
    fn receive_end_play(&mut self, _end_play_reason: u8) {
        if let Some(pending_timer) = self.pending_timer() {
            pending_timer.clear();
        }
    }

    #[ufunction(BlueprintCallable)]
    fn spawn_enemy(&mut self) {
        if let Err(e) = self.spawn() {
            log::warn!("[Combat] cannot spawn an enemy: {e}");
        }
    }

    #[ufunction(BlueprintCallable)]
    fn spawner_depleted(&mut self) {
        let Ok(actors) = self.actors_to_activate_when_depleted().to_vec() else {
            return;
        };

        let me = self.as_ref().upcast_to();

        for current_actor in actors {
            if let Some(mut combat_activatable) = interfaces::activatable(current_actor) {
                combat_activatable.activate_interaction(me);
            }
        }
    }
}

impl CombatEnemySpawner {
    fn spawn(&mut self) -> RustealResult<()> {
        let enemy_class = self.enemy_class();

        if enemy_class.is_null() {
            return Ok(());
        }

        let world = self.as_ref().upcast_to();
        let transform = self.spawn_capsule()?.checked()?.k2_get_component_to_world();

        let spawned = GameplayStatics::begin_deferred_actor_spawn_from_class(
            world,
            enemy_class.upcast_to(),
            &transform,
            Some(ESpawnActorCollisionHandlingMethod::AdjustIfPossibleButAlwaysSpawn),
            None,
            None,
        );

        let spawned = GameplayStatics::finish_spawning_actor(spawned, &transform, None);

        if let Ok(mut spawned_enemy) = CombatEnemy::from_obj(spawned) {
            spawned_enemy.set_death_listener(self.as_ref().upcast_to());
        }

        Ok(())
    }

    pub fn on_enemy_died(&mut self) {
        let spawn_count = self.spawn_count() - 1;
        self.set_spawn_count(spawn_count);

        if spawn_count <= 0 {
            self.set_timer(
                self.activation_delay(),
                method(self, Self::spawner_depleted),
            );
            return;
        }

        self.set_timer(self.respawn_delay(), method(self, Self::spawn_enemy));
    }

    fn set_timer(&mut self, time: f32, callback: impl FnMut() + Send + 'static) {
        if let Some(pending_timer) = self.pending_timer() {
            pending_timer.clear();
        }

        let pending_timer = timer::set_timer(
            &self.as_ref(),
            Duration::from_secs_f32(time),
            false,
            callback,
        )
        .inspect_err(|e| log::warn!("[Combat] spawner timer: {e}"))
        .ok();

        self.set_pending_timer(pending_timer);
    }
}

impl CombatActivatable for CombatEnemySpawner {
    fn toggle_interaction(&mut self, _activation_instigator: UObjectRef<Actor>) {}

    fn activate_interaction(&mut self, _activation_instigator: UObjectRef<Actor>) {
        if self.has_been_activated() || self.b_should_spawn_enemies_immediately() {
            return;
        }

        self.set_has_been_activated(true);
        self.spawn_enemy();
    }

    fn deactivate_interaction(&mut self, _activation_instigator: UObjectRef<Actor>) {}
}
