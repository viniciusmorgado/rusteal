use bindings::engine::{
    Actor, ActorComponentExt, PrimitiveComponentExt, SceneComponentExt, StaticMeshComponent,
};
use bindings::prelude::*;
use glam::DVec3;
use rusteal_runtime::runtime::{FName, RustealResult};
use rusteal_runtime::{uclass, uclass_impl};

use crate::grab_component::GrabComponent;

#[uclass(parent = Actor)]
pub struct GrabbableSmallCube {
    #[component(root)]
    static_mesh: StaticMeshComponent,

    #[component(attach = "static_mesh")]
    grab_component: GrabComponent,
}

#[uclass_impl]
impl GrabbableSmallCube {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        let mesh = self.static_mesh()?.checked()?;
        mesh.set_simulate_physics(true);
        mesh.set_collision_profile_name(FName::new("PhysicsActor").handle(), Some(true));
        mesh.set_can_ever_affect_navigation(false);
        mesh.set_relative_scale3_d(&FVector::from_dvec3(DVec3::splat(0.15)));

        self.grab_component()?
            .upcast_to::<bindings::engine::SceneComponent>()
            .checked()?
            .set_relative_scale3_d(&FVector::from_dvec3(DVec3::splat(3.333333)));

        Ok(())
    }
}
