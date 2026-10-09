use bindings::engine::{
    Actor, EDrawDebugTrace, ETraceTypeQuery, GameplayStatics, KismetSystemLibrary, Pawn,
    PlayerCameraManager, PlayerCameraManagerExt,
};
use bindings::prelude::*;
use rusteal_runtime::runtime::{OutRef, OwnedStruct, RustealResult, UObjectRef, UStructRef};
use rusteal_runtime::{uclass, uclass_impl};

use super::model::{self, CameraFrame, CameraSettings, CameraState};

#[uclass(parent = PlayerCameraManager)]
pub struct SideScrollingCameraManager {
    #[uproperty(EditAnywhere, default = model::CURRENT_ZOOM)]
    current_zoom: f32,

    #[uproperty(EditAnywhere, default = model::CAMERA_Z_OFFSET)]
    camera_z_offset: f32,

    #[uproperty(EditAnywhere, default = model::CAMERA_X_MIN_BOUNDS)]
    camera_x_min_bounds: f32,

    #[uproperty(EditAnywhere, default = model::CAMERA_X_MAX_BOUNDS)]
    camera_x_max_bounds: f32,

    state: CameraState,
}

#[uclass_impl]
impl SideScrollingCameraManager {
    #[ufunction(Override)]
    fn blueprint_update_camera(
        &mut self,
        camera_target: UObjectRef<Actor>,
        new_camera_location: UStructRef<FVector>,
        new_camera_rotation: UStructRef<FRotator>,
        new_camera_fov: OutRef<f32>,
    ) -> bool {
        self.update(
            camera_target,
            new_camera_location,
            new_camera_rotation,
            new_camera_fov,
        )
        .unwrap_or(false)
    }
}

impl SideScrollingCameraManager {
    fn update(
        &mut self,
        camera_target: UObjectRef<Actor>,
        new_camera_location: UStructRef<FVector>,
        new_camera_rotation: UStructRef<FRotator>,
        new_camera_fov: OutRef<f32>,
    ) -> RustealResult<bool> {
        let Ok(target_pawn) = camera_target.cast::<Pawn>() else {
            return Ok(false);
        };

        let pawn = target_pawn.checked()?;

        new_camera_rotation.set_pitch(0.0);
        new_camera_rotation.set_yaw(model::CAMERA_VIEW_YAW);
        new_camera_rotation.set_roll(0.0);
        new_camera_fov.set(model::CAMERA_FOV);

        let target_location = pawn.k2_get_actor_location().to_dvec3();
        let world_context = self.as_ref().upcast_to();

        let target_moving_vertically = pawn.get_velocity().to_dvec3().z.abs() > 1.0e-8;

        let ground_below = target_moving_vertically && {
            let end = target_location + glam::DVec3::new(0.0, 0.0, -1000.0);
            KismetSystemLibrary::line_trace_single(
                world_context,
                &FVector::from_dvec3(target_location),
                &FVector::from_dvec3(end),
                ETraceTypeQuery::TraceTypeQuery1,
                false,
                &[camera_target],
                EDrawDebugTrace::None,
                false,
                &OwnedStruct::new(),
                &OwnedStruct::new(),
                None,
            )
            .0
        };

        let frame = CameraFrame {
            target_location,
            camera_location: self.as_ref().checked()?.get_camera_location().to_dvec3(),
            target_moving_vertically,
            ground_below,
            delta_time: GameplayStatics::get_world_delta_seconds(world_context),
        };

        let settings = CameraSettings {
            zoom: self.current_zoom(),
            z_offset: self.camera_z_offset(),
            x_min: self.camera_x_min_bounds(),
            x_max: self.camera_x_max_bounds(),
        };

        let (location, state) = model::camera_location(&settings, &self.state(), &frame);
        self.set_state(state);

        new_camera_location.set_x(location.x);
        new_camera_location.set_y(location.y);
        new_camera_location.set_z(location.z);

        Ok(true)
    }
}
