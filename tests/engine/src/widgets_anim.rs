use std::sync::Mutex;

use bindings::core_ue::Object;
use bindings::engine::{
    ActorComponent, ActorExt, AnimInstance, EVisibilityBasedAnimTickOption, GameplayStatics,
    KismetSystemLibrary,
    SkeletalMesh, SkeletalMeshComponent, SkeletalMeshComponentExt, SkinnedMeshComponentExt,
};
use bindings::prelude::*;
use bindings::rusteal::{RustealAnimInstance, RustealUserWidget};
use bindings::slate_core::FGeometry;
use bindings::umg::{UserWidgetExt, WidgetExt};
use rusteal_runtime::runtime::{SubclassOf, Transform, UObjectRef, UStructRef, task};
use rusteal_runtime::{uclass, uclass_impl};

use crate::runner::{Context, Outcome};

static WIDGET_EVENTS: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());

fn widget_event(what: &'static str) {
    let mut events = WIDGET_EVENTS.lock().unwrap();

    if events.last() != Some(&what) {
        events.push(what);
    }
}

#[uclass(parent = RustealUserWidget)]
pub struct VirtualsWidget {}

#[uclass_impl]
impl VirtualsWidget {
    #[ufunction(Override)]
    fn receive_on_initialized(&mut self) {
        widget_event("on_initialized");
    }

    #[ufunction(Override)]
    fn receive_pre_construct(&mut self) {
        widget_event("pre_construct");
    }

    #[ufunction(Override)]
    fn receive_construct(&mut self) {
        widget_event("construct");
    }

    #[ufunction(Override)]
    fn receive_tick(&mut self, _my_geometry: UStructRef<FGeometry>, _in_delta_time: f32) {
        widget_event("tick");
    }

    #[ufunction(Override)]
    fn receive_destruct(&mut self) {
        widget_event("destruct");
    }
}

#[uclass(parent = RustealAnimInstance)]
pub struct VirtualsAnim {
    #[uproperty]
    initialized: bool,

    #[uproperty]
    updates: i32,

    #[uproperty]
    thread_safe_updates: i32,

    #[uproperty]
    worker_updates: i32,

    #[uproperty]
    unguarded_runs: i32,

    game_thread_only: i32,
}

#[uclass_impl]
impl VirtualsAnim {
    #[ufunction(Override)]
    fn receive_initialize_animation(&mut self) {
        self.set_initialized(true);
    }

    #[ufunction(Override)]
    fn receive_update_animation(&mut self, _delta_seconds: f32) {
        self.set_updates(self.updates() + 1);
        *self.game_thread_only_mut() += 1;
    }

    #[ufunction(Override, ThreadSafe)]
    fn receive_thread_safe_update_animation(this: VirtualsAnimThreadSafe, _delta_seconds: f32) {
        this.set_thread_safe_updates(this.thread_safe_updates() + 1);

        if !task::is_game_thread() {
            this.set_worker_updates(this.worker_updates() + 1);
        }
    }

    #[ufunction(Override)]
    fn blueprint_thread_safe_update_animation(&mut self, _delta_time: f32) {
        self.set_unguarded_runs(self.unguarded_runs() + 1);
    }
}

fn ensure(condition: bool, what: impl Into<String>) -> Result<(), String> {
    if condition { Ok(()) } else { Err(what.into()) }
}

pub fn widget_virtuals_reach_rust(context: &Context) -> Outcome {
    WIDGET_EVENTS.lock().unwrap().clear();

    let started = (|| {
        let e = |e| format!("{e:?}");
        let world: UObjectRef<Object> = context.world.upcast_to();
        let controller = GameplayStatics::get_player_controller(world, 0);
        let widget = create_widget::<VirtualsWidget>(&controller).map_err(e)?;
        widget.checked().map_err(e)?.add_to_viewport(None);
        Ok::<_, String>(widget)
    })();

    let widget = match started {
        Ok(widget) => widget,
        Err(e) => return Outcome::Done(Err(e)),
    };

    let mut frames = 0;

    Outcome::Pending(Box::new(move || {
        frames += 1;

        if frames == 10
            && let Ok(widget) = widget.checked()
        {
            widget.remove_from_parent();
        }

        if frames < 14 {
            return None;
        }

        let events = WIDGET_EVENTS.lock().unwrap().clone();
        let expected = ["on_initialized", "pre_construct", "construct", "destruct"];
        let in_order = expected
            .iter()
            .map(|what| events.iter().position(|e| e == what))
            .collect::<Option<Vec<_>>>()
            .is_some_and(|positions| positions.windows(2).all(|w| w[0] < w[1]));

        Some(ensure(in_order, format!("the widget raised {events:?}")))
    }))
}

pub fn anim_instance_updates_off_the_game_thread(context: &Context) -> Outcome {
    let started = (|| {
        let e = |e| format!("{e:?}");
        let world: UObjectRef<Object> = context.world.upcast_to();
        KismetSystemLibrary::execute_console_command(world, "a.ForceParallelAnimUpdate 1", None);

        let transform = FTransform::from_transform(Transform::from_translation(
            glam::DVec3::new(-40_000.0, 0.0, 40_000.0),
        ));
        let actor = context
            .world
            .spawn_actor::<Actor>(&transform)
            .map_err(e)?;
        let checked = actor.checked().map_err(e)?;

        let component = checked.add_component_by_class(
            SubclassOf::<SkeletalMeshComponent>::base().upcast_to::<ActorComponent>(),
            false,
            &transform,
            false,
        );
        let mesh_component = component
            .cast::<SkeletalMeshComponent>()
            .map_err(e)?
            .checked()
            .map_err(e)?;

        let mesh = load_object::<SkeletalMesh>("/Engine/EngineMeshes/SkeletalCube.SkeletalCube")
            .map_err(e)?;
        mesh_component.set_visibility_based_anim_tick_option(
            EVisibilityBasedAnimTickOption::AlwaysTickPoseAndRefreshBones,
        );
        mesh_component.set_anim_instance_class(
            SubclassOf::<VirtualsAnim>::base().upcast_to::<Object>(),
        );
        mesh_component.set_skeletal_mesh_asset(mesh);

        Ok::<_, String>((actor, mesh_component.as_ref()))
    })();

    let (actor, mesh_component) = match started {
        Ok(started) => started,
        Err(e) => return Outcome::Done(Err(e)),
    };

    let mut frames = 0;

    Outcome::Pending(Box::new(move || {
        frames += 1;

        if frames < 30 {
            return None;
        }

        let result = (|| {
            let e = |e| format!("{e:?}");
            let anim: UObjectRef<AnimInstance> = mesh_component.checked().map_err(e)?.get_anim_instance();
            let anim = VirtualsAnim::from_obj(anim).map_err(e)?;

            let updates = anim.updates();
            let thread_safe = anim.thread_safe_updates();
            let worker = anim.worker_updates();
            let unguarded = anim.unguarded_runs();
            let summary = format!(
                "initialized {}, {updates} updates, {thread_safe} thread-safe ({worker} on a \
                 worker), {unguarded} runs of an unguarded override",
                anim.initialized()
            );

            ensure(anim.initialized() && updates > 0 && thread_safe > 0, summary.clone())?;
            ensure(worker > 0, format!("no thread-safe update ran on a worker: {summary}"))?;
            ensure(
                unguarded <= thread_safe - worker,
                format!("an override without ThreadSafe ran on a worker: {summary}"),
            )?;
            ensure(
                anim.game_thread_only() == updates,
                format!("{} game-thread updates for {updates}", anim.game_thread_only()),
            )
        })();

        if let Ok(actor) = actor.checked() {
            actor.k2_destroy_actor();
        }

        Some(result)
    }))
}
