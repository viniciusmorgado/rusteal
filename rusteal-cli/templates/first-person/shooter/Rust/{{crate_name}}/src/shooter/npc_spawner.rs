use bindings::engine::{
    Actor, ArrowComponent, CapsuleComponent, CapsuleComponentExt,
    ESpawnActorCollisionHandlingMethod, GameplayStatics, KismetSystemLibrary,
    PrimitiveComponentExt, SceneComponent, SceneComponentExt,
};
use bindings::prelude::*;
use rusteal_runtime::runtime::{FName, LOG_WARNING, RustealResult, SubclassOf, ulog};
use rusteal_runtime::{uclass, uclass_impl};

use super::game_mode::ShooterGameMode;
use super::npc::ShooterNPC;

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

    #[uproperty(
        EditAnywhere,
        BlueprintReadOnly,
        name = "NPCClass",
        category = "NPC Spawner"
    )]
    npc_class: SubclassOf<ShooterNPC>,

    #[uproperty(
        EditAnywhere,
        BlueprintReadOnly,
        category = "NPC Spawner",
        default = 5.0
    )]
    initial_spawn_delay: f32,

    #[uproperty(EditAnywhere, BlueprintReadOnly, category = "NPC Spawner", default = 1)]
    spawn_count: i32,

    #[uproperty(
        EditAnywhere,
        BlueprintReadOnly,
        category = "NPC Spawner",
        default = 5.0
    )]
    respawn_delay: f32,
}

#[uclass_impl]
impl ShooterNPCSpawner {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
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

    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        let world = self.as_ref().upcast_to();

        if let Ok(game_mode) = ShooterGameMode::from_obj(GameplayStatics::get_game_mode(world))
            && !game_mode.should_spawn_enemy_npcs()
        {
            return;
        }

        if self.spawn_count() > 0 {
            self.schedule_spawn(self.initial_spawn_delay());
        }
    }

    #[ufunction(Override)]
    fn receive_end_play(&mut self, _end_play_reason: u8) {
        KismetSystemLibrary::k2_clear_timer(self.as_ref().upcast_to(), "SpawnNPC");
    }

    #[ufunction(name = "SpawnNPC")]
    fn spawn_npc(&mut self) {
        if let Err(e) = self.spawn() {
            ulog!(LOG_WARNING, "[Shooter] NPC spawn failed: {e}");
        }
    }
}

impl ShooterNPCSpawner {
    pub fn on_npc_died(&mut self) {
        self.set_spawn_count(self.spawn_count() - 1);

        if self.spawn_count() <= 0 {
            return;
        }

        self.schedule_spawn(self.respawn_delay());
    }

    fn schedule_spawn(&self, delay: f32) {
        KismetSystemLibrary::k2_set_timer(
            self.as_ref().upcast_to(),
            "SpawnNPC",
            delay,
            false,
            None,
            None,
            None,
        );
    }

    fn spawn(&mut self) -> RustealResult<()> {
        let npc_class = self.npc_class();

        if npc_class.is_null() {
            return Ok(());
        }

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

        if let Ok(spawned_npc) = ShooterNPC::from_obj(spawned) {
            spawned_npc.set_spawner(self.as_ref().cast()?);
        }

        Ok(())
    }
}
