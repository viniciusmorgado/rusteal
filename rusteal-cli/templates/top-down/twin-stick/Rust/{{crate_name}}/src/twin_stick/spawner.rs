// TwinStickSpawner: the TwinStick variant's `ATwinStickSpawner` in Rust, an
// actor that spawns groups of NPCs at random reachable points around itself,
// one by one, while the game mode allows more. Its Blueprint child
// `BP_TwinStickSpawner` sets the NPC class.

use bindings::engine::{Actor, ActorExt, GameplayStatics, KismetMathLibrary, KismetSystemLibrary};
use bindings::navigation::{NavigationSystemV1, RecastNavMesh};
use bindings::prelude::*;
use rusteal_runtime::runtime::{LOG_DISPLAY, Rotator, SubclassOf, UObjectRef, ulog};
use rusteal_runtime::{uclass, uclass_impl};

use super::game_mode::TwinStickGameMode;
use super::npc::TwinStickNPC;

#[uclass(parent = Actor)]
pub struct TwinStickSpawner {
    /// Type of NPC to spawn
    #[uproperty(EditAnywhere, name = "NPCClass", category = "NPC Spawner")]
    npc_class: SubclassOf<TwinStickNPC>,

    /// Time delay between enemy group spawns
    #[uproperty(EditAnywhere, category = "NPC Spawner", default = 5.0)]
    spawn_group_delay: f32,

    /// Min time delay between individual NPC spawns
    #[uproperty(EditAnywhere, category = "NPC Spawner", default = 0.33)]
    min_spawn_delay: f32,

    /// Max time delay between individual NPC spawns
    #[uproperty(EditAnywhere, category = "NPC Spawner", default = 0.66)]
    max_spawn_delay: f32,

    /// Radius around the spawner where it can spawn NPCs
    #[uproperty(EditAnywhere, category = "NPC Spawner", default = 600.0)]
    spawn_radius: f32,

    /// Number of NPCs to spawn per group
    #[uproperty(EditAnywhere, category = "NPC Spawner", default = 3)]
    spawn_group_size: i32,

    /// Pointer to the recast nav mesh actor, used to provide NPC spawn locations
    #[uproperty]
    nav_data: UObjectRef<RecastNavMesh>,

    /// Number of NPCs spawned in the current group
    spawn_count: i32,
}

#[uclass_impl]
impl TwinStickSpawner {
    /// Gameplay initialization
    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        let me = self.as_ref().upcast_to();

        // find the recast navmesh actor on the level
        let nav_meshes = GameplayStatics::get_all_actors_of_class(me, SubclassOf::<RecastNavMesh>::base().upcast_to());
        match nav_meshes.first() {
            Some(nav_mesh) => self.set_nav_data(nav_mesh.cast().unwrap_or_default()),
            None => ulog!(LOG_DISPLAY, "Could not find recast navmesh"),
        }

        // set up the spawn timer
        KismetSystemLibrary::k2_set_timer(me, "SpawnNPCGroup", self.spawn_group_delay(), true, None, None, None);

        // spawn the first group of NPCs
        self.spawn_npc_group();
    }

    /// Gameplay cleanup
    #[ufunction(Override)]
    fn receive_end_play(&mut self, _end_play_reason: u8) {
        // clear the spawn timers
        let me = self.as_ref().upcast_to();
        KismetSystemLibrary::k2_clear_timer(me, "SpawnNPCGroup");
        KismetSystemLibrary::k2_clear_timer(me, "SpawnNPC");
    }

    /// Spawns a new NPC group
    #[ufunction(name = "SpawnNPCGroup")]
    fn spawn_npc_group(&mut self) {
        // reset the group spawn counter
        self.set_spawn_count(0);

        // check if we're still under the max NPC cap
        let world = self.as_ref().upcast_to();
        if let Ok(game_mode) = TwinStickGameMode::from_obj(GameplayStatics::get_game_mode(world))
            && game_mode.can_spawn_npcs()
        {
            self.spawn_npc();
        }
    }

    /// Spawns an individual NPC
    #[ufunction(name = "SpawnNPC")]
    fn spawn_npc(&mut self) {
        let Ok(me) = self.as_ref().checked() else {
            return;
        };

        // find a random point around the spawner
        let (found, spawn_location) = NavigationSystemV1::k2_get_random_reachable_point_in_radius(
            me.as_ref().upcast_to(),
            &me.k2_get_actor_location(),
            self.spawn_radius(),
            Some(self.nav_data().upcast_to()),
            None,
        );
        if found {
            // spawn the NPC
            let transform = KismetMathLibrary::make_transform(
                &spawn_location,
                &FRotator::from_rotator(Rotator::ZERO),
                &FVector::from_dvec3(glam::DVec3::ONE),
            );
            let npc = GameplayStatics::begin_deferred_actor_spawn_from_class(
                me.as_ref().upcast_to(),
                self.npc_class().upcast_to(),
                &transform,
                None,
                None,
                None,
            );
            GameplayStatics::finish_spawning_actor(npc, &transform, None);
        }

        // increase the spawn counter
        self.set_spawn_count(self.spawn_count() + 1);

        // do we still have enemies left to spawn?
        if self.spawn_count() < self.spawn_group_size() {
            let delay = KismetMathLibrary::random_float_in_range(
                f64::from(self.min_spawn_delay()),
                f64::from(self.max_spawn_delay()),
            );
            KismetSystemLibrary::k2_set_timer(me.as_ref().upcast_to(), "SpawnNPC", delay as f32, false, None, None, None);
        }
    }
}
