use std::sync::atomic::{AtomicI32, Ordering};

use bindings::core_ue::{FRotatorExt, FVectorExt};
use bindings::engine::ActorExt;
use bindings::prelude::*;
use bindings::rusteal::{RustealActor, RustealProbeActor};
use rusteal_runtime::runtime::{OwnedStruct, Transform, UStructRef};
use rusteal_runtime::{uclass, uclass_impl};

use crate::runner::{Context, Outcome};

static STAMP: AtomicI32 = AtomicI32::new(1);

fn next_stamp() -> i32 {
    STAMP.fetch_add(1, Ordering::SeqCst)
}

#[uclass(parent = RustealActor)]
pub struct VirtualsActor {
    #[uproperty]
    pre_initialize_stamp: i32,

    #[uproperty]
    post_initialize_stamp: i32,

    #[uproperty]
    begin_play_stamp: i32,
}

#[uclass_impl]
impl VirtualsActor {
    #[ufunction(Override)]
    fn receive_pre_initialize_components(&mut self) {
        self.set_pre_initialize_stamp(next_stamp());
    }

    #[ufunction(Override)]
    fn receive_post_initialize_components(&mut self) {
        self.set_post_initialize_stamp(next_stamp());
    }

    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        self.set_begin_play_stamp(next_stamp());
    }
}

#[uclass(parent = RustealProbeActor)]
pub struct ProbeActor {}

#[uclass_impl]
impl ProbeActor {
    #[ufunction(Override)]
    fn receive_get_velocity(&self) -> OwnedStruct<FVector> {
        FVector::from_dvec3(glam::DVec3::new(1.0, 2.0, 3.0))
    }

    #[ufunction(Override)]
    fn receive_get_actor_eyes_view_point(
        &self,
        out_location: UStructRef<FVector>,
        out_rotation: UStructRef<FRotator>,
    ) {
        out_location.set_z(170.0);
        out_rotation.set_yaw(45.0);
    }
}

fn ensure(condition: bool, what: impl Into<String>) -> Result<(), String> {
    if condition { Ok(()) } else { Err(what.into()) }
}

pub fn actor_virtuals_reach_rust(context: &Context) -> Outcome {
    Outcome::Done((|| {
        let e = |e| format!("{e:?}");
        let transform = FTransform::from_transform(Transform::IDENTITY);
        let actor = context
            .world
            .spawn_actor::<VirtualsActor>(&transform)
            .map_err(e)?;
        let me = VirtualsActor::from_obj(actor).map_err(e)?;

        let pre = me.pre_initialize_stamp();
        let post = me.post_initialize_stamp();
        let begin = me.begin_play_stamp();

        ensure(
            pre > 0 && pre < post && post < begin,
            format!("pre {pre}, post {post}, begin play {begin}"),
        )?;

        actor.checked().map_err(e)?.k2_destroy_actor();
        Ok(())
    })())
}

pub fn project_virtuals_reach_rust(context: &Context) -> Outcome {
    Outcome::Done((|| {
        let e = |e| format!("{e:?}");
        let transform = FTransform::from_transform(Transform::IDENTITY);
        let actor = context
            .world
            .spawn_actor::<ProbeActor>(&transform)
            .map_err(e)?;
        let checked = actor.checked().map_err(e)?;

        let velocity = checked.get_velocity().to_dvec3();
        ensure(
            velocity == glam::DVec3::new(1.0, 2.0, 3.0),
            format!("GetVelocity gave {velocity:?}"),
        )?;

        let (location, rotation) = checked.get_actor_eyes_view_point();
        let location = location.to_dvec3();
        let yaw = rotation.as_ref().get_yaw();
        ensure(
            location.z == 170.0 && yaw == 45.0,
            format!("GetActorEyesViewPoint gave {location:?} and a yaw of {yaw}"),
        )?;

        checked.k2_destroy_actor();
        Ok(())
    })())
}
