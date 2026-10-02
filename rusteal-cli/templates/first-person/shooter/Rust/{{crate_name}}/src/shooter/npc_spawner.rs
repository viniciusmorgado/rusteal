// ShooterNPCSpawner: the Shooter variant's `AShooterNPCSpawner` in Rust, an
// actor that spawns Shooter NPCs one by one, waiting for each to die before
// spawning the next. Its Blueprint child `BP_ShooterNPCSpawner` sets the NPC
// class.

use bindings::engine::{
    Actor, ArrowComponent, CapsuleComponent, CapsuleComponentExt, ESpawnActorCollisionHandlingMethod,
    GameplayStatics, KismetSystemLibrary, PrimitiveComponentExt, SceneComponent, SceneComponentExt,
};
use bindings::prelude::*;
use rusteal_runtime::runtime::{FName, LOG_WARNING, RustealResult, SubclassOf, ulog};
use rusteal_runtime::{uclass, uclass_impl};

use super::game_mode::ShooterGameMode;
use super::npc::ShooterNPC;

/// The reference spawn capsule: `SetRelativeLocation(FVector(0.0f, 0.0f,
/// 90.0f))`, `SetCapsuleSize(35.0f, 90.0f)`.
const SPAWN_CAPSULE_HEIGHT: f64 = 90.0;
const SPAWN_CAPSULE_RADIUS: f32 = 35.0;
const SPAWN_CAPSULE_HALF_HEIGHT: f32 = 90.0;

#[uclass(parent = Actor)]
pub struct ShooterNPCSpawner {
    #[component(root, name = "Root")]
    root: SceneComponent,

    #[component(attach = "root", name = "Spawn Capsule")]
    spawn_capsule: CapsuleComponent,

    #[component(attach = "root", name = "Spawn Direction")]
    spawn_direction: ArrowComponent,

    /// Type of NPC to spawn
    #[uproperty(EditAnywhere, BlueprintReadOnly, name = "NPCClass", category = "NPC Spawner")]
    npc_class: SubclassOf<ShooterNPC>,

    /// Time to wait before spawning the first NPC on game start
    #[uproperty(EditAnywhere, BlueprintReadOnly, category = "NPC Spawner", default = 5.0)]
    initial_spawn_delay: f32,

    /// Number of NPCs to spawn
    #[uproperty(EditAnywhere, BlueprintReadOnly, category = "NPC Spawner", default = 1)]
    spawn_count: i32,

    /// Time to wait before spawning the next NPC after the current one dies
    #[uproperty(EditAnywhere, BlueprintReadOnly, category = "NPC Spawner", default = 5.0)]
    respawn_delay: f32,
}

#[uclass_impl]
impl ShooterNPCSpawner {
    /// Everything `AShooterNPCSpawner::AShooterNPCSpawner()` sets.
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        // create the reference spawn capsule
        let capsule = self.spawn_capsule()?.checked()?;
        capsule.k2_set_relative_location(
            &FVector::from_dvec3(glam::DVec3::new(0.0, 0.0, SPAWN_CAPSULE_HEIGHT)),
            false,
            false,
        );
        capsule.set_capsule_size(SPAWN_CAPSULE_RADIUS, SPAWN_CAPSULE_HALF_HEIGHT, Some(true));
        capsule.set_collision_profile_name(FName::new("NoCollision").handle(), Some(true));
        Ok(())
    }

    /// Initialization
    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        // ignore if enemies are disabled at the GameMode level
        let world = self.as_ref().upcast_to();
        if let Ok(game_mode) = ShooterGameMode::from_obj(GameplayStatics::get_game_mode(world))
            && !game_mode.should_spawn_enemy_npcs()
        {
            return;
        }

        // ensure we don't spawn NPCs if our initial spawn count is zero
        if self.spawn_count() > 0 {
            // schedule the first NPC spawn
            self.schedule_spawn(self.initial_spawn_delay());
        }
    }

    /// Cleanup
    #[ufunction(Override)]
    fn receive_end_play(&mut self, _end_play_reason: u8) {
        // clear the spawn timer
        KismetSystemLibrary::k2_clear_timer(self.as_ref().upcast_to(), "SpawnNPC");
    }

    /// Spawn an NPC and subscribe to its death event
    #[ufunction(name = "SpawnNPC")]
    fn spawn_npc(&mut self) {
        if let Err(e) = self.spawn() {
            ulog!(LOG_WARNING, "[Shooter] NPC spawn failed: {e}");
        }
    }
}

impl ShooterNPCSpawner {
    /// Called when the spawned NPC has died
    pub fn on_npc_died(&mut self) {
        // decrease the spawn counter
        self.set_spawn_count(self.spawn_count() - 1);

        // is this the last NPC we should spawn?
        if self.spawn_count() <= 0 {
            return;
        }

        // schedule the next NPC spawn
        self.schedule_spawn(self.respawn_delay());
    }

    fn schedule_spawn(&self, delay: f32) {
        KismetSystemLibrary::k2_set_timer(self.as_ref().upcast_to(), "SpawnNPC", delay, false, None, None, None);
    }

    fn spawn(&mut self) -> RustealResult<()> {
        // ensure the NPC class is valid
        let npc_class = self.npc_class();
        if npc_class.is_null() {
            return Ok(());
        }

        // spawn the NPC at the reference capsule's transform
        let transform = self.spawn_capsule()?.checked()?.k2_get_component_to_world();
        let spawned = GameplayStatics::begin_deferred_actor_spawn_from_class(
            self.as_ref().upcast_to(),
            npc_class.upcast_to(),
            &transform,
            Some(ESpawnActorCollisionHandlingMethod::AdjustIfPossibleButAlwaysSpawn),
            None,
            None,
        );
        let spawned = GameplayStatics::finish_spawning_actor(spawned, &transform, None);

        // subscribe to the death delegate
        if let Ok(spawned_npc) = ShooterNPC::from_obj(spawned) {
            spawned_npc.set_spawner(self.as_ref().cast()?);
        }
        Ok(())
    }
}
