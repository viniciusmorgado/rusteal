use std::time::Duration;

use bindings::engine::{Actor, ActorExt, GameplayStatics, KismetMathLibrary};
use bindings::navigation::{NavigationSystemV1, RecastNavMesh};
use bindings::prelude::*;
use rusteal_runtime::runtime::timer::{self, TimerHandle};
use rusteal_runtime::runtime::{Rotator, SubclassOf, UObjectRef, method};
use rusteal_runtime::{uclass, uclass_impl};

use super::game_mode::TwinStickGameMode;
use super::npc::TwinStickNPC;

#[uclass(parent = Actor)]
pub struct TwinStickSpawner {
    #[uproperty(EditAnywhere, name = "NPCClass", category = "NPC Spawner")]
    npc_class: SubclassOf<TwinStickNPC>,

    #[uproperty(EditAnywhere, category = "NPC Spawner", default = 5.0)]
    spawn_group_delay: f32,

    #[uproperty(EditAnywhere, category = "NPC Spawner", default = 0.33)]
    min_spawn_delay: f32,

    #[uproperty(EditAnywhere, category = "NPC Spawner", default = 0.66)]
    max_spawn_delay: f32,

    #[uproperty(EditAnywhere, category = "NPC Spawner", default = 600.0)]
    spawn_radius: f32,

    #[uproperty(EditAnywhere, category = "NPC Spawner", default = 3)]
    spawn_group_size: i32,

    #[uproperty]
    nav_data: UObjectRef<RecastNavMesh>,

    spawn_count: i32,

    spawn_group_timer: Option<TimerHandle>,

    spawn_npc_timer: Option<TimerHandle>,
}

#[uclass_impl]
impl TwinStickSpawner {
    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        let me = self.as_ref().upcast_to();

        let nav_meshes = GameplayStatics::get_all_actors_of_class(
            me,
            SubclassOf::<RecastNavMesh>::base().upcast_to(),
        );

        match nav_meshes.first() {
            Some(nav_mesh) => self.set_nav_data(nav_mesh.cast().unwrap_or_default()),
            None => log::info!("Could not find recast navmesh"),
        }

        let group = timer::set_timer(
            &self.as_ref(),
            Duration::from_secs_f32(self.spawn_group_delay()),
            true,
            method(self, Self::spawn_npc_group),
        );

        self.set_spawn_group_timer(group.ok());

        self.spawn_npc_group();
    }

    #[ufunction(Override)]
    fn receive_end_play(&mut self, _end_play_reason: u8) {
        if let Some(timer) = self.spawn_group_timer() {
            timer.clear();
        }

        if let Some(timer) = self.spawn_npc_timer() {
            timer.clear();
        }
    }

    #[ufunction(name = "SpawnNPCGroup")]
    fn spawn_npc_group(&mut self) {
        self.set_spawn_count(0);

        let world = self.as_ref().upcast_to();

        if let Ok(game_mode) = TwinStickGameMode::from_obj(GameplayStatics::get_game_mode(world))
            && game_mode.can_spawn_npcs()
        {
            self.spawn_npc();
        }
    }

    #[ufunction(name = "SpawnNPC")]
    fn spawn_npc(&mut self) {
        let Ok(me) = self.as_ref().checked() else {
            return;
        };

        let (found, spawn_location) = NavigationSystemV1::k2_get_random_reachable_point_in_radius(
            me.as_ref().upcast_to(),
            &me.k2_get_actor_location(),
            self.spawn_radius(),
            Some(self.nav_data().upcast_to()),
            None,
        );

        if found {
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

        self.set_spawn_count(self.spawn_count() + 1);

        if self.spawn_count() < self.spawn_group_size() {
            let delay = KismetMathLibrary::random_float_in_range(
                f64::from(self.min_spawn_delay()),
                f64::from(self.max_spawn_delay()),
            );

            if let Some(timer) = self.spawn_npc_timer() {
                timer.clear();
            }

            let spawn = timer::set_timer(
                &self.as_ref(),
                Duration::from_secs_f32(delay as f32),
                false,
                method(self, Self::spawn_npc),
            );

            self.set_spawn_npc_timer(spawn.ok());
        }
    }
}
