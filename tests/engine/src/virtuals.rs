use std::sync::Mutex;
use std::sync::atomic::{AtomicI32, Ordering};

use bindings::core_ue::{FRotatorExt, FVectorExt};
use bindings::engine::{ActorComponent, ActorExt, ETickingGroup};
use bindings::prelude::*;
use bindings::rusteal::{
    RustealActor, RustealActorComponent, RustealCharacter, RustealGameModeBase, RustealPawn,
    RustealPlayerController, RustealProbeActor, RustealSceneComponent,
};
use rusteal_runtime::runtime::tick::live_tick_functions;
use rusteal_runtime::runtime::{OwnedStruct, SubclassOf, Transform, UObjectRef, UStructRef};
use rusteal_runtime::{uclass, uclass_impl};

use crate::runner::{Context, Outcome};

static STAMP: AtomicI32 = AtomicI32::new(1);

fn next_stamp() -> i32 {
    STAMP.fetch_add(1, Ordering::SeqCst)
}

static EVENTS: Mutex<Vec<String>> = Mutex::new(Vec::new());

fn record(what: &str) {
    EVENTS.lock().unwrap().push(what.to_string());
}

fn recorded() -> Vec<String> {
    EVENTS.lock().unwrap().clone()
}

fn position(events: &[String], what: &str) -> Option<usize> {
    events.iter().position(|event| event == what)
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

    #[ufunction(Override)]
    fn receive_post_actor_created(&mut self) {
        record("actor:post_actor_created");
    }

    #[ufunction(Override)]
    fn receive_post_register_all_components(&mut self) {
        record("actor:post_register_all_components");
    }

    #[ufunction(Override)]
    fn receive_on_construction(&mut self, _transform: UStructRef<FTransform>) {
        record("actor:on_construction");
    }

    #[ufunction(Override)]
    fn receive_tick(&mut self, _delta_seconds: f32) {
        record("actor:tick");
    }
}

#[uclass(parent = RustealPawn)]
pub struct VirtualsPawn {}

#[uclass_impl]
impl VirtualsPawn {
    #[ufunction(Override)]
    fn receive_post_initialize_components(&mut self) {
        record("pawn:post_initialize_components");
    }
}

#[uclass(parent = RustealCharacter)]
pub struct VirtualsCharacter {}

#[uclass_impl]
impl VirtualsCharacter {
    #[ufunction(Override)]
    fn receive_post_initialize_components(&mut self) {
        record("character:post_initialize_components");
    }
}

#[uclass(parent = RustealPlayerController)]
pub struct VirtualsPlayerController {}

#[uclass_impl]
impl VirtualsPlayerController {
    #[ufunction(Override)]
    fn receive_pre_initialize_components(&mut self) {
        record("controller:pre_initialize_components");
    }
}

#[uclass(parent = RustealGameModeBase)]
pub struct VirtualsGameMode {}

#[uclass_impl]
impl VirtualsGameMode {
    #[ufunction(Override)]
    fn receive_post_actor_created(&mut self) {
        record("game_mode:post_actor_created");
    }
}

#[uclass(parent = RustealActorComponent)]
pub struct VirtualsComponent {}

#[uclass_impl]
impl VirtualsComponent {
    #[ufunction(Override)]
    fn receive_on_register(&mut self) {
        record("component:on_register");
    }

    #[ufunction(Override)]
    fn receive_initialize_component(&mut self) {
        record("component:initialize_component");
    }

    #[ufunction(Override)]
    fn receive_tick_component(&mut self, _delta_time: f32) {
        record("component:tick");
    }

    #[ufunction(Override)]
    fn receive_uninitialize_component(&mut self) {
        record("component:uninitialize_component");
    }

    #[ufunction(Override)]
    fn receive_on_unregister(&mut self) {
        record("component:on_unregister");
    }
}

#[uclass(parent = RustealSceneComponent)]
pub struct VirtualsSceneComponent {}

#[uclass_impl]
impl VirtualsSceneComponent {
    #[ufunction(Override)]
    fn receive_on_register(&mut self) {
        record("scene_component:on_register");
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

fn spawn_at<T: rusteal_runtime::runtime::UeClass>(
    context: &Context,
) -> Result<UObjectRef<T>, String> {
    let transform = FTransform::from_transform(Transform::IDENTITY);

    context
        .world
        .spawn_actor::<T>(&transform)
        .map_err(|e| format!("spawn: {e:?}"))
}

pub fn every_curated_virtual_reaches_rust(context: &Context) -> Outcome {
    EVENTS.lock().unwrap().clear();
    let live_before = live_tick_functions();

    let started = (|| {
        let e = |e| format!("{e:?}");
        let actor = spawn_at::<VirtualsActor>(context)?;
        let checked = actor.checked().map_err(e)?;
        let identity = FTransform::from_transform(Transform::IDENTITY);

        checked.add_component_by_class(
            SubclassOf::<VirtualsComponent>::base().upcast_to::<ActorComponent>(),
            false,
            &identity,
            false,
        );
        checked.add_component_by_class(
            SubclassOf::<VirtualsSceneComponent>::base().upcast_to::<ActorComponent>(),
            false,
            &identity,
            false,
        );

        let others: Vec<UObjectRef<Actor>> = vec![
            spawn_at::<VirtualsPawn>(context)?.upcast_to(),
            context
                .world
                .spawn_actor::<VirtualsCharacter>(&FTransform::from_transform(
                    Transform::from_translation(glam::DVec3::new(0.0, 40_000.0, 40_000.0)),
                ))
                .map_err(|e| format!("spawn the character: {e:?}"))?
                .upcast_to(),
            spawn_at::<VirtualsPlayerController>(context)?.upcast_to(),
            context
                .world
                .spawn_actor_deferred::<VirtualsGameMode>(&identity)
                .map_err(e)?
                .upcast_to(),
        ];

        let late = add_tick_function(&actor, ETickingGroup::TG_PrePhysics, |_| {
            record("tick_function:late")
        })
        .map_err(e)?;
        let early = add_tick_function(&actor, ETickingGroup::TG_PrePhysics, |_| {
            record("tick_function:early")
        })
        .map_err(e)?;
        late.add_prerequisite_function(&early).map_err(e)?;
        late.add_prerequisite(&actor).map_err(e)?;

        let post_physics = add_tick_function(&actor, ETickingGroup::TG_PostPhysics, |delta| {
            if delta > 0.0 {
                record("tick_function:post_physics");
            }
        })
        .map_err(e)?;

        let removed = add_tick_function(&actor, ETickingGroup::TG_PrePhysics, |_| {
            record("tick_function:removed")
        })
        .map_err(e)?;
        removed.remove();

        let _ = post_physics;
        Ok::<_, String>((actor, others))
    })();

    let (actor, others) = match started {
        Ok(started) => started,
        Err(e) => return Outcome::Done(Err(e)),
    };

    let mut frames = 0;
    let mut live_while_running = 0;

    Outcome::Pending(Box::new(move || {
        frames += 1;

        if frames == 8 {
            live_while_running = live_tick_functions();

            if let Ok(actor) = actor.checked() {
                actor.k2_destroy_actor();
            }

            for other in &others {
                if let Ok(other) = other.checked() {
                    other.k2_destroy_actor();
                }
            }
        }

        if frames < 14 {
            return None;
        }

        let events = recorded();
        let has = |what: &str| position(&events, what).is_some();
        let before = |a: &str, b: &str| match (position(&events, a), position(&events, b)) {
            (Some(a), Some(b)) => a < b,
            _ => false,
        };

        let expected = [
            "actor:post_actor_created",
            "actor:post_register_all_components",
            "actor:on_construction",
            "actor:tick",
            "pawn:post_initialize_components",
            "character:post_initialize_components",
            "controller:pre_initialize_components",
            "game_mode:post_actor_created",
            "component:on_register",
            "component:initialize_component",
            "component:tick",
            "component:uninitialize_component",
            "component:on_unregister",
            "scene_component:on_register",
            "tick_function:early",
            "tick_function:late",
            "tick_function:post_physics",
        ];

        let missing: Vec<&str> = expected.iter().copied().filter(|what| !has(what)).collect();

        let late_after_early = events
            .iter()
            .enumerate()
            .filter(|(_, e)| *e == "tick_function:late")
            .all(|(i, _)| events[..i].iter().any(|e| e == "tick_function:early"));

        let live_after = live_tick_functions();

        Some(
            ensure(missing.is_empty(), format!("never raised: {missing:?}"))
                .and_then(|_| {
                    ensure(
                        before("actor:post_actor_created", "actor:on_construction"),
                        "OnConstruction came before PostActorCreated",
                    )
                })
                .and_then(|_| {
                    ensure(
                        before("component:on_register", "component:initialize_component")
                            && before(
                                "component:uninitialize_component",
                                "component:on_unregister",
                            ),
                        "the component's events are out of order",
                    )
                })
                .and_then(|_| {
                    ensure(
                        late_after_early,
                        "a tick function ran before its prerequisite",
                    )
                })
                .and_then(|_| {
                    ensure(
                        !has("tick_function:removed"),
                        "a removed tick function ran",
                    )
                })
                .and_then(|_| {
                    ensure(
                        live_while_running == live_before + 3 && live_after == live_before,
                        format!(
                            "{live_before} tick functions before, {live_while_running} while \
                             running, {live_after} after the actor was destroyed"
                        ),
                    )
                }),
        )
    }))
}
