use bindings::engine::{
    Actor, ECollisionChannel, ECollisionEnabled, ECollisionResponse, KismetSystemLibrary,
    PrimitiveComponentExt, SceneComponent, SphereComponent, SphereComponentExt,
    StaticMeshComponent,
};
use rusteal_runtime::runtime::{FName, LOG_WARNING, RustealResult, SubclassOf, UObjectRef, ulog};
use rusteal_runtime::{uclass, uclass_impl};

use super::npc::TwinStickNPC;

const COLLISION_RADIUS: f32 = 750.0;

#[uclass(parent = Actor)]
pub struct TwinStickAoEAttack {
    #[component(root, name = "Root")]
    root: SceneComponent,

    #[component(attach = "root", name = "Sphere Visual")]
    sphere_visual: StaticMeshComponent,

    #[component(attach = "root", name = "Collision Sphere")]
    collision_sphere: SphereComponent,

    #[uproperty(
        EditAnywhere,
        name = "StartAoETime",
        category = "AoE Attack",
        default = 0.033
    )]
    start_ao_e_time: f32,

    #[uproperty(
        EditAnywhere,
        name = "StopAoETime",
        category = "AoE Attack",
        default = 0.5
    )]
    stop_ao_e_time: f32,

    is_ao_e_active: bool,
}

#[uclass_impl]
impl TwinStickAoEAttack {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        self.sphere_visual()?
            .checked()?
            .set_collision_profile_name(FName::new("NoCollision").handle(), Some(true));

        let sphere = self.collision_sphere()?.checked()?;
        sphere.set_sphere_radius(COLLISION_RADIUS, Some(true));
        sphere.set_notify_rigid_body_collision(true);
        sphere.set_collision_enabled(ECollisionEnabled::QueryOnly);
        sphere.set_collision_object_type(ECollisionChannel::ECC_WorldDynamic);
        sphere.set_collision_response_to_all_channels(ECollisionResponse::ECR_Ignore);

        sphere.set_collision_response_to_channel(
            ECollisionChannel::ECC_Pawn,
            ECollisionResponse::ECR_Overlap,
        );

        Ok(())
    }

    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        if let Err(e) = self.bind_overlap() {
            ulog!(LOG_WARNING, "[TwinStick] AoE overlap: {e}");
        }

        let me = self.as_ref().upcast_to();

        KismetSystemLibrary::k2_set_timer(
            me,
            "StartAoE",
            self.start_ao_e_time(),
            false,
            None,
            None,
            None,
        );

        KismetSystemLibrary::k2_set_timer(
            me,
            "StopAoE",
            self.stop_ao_e_time(),
            false,
            None,
            None,
            None,
        );
    }

    #[ufunction(Override)]
    fn receive_end_play(&mut self, _end_play_reason: u8) {
        let me = self.as_ref().upcast_to();
        KismetSystemLibrary::k2_clear_timer(me, "StartAoE");
        KismetSystemLibrary::k2_clear_timer(me, "StopAoE");
    }

    #[ufunction(name = "StartAoE")]
    fn start_ao_e(&mut self) {
        self.set_is_ao_e_active(true);

        let Ok(sphere) = self.collision_sphere().and_then(|s| s.checked()) else {
            return;
        };

        for current in
            sphere.get_overlapping_actors(Some(SubclassOf::<TwinStickNPC>::base().upcast_to()))
        {
            if let Ok(npc) = TwinStickNPC::from_obj(current) {
                npc.projectile_impact();
            }
        }
    }

    #[ufunction(name = "StopAoE")]
    fn stop_ao_e(&mut self) {
        self.set_is_ao_e_active(false);

        KismetSystemLibrary::k2_clear_timer(self.as_ref().upcast_to(), "StartAoE");

        self.bp_ao_e_finished();
    }

    #[ufunction(BlueprintImplementableEvent, name = "BP_AoEFinished")]
    fn bp_ao_e_finished(&self) {}
}

impl TwinStickAoEAttack {
    fn bind_overlap(&self) -> RustealResult<()> {
        let me: UObjectRef<Actor> = self.as_ref();

        self.collision_sphere()?
            .checked()?
            .on_component_begin_overlap()
            .add(
                move |_overlapped, other_actor, _other_comp, _body_index, _from_sweep, _sweep| {
                    if let Ok(aoe) = TwinStickAoEAttack::from_obj(me) {
                        aoe.on_ao_e_overlap(other_actor);
                    }
                },
            )?
            .detach();

        Ok(())
    }

    fn on_ao_e_overlap(&self, other_actor: UObjectRef<Actor>) {
        if self.is_ao_e_active()
            && let Ok(npc) = TwinStickNPC::from_obj(other_actor)
        {
            npc.projectile_impact();
        }
    }
}
