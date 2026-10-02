// TwinStickAoEAttack: the TwinStick variant's `ATwinStickAoEAttack` in Rust,
// a short-lived area attack that destroys the NPCs inside it, or entering it,
// while it is active. Its Blueprint child `BP_TwinStickAoEAttack` fades it
// out and destroys it (`BP_AoEFinished`).

use bindings::engine::{
    Actor, ECollisionChannel, ECollisionEnabled, ECollisionResponse,
    KismetSystemLibrary, PrimitiveComponentExt, SceneComponent, SphereComponent,
    SphereComponentExt, StaticMeshComponent,
};
use rusteal_runtime::runtime::{FName, LOG_WARNING, RustealResult, SubclassOf, UObjectRef, ulog};
use rusteal_runtime::{uclass, uclass_impl};

use super::npc::TwinStickNPC;

/// `CollisionSphere->SetSphereRadius(750.0f)`.
const COLLISION_RADIUS: f32 = 750.0;

#[uclass(parent = Actor)]
pub struct TwinStickAoEAttack {
    #[component(root, name = "Root")]
    root: SceneComponent,

    /// Provides the visual representation for the AoE attack
    #[component(attach = "root", name = "Sphere Visual")]
    sphere_visual: StaticMeshComponent,

    /// Provides the collision volume for the AoE attack
    #[component(attach = "root", name = "Collision Sphere")]
    collision_sphere: SphereComponent,

    /// Time to wait between AoE damage ticks
    #[uproperty(EditAnywhere, name = "StartAoETime", category = "AoE Attack", default = 0.033)]
    start_ao_e_time: f32,

    /// Time to wait before stopping AoE damage checks
    #[uproperty(EditAnywhere, name = "StopAoETime", category = "AoE Attack", default = 0.5)]
    stop_ao_e_time: f32,

    /// While true, the AoE will damage anything that overlaps it
    is_ao_e_active: bool,
}

#[uclass_impl]
impl TwinStickAoEAttack {
    /// Everything `ATwinStickAoEAttack::ATwinStickAoEAttack()` sets.
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        // create the mesh that provides the visual representation for the AoE
        self.sphere_visual()?
            .checked()?
            .set_collision_profile_name(FName::new("NoCollision").handle(), Some(true));

        // create the collision sphere
        let sphere = self.collision_sphere()?.checked()?;
        sphere.set_sphere_radius(COLLISION_RADIUS, Some(true));
        sphere.set_notify_rigid_body_collision(true);
        sphere.set_collision_enabled(ECollisionEnabled::QueryOnly);
        sphere.set_collision_object_type(ECollisionChannel::ECC_WorldDynamic);
        sphere.set_collision_response_to_all_channels(ECollisionResponse::ECR_Ignore);
        sphere.set_collision_response_to_channel(ECollisionChannel::ECC_Pawn, ECollisionResponse::ECR_Overlap);
        Ok(())
    }

    /// Initialization
    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        // handle collision with the AoE while it's active
        if let Err(e) = self.bind_overlap() {
            ulog!(LOG_WARNING, "[TwinStick] AoE overlap: {e}");
        }

        // set up the AoE timers
        let me = self.as_ref().upcast_to();
        KismetSystemLibrary::k2_set_timer(me, "StartAoE", self.start_ao_e_time(), false, None, None, None);
        KismetSystemLibrary::k2_set_timer(me, "StopAoE", self.stop_ao_e_time(), false, None, None, None);
    }

    /// Cleanup
    #[ufunction(Override)]
    fn receive_end_play(&mut self, _end_play_reason: u8) {
        // clear the timers
        let me = self.as_ref().upcast_to();
        KismetSystemLibrary::k2_clear_timer(me, "StartAoE");
        KismetSystemLibrary::k2_clear_timer(me, "StopAoE");
    }

    /// Called when the start AoE timer triggers
    #[ufunction(name = "StartAoE")]
    fn start_ao_e(&mut self) {
        // raise the active flag
        self.set_is_ao_e_active(true);

        // find all actors overlapping the NPC and tell each it's been hit
        let Ok(sphere) = self.collision_sphere().and_then(|s| s.checked()) else {
            return;
        };
        for current in sphere.get_overlapping_actors(Some(SubclassOf::<TwinStickNPC>::base().upcast_to())) {
            if let Ok(npc) = TwinStickNPC::from_obj(current) {
                npc.projectile_impact();
            }
        }
    }

    /// Called when the stop AoE timer triggers
    #[ufunction(name = "StopAoE")]
    fn stop_ao_e(&mut self) {
        // drop the active flag
        self.set_is_ao_e_active(false);

        // stop the damage tick timer
        KismetSystemLibrary::k2_clear_timer(self.as_ref().upcast_to(), "StartAoE");

        // call the BP handler. It will be responsible for destroying the Actor when it's done
        self.bp_ao_e_finished();
    }

    /// Allows Blueprint handling of AoE fade out effects. NOTE: Call Destroy Actor at the end of this!
    #[ufunction(BlueprintImplementableEvent, name = "BP_AoEFinished")]
    fn bp_ao_e_finished(&self) {}
}

impl TwinStickAoEAttack {
    /// `CollisionSphere->OnComponentBeginOverlap.AddDynamic(this, &OnAoEOverlap)`
    fn bind_overlap(&self) -> RustealResult<()> {
        let me: UObjectRef<Actor> = self.as_ref();
        self.collision_sphere()?
            .checked()?
            .on_component_begin_overlap()
            .add(move |_overlapped, other_actor, _other_comp, _body_index, _from_sweep, _sweep| {
                if let Ok(aoe) = TwinStickAoEAttack::from_obj(me) {
                    aoe.on_ao_e_overlap(other_actor);
                }
            })?
            .detach();
        Ok(())
    }

    /// Handles collision with the AoE while it's active
    fn on_ao_e_overlap(&self, other_actor: UObjectRef<Actor>) {
        // is the explosion active? did we overlap an NPC? tell the NPC it's been hit
        if self.is_ao_e_active()
            && let Ok(npc) = TwinStickNPC::from_obj(other_actor)
        {
            npc.projectile_impact();
        }
    }
}
