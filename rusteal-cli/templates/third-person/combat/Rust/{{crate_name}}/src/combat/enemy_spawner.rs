// CombatEnemySpawner: the Combat variant's `ACombatEnemySpawner` in Rust.
// Spawns enemies one at a time, the next after the last one dies; it can wait
// for an activation volume, and activates other actors once depleted.

use bindings::engine::{
    Actor, ArrowComponent, CapsuleComponent, CapsuleComponentExt, ESpawnActorCollisionHandlingMethod,
    GameplayStatics, KismetSystemLibrary, PrimitiveComponentExt, SceneComponent, SceneComponentExt,
};
use bindings::prelude::*;
use rusteal_runtime::runtime::{FName, LOG_WARNING, RustealResult, SubclassOf, UObjectRef, UeArray, ulog};
use rusteal_runtime::{uclass, uclass_impl};

use super::enemy::CombatEnemy;
use super::interfaces::{self, CombatActivatable};
use super::model;

#[uclass(parent = Actor)]
pub struct CombatEnemySpawner {
    #[component(root)]
    root: SceneComponent,

    /// Enemy spawn location component
    #[component(attach = "root", name = "Spawn Capsule")]
    spawn_capsule: CapsuleComponent,

    /// Spawn direction indicator
    #[component(attach = "root", name = "Spawn Direction")]
    spawn_direction: ArrowComponent,

    /// Type of enemy to spawn
    #[uproperty(EditAnywhere, BlueprintReadOnly)]
    enemy_class: SubclassOf<CombatEnemy>,

    /// If true, the first enemy will be spawned as soon as the game starts
    #[uproperty(EditAnywhere, BlueprintReadOnly, default = true)]
    b_should_spawn_enemies_immediately: bool,

    /// Time to wait before spawning the first enemy on game start
    #[uproperty(EditAnywhere, BlueprintReadOnly, default = model::INITIAL_SPAWN_DELAY)]
    initial_spawn_delay: f32,

    /// Number of enemies to spawn
    #[uproperty(EditAnywhere, BlueprintReadOnly, default = model::SPAWN_COUNT)]
    spawn_count: i32,

    /// Time to wait before spawning the next enemy after the current one dies
    #[uproperty(EditAnywhere, BlueprintReadOnly, default = model::RESPAWN_DELAY)]
    respawn_delay: f32,

    /// Time to wait after this spawner is depleted before activating the actor list
    #[uproperty(EditAnywhere, BlueprintReadOnly, default = model::ACTIVATION_DELAY)]
    activation_delay: f32,

    /// List of actors to activate after the last enemy dies
    #[uproperty(EditAnywhere, BlueprintReadOnly)]
    actors_to_activate_when_depleted: UeArray<UObjectRef<Actor>>,

    /// Flag to ensure this is only activated once
    has_been_activated: bool,
}

#[uclass_impl]
impl CombatEnemySpawner {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        let capsule = self.spawn_capsule()?.checked()?;
        capsule.k2_set_relative_location(&FVector::from_dvec3(model::SPAWN_CAPSULE_LOCATION), false, false);
        capsule.set_capsule_size(model::CAPSULE_RADIUS, model::CAPSULE_HALF_HEIGHT, Some(true));
        capsule.set_collision_profile_name(FName::new("NoCollision").handle(), Some(true));
        Ok(())
    }

    /// Initialization
    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        if self.b_should_spawn_enemies_immediately() {
            self.set_timer("SpawnEnemy", self.initial_spawn_delay());
        }
    }

    /// Cleanup
    #[ufunction(Override)]
    fn receive_end_play(&mut self, _end_play_reason: u8) {
        // the spawn timer runs one function at a time
        for function in ["SpawnEnemy", "SpawnerDepleted"] {
            KismetSystemLibrary::k2_clear_timer(self.as_ref().upcast_to(), function);
        }
    }

    /// Spawn an enemy and subscribe to its death event
    #[ufunction(BlueprintCallable)]
    fn spawn_enemy(&mut self) {
        if let Err(e) = self.spawn() {
            ulog!(LOG_WARNING, "[Combat] cannot spawn an enemy: {e}");
        }
    }

    /// Called after the last spawned enemy has died
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

    /// Called when the spawned enemy has died
    pub fn on_enemy_died(&mut self) {
        let spawn_count = self.spawn_count() - 1;
        self.set_spawn_count(spawn_count);
        if spawn_count <= 0 {
            self.set_timer("SpawnerDepleted", self.activation_delay());
            return;
        }
        self.set_timer("SpawnEnemy", self.respawn_delay());
    }

    fn set_timer(&self, function: &str, time: f32) {
        // one spawn timer: the next function replaces the pending one
        for pending in ["SpawnEnemy", "SpawnerDepleted"] {
            KismetSystemLibrary::k2_clear_timer(self.as_ref().upcast_to(), pending);
        }
        KismetSystemLibrary::k2_set_timer(self.as_ref().upcast_to(), function, time, false, None, None, None);
    }
}

impl CombatActivatable for CombatEnemySpawner {
    /// Toggles the Spawner
    fn toggle_interaction(&mut self, _activation_instigator: UObjectRef<Actor>) {}

    /// Activates the Spawner
    fn activate_interaction(&mut self, _activation_instigator: UObjectRef<Actor>) {
        if self.has_been_activated() || self.b_should_spawn_enemies_immediately() {
            return;
        }
        self.set_has_been_activated(true);
        self.spawn_enemy();
    }

    /// Deactivates the Spawner
    fn deactivate_interaction(&mut self, _activation_instigator: UObjectRef<Actor>) {}
}
