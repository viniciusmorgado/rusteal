use std::sync::{Arc, Mutex};

use bindings::engine::{KismetMathLibrary, KismetStringLibrary};
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
    (
        "ufunction_return_by_reflection",
        ufunction_return_by_reflection,
    ),
    (
        "ufunction_param_by_reflection",
        ufunction_param_by_reflection,
    ),
    ("ufunction_echo_by_reflection", ufunction_echo_by_reflection),
    ("component_is_created", component_is_created),
    ("udelegate_reaches_a_closure", udelegate_reaches_a_closure),
    ("engine_function_call", engine_function_call),
    ("engine_string_is_utf8", engine_string_is_utf8),
    (
        "engine_string_longer_than_512_bytes",
        engine_string_longer_than_512_bytes,
    ),
    (
        "rust_string_property_longer_than_512_bytes",
        rust_string_property_longer_than_512_bytes,
    ),
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

fn ufunction_return_by_reflection(context: &Context) -> Outcome {
    done((|| {
        let actor = TestActor::from_obj(spawn(context)?).map_err(|e| format!("{e:?}"))?;
        actor.set_count(9);
        let call = DynamicCall::new(&actor.as_ref(), "ReadCount").map_err(|e| format!("{e:?}"))?;
        let result = call.call().map_err(|e| format!("{e:?}"))?;
        let value: i32 = result.get("ReturnValue").map_err(|e| format!("{e:?}"))?;
        ensure(value == 9, format!("ReadCount returned {value}"))
    })())
}

fn ufunction_param_by_reflection(context: &Context) -> Outcome {
    done((|| {
        let actor = TestActor::from_obj(spawn(context)?).map_err(|e| format!("{e:?}"))?;
        let mut call = DynamicCall::new(&actor.as_ref(), "Store").map_err(|e| format!("{e:?}"))?;
        call.set("Value", 13i32).map_err(|e| format!("{e:?}"))?;
        call.call().map_err(|e| format!("{e:?}"))?;
        ensure(
            actor.count() == 13,
            format!("Store left count at {}", actor.count()),
        )
    })())
}

fn ufunction_echo_by_reflection(context: &Context) -> Outcome {
    done((|| {
        let actor = spawn(context)?;
        let mut call = DynamicCall::new(&actor, "Echo").map_err(|e| format!("{e:?}"))?;
        call.set("Value", 21i32).map_err(|e| format!("{e:?}"))?;
        let result = call.call().map_err(|e| format!("{e:?}"))?;
        let value: i32 = result.get("ReturnValue").map_err(|e| format!("{e:?}"))?;
        ensure(value == 21, format!("Echo returned {value}"))
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

fn engine_string_is_utf8(_: &Context) -> Outcome {
    let joined = KismetStringLibrary::concat_str_str("ação", "ñü");
    done(ensure(joined == "açãoñü", format!("got {joined:?}")))
}

fn engine_string_longer_than_512_bytes(_: &Context) -> Outcome {
    let a = "ação-".repeat(120);
    let b = "ñü-".repeat(150);
    let joined = KismetStringLibrary::concat_str_str(&a, &b);

    done(ensure(
        joined == format!("{a}{b}"),
        format!("got {} bytes of {}", joined.len(), a.len() + b.len()),
    ))
}

fn rust_string_property_longer_than_512_bytes(context: &Context) -> Outcome {
    done((|| {
        let actor = TestActor::from_obj(spawn(context)?).map_err(|e| format!("{e:?}"))?;
        let text = "çãõ-".repeat(300);
        actor.set_label(&text);
        ensure(
            actor.label() == text,
            format!("got {} bytes", actor.label().len()),
        )
    })())
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
