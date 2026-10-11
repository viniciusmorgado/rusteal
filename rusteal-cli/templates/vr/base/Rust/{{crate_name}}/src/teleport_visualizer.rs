use bindings::engine::{
    Actor, ActorExt, GameplayStatics, PlayerCameraManagerExt, SceneComponent, SceneComponentExt,
};
use bindings::head_mounted_display::EHMDTrackingOrigin;
use bindings::niagara::{NiagaraComponent, NiagaraComponentExt};
use bindings::prelude::*;
use bindings::xr_base::HeadMountedDisplayFunctionLibrary;
use glam::DVec3;
use rusteal_runtime::runtime::{FName, RustealResult};
use rusteal_runtime::{uclass, uclass_impl};

#[uclass(parent = Actor)]
pub struct TeleportVisualizer {
    #[component(root)]
    root: SceneComponent,

    #[component(attach = "root", name = "NS_PlayAreaBounds")]
    ns_play_area_bounds: NiagaraComponent,

    #[component(attach = "root", name = "NS_TeleportRing")]
    ns_teleport_ring: NiagaraComponent,
}

#[uclass_impl]
impl TeleportVisualizer {
    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        let bounds = HeadMountedDisplayFunctionLibrary::get_play_area_bounds(Some(
            EHMDTrackingOrigin::Stage,
        ))
        .to_dvec2();

        if let Ok(niagara) = self.ns_play_area_bounds().and_then(|n| n.checked()) {
            niagara.set_variable_vec3(
                FName::new("User.PlayAreaBounds").handle(),
                &FVector::from_dvec3(DVec3::new(bounds.x, bounds.y, 0.0)),
            );
        }
    }

    #[ufunction(Override)]
    fn receive_tick(&mut self, _delta_seconds: f32) {
        if let Err(e) = self.follow_player() {
            log::warn!("[VR] teleport visualizer update failed: {e}");
        }
    }
}

impl TeleportVisualizer {
    fn follow_player(&self) -> RustealResult<()> {
        let me = self.as_ref();
        let pawn = GameplayStatics::get_player_pawn(me.upcast_to(), 0).checked()?;
        let camera = GameplayStatics::get_player_camera_manager(me.upcast_to(), 0).checked()?;

        let local = bindings::engine::KismetMathLibrary::inverse_transform_location(
            &pawn.get_transform(),
            &camera.get_camera_location(),
        )
        .to_dvec3();

        self.ns_play_area_bounds()?
            .checked()?
            .k2_set_relative_location(
                &FVector::from_dvec3(DVec3::new(-local.x, -local.y, 0.0)),
                false,
                false,
            );

        me.checked()?
            .k2_set_actor_rotation(&pawn.k2_get_actor_rotation(), false);
        Ok(())
    }
}
