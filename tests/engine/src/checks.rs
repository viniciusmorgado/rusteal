use std::sync::{Arc, Mutex};

use bindings::engine::KismetMathLibrary;
use bindings::prelude::*;
use rusteal_runtime::runtime::{DynamicCall, Transform, UObjectRef, UeClass, task};

use crate::fixtures::TestActor;
use crate::runner::{Check, Context, Outcome};

pub const ALL: &[(&str, Check)] = &[
    ("rust_class_is_registered", rust_class_is_registered),
    ("spawn_rust_actor", spawn_rust_actor),
    ("scalar_properties_round_trip", scalar_properties_round_trip),
    ("string_property_round_trip", string_property_round_trip),
    (
        "ufunction_called_by_reflection",
        ufunction_called_by_reflection,
    ),
    ("component_is_created", component_is_created),
    ("udelegate_reaches_a_closure", udelegate_reaches_a_closure),
    ("engine_function_call", engine_function_call),
    (
        "task_comes_back_to_the_game_thread",
        task_comes_back_to_the_game_thread,
    ),
];

fn ensure(condition: bool, what: impl Into<String>) -> Result<(), String> {
    if condition { Ok(()) } else { Err(what.into()) }
}

fn done(result: Result<(), String>) -> Outcome {
    Outcome::Done(result)
}

fn spawn(context: &Context) -> Result<UObjectRef<TestActor>, String> {
    let transform = FTransform::from_transform(Transform::IDENTITY);

    context
        .world
        .spawn_actor::<TestActor>(&transform)
        .map_err(|e| format!("spawn: {e:?}"))
}

fn rust_class_is_registered(_: &Context) -> Outcome {
    done(ensure(
        !TestActor::static_class().is_null(),
        "TestActor has no UClass",
    ))
}

fn spawn_rust_actor(context: &Context) -> Outcome {
    done(spawn(context).map(|_| ()))
}

fn scalar_properties_round_trip(context: &Context) -> Outcome {
    done((|| {
        let actor = TestActor::from_obj(spawn(context)?).map_err(|e| format!("{e:?}"))?;
        actor.set_count(41);
        actor.set_speed(2.5);
        actor.set_b_enabled(true);
        ensure(actor.count() == 41, format!("count is {}", actor.count()))?;
        ensure(actor.speed() == 2.5, format!("speed is {}", actor.speed()))?;
        ensure(actor.b_enabled(), "enabled is false")
    })())
}

fn string_property_round_trip(context: &Context) -> Outcome {
    done((|| {
        let actor = TestActor::from_obj(spawn(context)?).map_err(|e| format!("{e:?}"))?;
        let text = "héllo wörld";
        actor.set_label(text);
        ensure(
            actor.label() == text,
            format!("label is {:?}", actor.label()),
        )
    })())
}

fn ufunction_called_by_reflection(context: &Context) -> Outcome {
    done((|| {
        let actor = spawn(context)?;
        let mut call = DynamicCall::new(&actor, "Add").map_err(|e| format!("{e:?}"))?;
        call.set("A", 2i32).map_err(|e| format!("{e:?}"))?;
        call.set("B", 3i32).map_err(|e| format!("{e:?}"))?;
        let result = call.call().map_err(|e| format!("{e:?}"))?;
        let sum: i32 = result.get("ReturnValue").map_err(|e| format!("{e:?}"))?;
        ensure(sum == 5, format!("Add returned {sum}"))
    })())
}

fn component_is_created(context: &Context) -> Outcome {
    done((|| {
        let actor = TestActor::from_obj(spawn(context)?).map_err(|e| format!("{e:?}"))?;
        let root = actor.root().map_err(|e| format!("{e:?}"))?;
        ensure(root.checked().is_ok(), "no root component")
    })())
}

fn udelegate_reaches_a_closure(context: &Context) -> Outcome {
    done((|| {
        let actor = TestActor::from_obj(spawn(context)?).map_err(|e| format!("{e:?}"))?;
        let seen = Arc::new(Mutex::new(0));
        let sink = seen.clone();

        actor
            .on_ping_add(move |value| *sink.lock().unwrap() = value)
            .map_err(|e| format!("{e:?}"))?
            .detach();

        actor.ping(7);
        let value = *seen.lock().unwrap();
        ensure(value == 7, format!("the listener saw {value}"))
    })())
}

fn engine_function_call(_: &Context) -> Outcome {
    let sum = KismetMathLibrary::add_int_int(40, Some(2));
    done(ensure(sum == 42, format!("Add_IntInt returned {sum}")))
}

fn task_comes_back_to_the_game_thread(_: &Context) -> Outcome {
    let result = Arc::new(Mutex::new(None));
    let sink = result.clone();

    task::spawn(|| 6 * 7, move |value| *sink.lock().unwrap() = Some(value));

    Outcome::Pending(Box::new(move || {
        result
            .lock()
            .unwrap()
            .map(|value| ensure(value == 42, format!("the task gave {value}")))
    }))
}
