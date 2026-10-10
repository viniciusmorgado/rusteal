use bindings::engine::{
    CameraComponent, CameraComponentExt, ECameraProjectionMode, FloatingPawnMovement,
    MovementComponentExt, Pawn, SceneComponent,
};
use bindings::prelude::*;
use glam::DVec3;
use rusteal_runtime::runtime::{RustealResult, UObjectRef};
use rusteal_runtime::{uclass, uclass_impl};

const ORTHO_WIDTH: f32 = 1500.0;
const AUTO_PLANE_SHIFT: f32 = 1.0;
const PLANE_HEIGHT: f64 = 1500.0;

#[uclass(parent = Pawn)]
pub struct StrategyPawn {
    #[component(root, name = "Root")]
    root: SceneComponent,

    #[component(attach = "root")]
    camera: CameraComponent,

    #[component(name = "Floating Pawn Movement")]
    floating_pawn_movement: FloatingPawnMovement,
}

#[uclass_impl]
impl StrategyPawn {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        let camera = self.camera()?.checked()?;
        camera.set_projection_mode(ECameraProjectionMode::Orthographic);
        camera.set_ortho_width(ORTHO_WIDTH);
        camera.set_auto_plane_shift(AUTO_PLANE_SHIFT);
        camera.set_update_ortho_planes(false);

        let movement = self.floating_pawn_movement()?.checked()?;
        movement.set_plane_constraint_enabled(true);
        movement.set_plane_constraint_normal(&FVector::from_dvec3(DVec3::Z));
        movement.set_plane_constraint_origin(&FVector::from_dvec3(DVec3::Z * PLANE_HEIGHT));

        Ok(())
    }
}

impl StrategyPawn {
    pub fn set_zoom_modifier(&self, value: f32) -> RustealResult<()> {
        self.camera()?.checked()?.set_ortho_width(value);

        Ok(())
    }

    pub fn get_camera(&self) -> RustealResult<UObjectRef<CameraComponent>> {
        self.camera()
    }
}
