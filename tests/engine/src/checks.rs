use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use bindings::core_ue::{
    FARFilter, FARFilterExt, FGuid, FInstancedStruct, FIntPoint, FIntPointExt,
};
use bindings::engine::SceneComponentExt;
use bindings::engine::{
    ActorComponent, BlendableInterface, CameraComponent, CameraComponentExt, CurveFloat,
    CurveFloatExt, ECollisionChannel, EMoveComponentAction, FMinimalViewInfoExt,
    FPerQualityLevelInt, FPerQualityLevelIntExt, FPostProcessSettingsExt, FRichCurveExt,
    FRichCurveKey, FRichCurveKeyExt, FWeightedBlendableExt, FWeightedBlendablesExt,
    GameplayStatics, InputSettings, InputSettingsExt, KismetGuidLibrary, KismetMathLibrary,
    KismetStringLibrary, KismetSystemLibrary, KismetTextLibrary, Material, PlayerInput,
    PostProcessVolume, PostProcessVolumeExt, ProjectileMovementComponent,
    ProjectileMovementComponentExt, StaticMeshActor, TextRenderComponent, TextRenderComponentExt,
    TimelineComponent, TimelineComponentExt, VolumetricCloudComponent, VolumetricCloudComponentExt,
};
use bindings::prelude::*;
use bindings::slate_core::{
    EVerticalAlignment, FSlateBrush, FToolBarStyle, FToolBarStyleExt, FWrapButtonStyle,
    FWrapButtonStyleExt,
};
use rusteal_runtime::runtime::{
    DynamicCall, FName, Inherits, OwnedStruct, Rotator, SoftClassRef, SoftObjectRef, SubclassOf,
    TWeakObjectPtr, Text, Transform, UObjectRef, UeClass, UeStruct, task, timer,
};

use crate::fixtures::{
    ChildTestActor, Loadout, LoadoutExt, Mood, Pingable, TestActor, TestLibrary,
};
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
    (
        "udelegate_listener_survives_garbage_collection",
        udelegate_listener_survives_garbage_collection,
    ),
    (
        "udelegate_carries_strings_arrays_and_texts",
        udelegate_carries_strings_arrays_and_texts,
    ),
    (
        "single_udelegate_calls_its_one_binding",
        single_udelegate_calls_its_one_binding,
    ),
    (
        "sparse_udelegate_keeps_listeners_per_object",
        sparse_udelegate_keeps_listeners_per_object,
    ),
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
        "engine_function_takes_an_interface",
        engine_function_takes_an_interface,
    ),
    (
        "engine_function_takes_any_object_as_iinterface",
        engine_function_takes_any_object_as_iinterface,
    ),
    (
        "engine_soft_property_keeps_the_path",
        engine_soft_property_keeps_the_path,
    ),
    (
        "engine_soft_reference_crosses_as_a_path",
        engine_soft_reference_crosses_as_a_path,
    ),
    (
        "engine_weak_property_follows_its_object",
        engine_weak_property_follows_its_object,
    ),
    (
        "engine_struct_with_an_array_returned_by_a_function",
        engine_struct_with_an_array_returned_by_a_function,
    ),
    (
        "engine_struct_passed_by_reference",
        engine_struct_passed_by_reference,
    ),
    (
        "engine_soft_class_reference_crosses_as_a_path",
        engine_soft_class_reference_crosses_as_a_path,
    ),
    (
        "engine_soft_class_property_keeps_the_path",
        engine_soft_class_property_keeps_the_path,
    ),
    ("engine_text_round_trip", engine_text_round_trip),
    ("engine_optional_fields", engine_optional_fields),
    (
        "instanced_struct_holds_a_struct",
        instanced_struct_holds_a_struct,
    ),
    (
        "engine_structs_without_static_struct",
        engine_structs_without_static_struct,
    ),
    (
        "engine_text_keeps_its_localization",
        engine_text_keeps_its_localization,
    ),
    (
        "engine_function_takes_a_closure",
        engine_function_takes_a_closure,
    ),
    (
        "engine_member_function_takes_a_closure_with_arguments",
        engine_member_function_takes_a_closure_with_arguments,
    ),
    ("rust_integer_properties", rust_integer_properties),
    (
        "rust_text_and_string_properties",
        rust_text_and_string_properties,
    ),
    ("rust_reference_properties", rust_reference_properties),
    ("rust_container_properties", rust_container_properties),
    ("rust_struct_fields", rust_struct_fields),
    (
        "every_other_property_kind_round_trips",
        every_other_property_kind_round_trips,
    ),
    (
        "ufunction_takes_and_returns_any_type",
        ufunction_takes_and_returns_any_type,
    ),
    (
        "interface_function_takes_any_type",
        interface_function_takes_any_type,
    ),
    ("ufunction_out_parameters", ufunction_out_parameters),
    (
        "native_event_dispatches_to_the_override",
        native_event_dispatches_to_the_override,
    ),
    (
        "override_takes_containers_and_soft_references",
        override_takes_containers_and_soft_references,
    ),
    (
        "static_functions_of_a_library",
        static_functions_of_a_library,
    ),
    (
        "task_comes_back_to_the_game_thread",
        task_comes_back_to_the_game_thread,
    ),
    (
        "async_task_runs_over_frames_and_stops_with_its_owner",
        async_task_runs_over_frames_and_stops_with_its_owner,
    ),
    (
        "timers_fire_clear_and_free_their_closures",
        timers_fire_clear_and_free_their_closures,
    ),
    (
        "sleep_follows_world_time_and_pause",
        sleep_follows_world_time_and_pause,
    ),
    (
        "latent_engine_functions_resume_their_task",
        latent_engine_functions_resume_their_task,
    ),
    (
        "latent_action_of_a_destroyed_owner_is_cancelled",
        latent_action_of_a_destroyed_owner_is_cancelled,
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

fn udelegate_carries_strings_arrays_and_texts(context: &Context) -> Outcome {
    done((|| {
        let actor = TestActor::from_obj(spawn(context)?).map_err(|e| format!("{e:?}"))?;
        let seen = Arc::new(Mutex::new(None));
        let sink = seen.clone();

        let _binding = actor
            .on_named_add(move |name, tags, title| {
                *sink.lock().unwrap() = Some((name, tags, title));
            })
            .map_err(|e| format!("{e:?}"))?;

        let long = "ç".repeat(400);

        actor.on_named(
            long.clone(),
            vec![FName::new("Fast"), FName::new("Loud")],
            Text::from("Captain"),
        );

        let (name, tags, title) = seen
            .lock()
            .unwrap()
            .take()
            .ok_or("the listener saw nothing")?;
        ensure(
            name == long,
            format!("the name crossed as {} bytes", name.len()),
        )?;
        let tags: Vec<String> = tags.iter().map(FName::to_string).collect();
        ensure(
            tags == ["Fast", "Loud"],
            format!("the tags crossed as {tags:?}"),
        )?;
        ensure(
            title.to_string() == "Captain",
            format!("the title crossed as {title}"),
        )
    })())
}

fn single_udelegate_calls_its_one_binding(context: &Context) -> Outcome {
    done((|| {
        let actor = TestActor::from_obj(spawn(context)?).map_err(|e| format!("{e:?}"))?;
        let weights = HashMap::from([("Gold".to_string(), 3), ("Iron".to_string(), 1)]);

        actor.on_asked("Nobody listens".into(), weights.clone());

        let first = Arc::new(Mutex::new(Vec::new()));
        let second = Arc::new(Mutex::new(Vec::new()));
        let (first_sink, second_sink) = (first.clone(), second.clone());

        let first_binding = actor
            .on_asked_bind(move |question, weights| {
                first_sink.lock().unwrap().push((question, weights));
            })
            .map_err(|e| format!("{e:?}"))?;

        actor.on_asked("Which ore?".into(), weights.clone());

        let second_binding = actor
            .on_asked_bind(move |question, weights| {
                second_sink.lock().unwrap().push((question, weights));
            })
            .map_err(|e| format!("{e:?}"))?;

        drop(first_binding);
        actor.on_asked("And now?".into(), weights.clone());

        drop(second_binding);
        actor.on_asked("Anyone?".into(), weights.clone());

        let first = first.lock().unwrap();
        let second = second.lock().unwrap();

        ensure(
            first.len() == 1 && first[0] == ("Which ore?".to_string(), weights.clone()),
            format!("the first binding saw {first:?}"),
        )?;

        ensure(
            second.len() == 1 && second[0] == ("And now?".to_string(), weights),
            format!("the second binding saw {second:?}"),
        )
    })())
}

fn sparse_udelegate_keeps_listeners_per_object(context: &Context) -> Outcome {
    done((|| {
        let a = TestActor::from_obj(spawn(context)?).map_err(|e| format!("{e:?}"))?;
        let b = TestActor::from_obj(spawn(context)?).map_err(|e| format!("{e:?}"))?;

        a.on_scored(1, "Nobody".into());

        let seen = Arc::new(Mutex::new(Vec::new()));
        let (a_sink, a_sink_2, b_sink) = (seen.clone(), seen.clone(), seen.clone());

        let a_first = a
            .on_scored_add(move |points, by| a_sink.lock().unwrap().push(("a1", points, by)))
            .map_err(|e| format!("{e:?}"))?;

        let _a_second = a
            .on_scored_add(move |points, by| a_sink_2.lock().unwrap().push(("a2", points, by)))
            .map_err(|e| format!("{e:?}"))?;

        let _b = b
            .on_scored_add(move |points, by| b_sink.lock().unwrap().push(("b", points, by)))
            .map_err(|e| format!("{e:?}"))?;

        a.on_scored(10, "Ada".into());
        drop(a_first);
        a.on_scored(20, "Ada".into());
        b.on_scored(30, "Lin".into());

        let mut seen = seen.lock().unwrap().clone();
        seen.sort();

        let expected = vec![
            ("a1", 10, "Ada".to_string()),
            ("a2", 10, "Ada".to_string()),
            ("a2", 20, "Ada".to_string()),
            ("b", 30, "Lin".to_string()),
        ];

        ensure(seen == expected, format!("the listeners saw {seen:?}"))
    })())
}

fn udelegate_listener_survives_garbage_collection(context: &Context) -> Outcome {
    let bound = (|| {
        let actor = spawn(context)?;
        let seen = Arc::new(Mutex::new(0));
        let sink = seen.clone();

        TestActor::from_obj(actor)
            .map_err(|e| format!("{e:?}"))?
            .on_ping_add(move |value| *sink.lock().unwrap() = value)
            .map_err(|e| format!("{e:?}"))?
            .detach();

        Ok::<_, String>((actor, seen))
    })();

    let (actor, seen) = match bound {
        Ok(bound) => bound,
        Err(e) => return done(Err(e)),
    };

    KismetSystemLibrary::collect_garbage();
    let mut frames = 0;

    Outcome::Pending(Box::new(move || {
        frames += 1;

        if frames < 5 {
            return None;
        }

        let fired = TestActor::from_obj(actor).map(|actor| actor.ping(9));

        if let Err(e) = fired {
            return Some(Err(format!("{e:?}")));
        }

        let value = *seen.lock().unwrap();
        Some(ensure(
            value == 9,
            format!("after a garbage collection the listener saw {value}"),
        ))
    }))
}

struct SetOnDrop(Arc<AtomicBool>);

impl Drop for SetOnDrop {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

fn async_task_runs_over_frames_and_stops_with_its_owner(context: &Context) -> Outcome {
    let started = (|| {
        let actor = spawn(context)?;
        let doomed = spawn(context)?;
        let steps = Arc::new(AtomicUsize::new(0));
        let dropped = Arc::new(AtomicBool::new(false));
        let answer = Arc::new(Mutex::new(None));

        let counter = steps.clone();
        let counting = task::spawn_local(&actor, async move {
            for _ in 0..3 {
                task::next_frame().await;
                counter.fetch_add(1, Ordering::SeqCst);
            }

            let worked = task::run_blocking(|| 6 * 7).await;
            worked.unwrap_or(0)
        });

        let sink = answer.clone();
        let _join = task::spawn_local(&actor, async move {
            *sink.lock().unwrap() = counting.await;
        });

        let guard = SetOnDrop(dropped.clone());
        let _parked = task::spawn_local(&doomed, async move {
            let _guard = guard;
            std::future::pending::<()>().await;
        });

        doomed
            .checked()
            .map_err(|e| format!("checked: {e:?}"))?
            .k2_destroy_actor();

        Ok::<_, String>((steps, dropped, answer))
    })();

    let (steps, dropped, answer) = match started {
        Ok(started) => started,
        Err(e) => return done(Err(e)),
    };

    let mut frames = 0;

    Outcome::Pending(Box::new(move || {
        frames += 1;

        let answer = *answer.lock().unwrap();

        if answer.is_none() && frames < 120 {
            return None;
        }

        let steps = steps.load(Ordering::SeqCst);

        Some(
            ensure(steps == 3, format!("the task counted {steps} frames"))
                .and_then(|_| ensure(answer == Some(42), format!("the join gave {answer:?}")))
                .and_then(|_| {
                    ensure(
                        dropped.load(Ordering::SeqCst),
                        "the task of a destroyed actor was not dropped",
                    )
                }),
        )
    }))
}

fn timers_fire_clear_and_free_their_closures(context: &Context) -> Outcome {
    let baseline = timer::live_timer_callbacks();

    let started = (|| {
        let actor = spawn(context)?;
        let e = |e| format!("{e:?}");
        let looping_count = Arc::new(AtomicUsize::new(0));
        let once_count = Arc::new(AtomicUsize::new(0));
        let event_count = Arc::new(AtomicUsize::new(0));

        let counter = looping_count.clone();
        let looping = timer::set_timer(&actor, Duration::from_millis(20), true, move || {
            counter.fetch_add(1, Ordering::SeqCst);
        })
        .map_err(e)?;

        let counter = once_count.clone();
        timer::set_timer(&actor, Duration::from_millis(20), false, move || {
            counter.fetch_add(1, Ordering::SeqCst);
        })
        .map_err(e)?;

        for _ in 0..50 {
            let counter = event_count.clone();
            KismetSystemLibrary::k2_set_timer_delegate(
                move || {
                    counter.fetch_add(1, Ordering::SeqCst);
                },
                0.01,
                false,
                None,
                None,
                None,
            );
        }

        Ok::<_, String>((looping, looping_count, once_count, event_count))
    })();

    let (looping, looping_count, once_count, event_count) = match started {
        Ok(started) => started,
        Err(e) => return done(Err(e)),
    };

    let mut frames = 0;
    let mut cleared_at: Option<(usize, usize)> = None;

    Outcome::Pending(Box::new(move || {
        frames += 1;

        let loops = looping_count.load(Ordering::SeqCst);
        let events = event_count.load(Ordering::SeqCst);

        match cleared_at {
            None => {
                if (loops < 3 || events < 50) && frames < 600 {
                    return None;
                }

                looping.clear();
                cleared_at = Some((loops, frames));
                None
            }
            Some((loops_then, frame_then)) => {
                if frames < frame_then + 20 {
                    return None;
                }

                let once = once_count.load(Ordering::SeqCst);
                let live = timer::live_timer_callbacks();

                Some(
                    ensure(
                        loops_then >= 3,
                        format!("the looping timer fired {loops_then} times"),
                    )
                    .and_then(|_| ensure(once == 1, format!("the one-shot fired {once} times")))
                    .and_then(|_| {
                        ensure(
                            events == 50,
                            format!("{events} of 50 timer delegates fired"),
                        )
                    })
                    .and_then(|_| {
                        ensure(loops == loops_then, "the looping timer fired after clear")
                    })
                    .and_then(|_| ensure(!looping.is_active(), "a cleared timer is active"))
                    .and_then(|_| {
                        ensure(
                            live == baseline,
                            format!("{live} timer closures alive, {baseline} before"),
                        )
                    }),
                )
            }
        }
    }))
}

fn sleep_follows_world_time_and_pause(context: &Context) -> Outcome {
    let started = (|| {
        let actor = spawn(context)?;
        let world_slept = Arc::new(Mutex::new(None));
        let paused_woke = Arc::new(AtomicBool::new(false));
        let real_woke = Arc::new(AtomicBool::new(false));
        let world: UObjectRef<bindings::core_ue::Object> = actor.upcast_to();

        let sink = world_slept.clone();
        let _measured = task::spawn_local(&actor, async move {
            let before = GameplayStatics::get_time_seconds(world);
            timer::sleep(&actor, Duration::from_millis(100)).await;
            *sink.lock().unwrap() = Some(GameplayStatics::get_time_seconds(world) - before);
        });

        Ok::<_, String>((actor, world, world_slept, paused_woke, real_woke))
    })();

    let (actor, world, world_slept, paused_woke, real_woke) = match started {
        Ok(started) => started,
        Err(e) => return done(Err(e)),
    };

    let mut frames = 0;
    let mut paused_at: Option<usize> = None;
    let mut unpaused_at: Option<usize> = None;

    Outcome::Pending(Box::new(move || {
        frames += 1;
        let slept = *world_slept.lock().unwrap();

        let Some(slept) = slept else {
            return (frames > 600).then(|| Err("the world sleep never woke".to_string()));
        };

        match (paused_at, unpaused_at) {
            (None, _) => {
                if !GameplayStatics::set_game_paused(world, true) {
                    return Some(Err("the game could not be paused".to_string()));
                }

                let sink = paused_woke.clone();
                let _paused = task::spawn_local(&actor, async move {
                    timer::sleep(&actor, Duration::from_millis(50)).await;
                    sink.store(true, Ordering::SeqCst);
                });

                let sink = real_woke.clone();
                let _real = task::spawn_local(&actor, async move {
                    timer::sleep_real(Duration::from_millis(50)).await;
                    sink.store(true, Ordering::SeqCst);
                });

                paused_at = Some(frames);
                None
            }
            (Some(at), None) => {
                if !real_woke.load(Ordering::SeqCst) && frames < at + 600 {
                    return None;
                }

                if frames < at + 30 {
                    return None;
                }

                let woke_while_paused = paused_woke.load(Ordering::SeqCst);
                GameplayStatics::set_game_paused(world, false);

                if woke_while_paused {
                    return Some(Err(
                        "a world sleep woke while the game was paused".to_string()
                    ));
                }

                if !real_woke.load(Ordering::SeqCst) {
                    return Some(Err("sleep_real did not wake while paused".to_string()));
                }

                unpaused_at = Some(frames);
                None
            }
            (Some(_), Some(at)) => {
                if !paused_woke.load(Ordering::SeqCst) && frames < at + 600 {
                    return None;
                }

                Some(
                    ensure(slept >= 0.099, format!("the world sleep lasted {slept} s")).and_then(
                        |_| {
                            ensure(
                                paused_woke.load(Ordering::SeqCst),
                                "the world sleep did not wake after unpausing",
                            )
                        },
                    ),
                )
            }
        }
    }))
}

#[derive(Default)]
struct LatentResults {
    waited: f64,
    loaded: bool,
    moved_to: Option<glam::DVec3>,
    error: Option<String>,
    finished: bool,
}

fn latent_engine_functions_resume_their_task(context: &Context) -> Outcome {
    let started = (|| {
        let actor = spawn(context)?;
        let results = Arc::new(Mutex::new(LatentResults::default()));
        let sink = results.clone();

        let _task = task::spawn_local(&actor, async move {
            let world: UObjectRef<bindings::core_ue::Object> = actor.upcast_to();

            let before = GameplayStatics::get_time_seconds(world);
            KismetSystemLibrary::delay(world, 0.2).await;
            let waited = GameplayStatics::get_time_seconds(world) - before;
            KismetSystemLibrary::delay_until_next_frame(world).await;

            let loaded = Arc::new(AtomicBool::new(false));
            let flag = loaded.clone();

            KismetSystemLibrary::load_asset(
                world,
                &SoftObjectRef::new("/Engine/BasicShapes/Cube.Cube"),
                move |object| flag.store(!object.raw().is_null(), Ordering::SeqCst),
            )
            .await;

            let moved = async {
                let root = TestActor::from_obj(actor)
                    .and_then(|actor| actor.root())
                    .map_err(|e| format!("{e:?}"))?;

                KismetSystemLibrary::move_component_to(
                    root,
                    &FVector::from_dvec3(glam::DVec3::new(0.0, 0.0, 300.0)),
                    &FRotator::from_rotator(Rotator::ZERO),
                    false,
                    false,
                    0.1,
                    false,
                    EMoveComponentAction::Move,
                )
                .await;

                let checked = root.checked().map_err(|e| format!("{e:?}"))?;
                Ok::<_, String>(checked.get_relative_location().to_dvec3())
            }
            .await;

            let mut results = sink.lock().unwrap();
            results.waited = waited;
            results.loaded = loaded.load(Ordering::SeqCst);

            match moved {
                Ok(at) => results.moved_to = Some(at),
                Err(e) => results.error = Some(e),
            }

            results.finished = true;
        });

        Ok::<_, String>(results)
    })();

    let results = match started {
        Ok(results) => results,
        Err(e) => return done(Err(e)),
    };

    let mut frames = 0;

    Outcome::Pending(Box::new(move || {
        frames += 1;
        let results = results.lock().unwrap();

        if !results.finished {
            return (frames > 1200).then(|| Err("the latent calls never finished".to_string()));
        }

        if let Some(e) = &results.error {
            return Some(Err(e.clone()));
        }

        let waited = results.waited;
        let loaded = results.loaded;
        let moved_to = results.moved_to;

        Some(
            ensure(waited >= 0.199, format!("delay(0.2) waited {waited} s"))
                .and_then(|_| ensure(loaded, "load_asset gave no object"))
                .and_then(|_| {
                    ensure(
                        moved_to.is_some_and(|at| (at.z - 300.0).abs() < 0.01),
                        format!("move_component_to ended at {moved_to:?}"),
                    )
                }),
        )
    }))
}

fn latent_action_of_a_destroyed_owner_is_cancelled(context: &Context) -> Outcome {
    let baseline = rusteal_runtime::runtime::latent::live_latent_proxies();

    let started = (|| {
        let doomed = spawn(context)?;
        let resumed = Arc::new(AtomicBool::new(false));
        let dropped = Arc::new(AtomicBool::new(false));
        let flag = resumed.clone();
        let guard = SetOnDrop(dropped.clone());

        let _task = task::spawn_local(&doomed, async move {
            let _guard = guard;
            KismetSystemLibrary::delay(doomed.upcast_to(), 10.0).await;
            flag.store(true, Ordering::SeqCst);
        });

        Ok::<_, String>((doomed, resumed, dropped))
    })();

    let (doomed, resumed, dropped) = match started {
        Ok(started) => started,
        Err(e) => return done(Err(e)),
    };

    let mut frames = 0;
    let mut destroyed_at: Option<rusteal_runtime::runtime::time::Instant> = None;

    Outcome::Pending(Box::new(move || {
        frames += 1;

        let Some(at) = destroyed_at else {
            if frames < 3 {
                return None;
            }

            if rusteal_runtime::runtime::latent::live_latent_proxies() <= baseline {
                return Some(Err("the delay made no latent proxy".to_string()));
            }

            if let Ok(actor) = doomed.checked() {
                actor.k2_destroy_actor();
            }

            destroyed_at = Some(rusteal_runtime::runtime::time::Instant::now());
            return None;
        };

        let live = rusteal_runtime::runtime::latent::live_latent_proxies();

        if live > baseline && at.elapsed() < Duration::from_secs(5) {
            return None;
        }

        Some(
            ensure(
                live <= baseline,
                format!("{live} latent proxies 5 s after the owner died, {baseline} before"),
            )
            .and_then(|_| ensure(dropped.load(Ordering::SeqCst), "the task was not dropped"))
            .and_then(|_| {
                ensure(
                    !resumed.load(Ordering::SeqCst),
                    "the cancelled task resumed",
                )
            }),
        )
    }))
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

const DEFAULT_MATERIAL: &str = "/Engine/EngineMaterials/DefaultMaterial.DefaultMaterial";

fn default_material() -> Result<UObjectRef<Material>, String> {
    SoftObjectRef::<Material>::new(DEFAULT_MATERIAL)
        .load_synchronous()
        .map_err(|e| format!("load: {e:?}"))
}

fn add_component<T: UeClass + Inherits<ActorComponent>>(
    context: &Context,
) -> Result<UObjectRef<T>, String> {
    let actor = spawn(context)?
        .checked()
        .map_err(|e| format!("checked: {e:?}"))?;
    let transform = FTransform::from_transform(Transform::IDENTITY);

    actor
        .add_component_by_class(
            SubclassOf::<T>::base().upcast_to::<ActorComponent>(),
            false,
            &transform,
            false,
        )
        .cast::<T>()
        .map_err(|e| format!("add component: {e:?}"))
}

fn engine_soft_property_keeps_the_path(context: &Context) -> Outcome {
    done((|| {
        let material = default_material()?;
        let cloud = add_component::<VolumetricCloudComponent>(context)?
            .checked()
            .map_err(|e| format!("checked: {e:?}"))?;

        cloud.set_material(material.upcast());

        let soft = cloud.get_material();
        ensure(
            soft.path() == DEFAULT_MATERIAL,
            format!("path {:?}", soft.path()),
        )?;
        ensure(
            soft.get().raw() == material.raw(),
            "the soft reference does not resolve to the material",
        )
    })())
}

fn engine_soft_reference_crosses_as_a_path(_: &Context) -> Outcome {
    done((|| {
        let material = default_material()?;
        let loaded =
            KismetSystemLibrary::load_asset_blocking(&SoftObjectRef::new(DEFAULT_MATERIAL));
        ensure(
            loaded.raw() == material.raw(),
            "LoadAsset_Blocking did not load the path",
        )?;

        let soft =
            KismetSystemLibrary::conv_object_to_soft_object_reference(material.upcast().upcast());
        ensure(
            soft.path() == DEFAULT_MATERIAL,
            format!("path {:?}", soft.path()),
        )
    })())
}

fn engine_weak_property_follows_its_object(context: &Context) -> Outcome {
    done((|| {
        let movement = add_component::<ProjectileMovementComponent>(context)?
            .checked()
            .map_err(|e| format!("checked: {e:?}"))?;
        let target = spawn(context)?;
        let root = TestActor::from_obj(target)
            .map_err(|e| format!("{e:?}"))?
            .root()
            .map_err(|e| format!("{e:?}"))?;

        movement.set_homing_target_component(TWeakObjectPtr::from_ref(&root));

        let followed = movement.get_homing_target_component().get();
        ensure(
            followed.map(|c| c.raw()) == Some(root.raw()),
            "the weak reference does not reach the component",
        )?;

        target
            .checked()
            .map_err(|e| format!("checked: {e:?}"))?
            .k2_destroy_actor();

        ensure(
            movement.get_homing_target_component().get().is_none(),
            "the weak reference outlived the destroyed component",
        )
    })())
}

fn engine_struct_with_an_array_returned_by_a_function(context: &Context) -> Outcome {
    done((|| {
        let material = default_material()?;
        let camera = add_component::<CameraComponent>(context)?
            .checked()
            .map_err(|e| format!("checked: {e:?}"))?;
        let blendable = material
            .cast::<BlendableInterface>()
            .map_err(|e| format!("cast: {e:?}"))?;

        camera.add_or_update_blendable(blendable, Some(0.25));

        let mut kept = Vec::new();

        for _ in 0..200 {
            let view = camera.get_camera_view(0.0);
            kept.push(view.clone());
        }

        for view in &kept {
            let settings = view.as_ref().get_post_process_settings();
            let blendables = settings.as_ref().get_weighted_blendables();
            let array = blendables.as_ref().array();
            let len = array.len().map_err(|e| format!("len: {e:?}"))?;
            ensure(len == 1, format!("{len} blendables"))?;
            let first = array.get(0).map_err(|e| format!("get: {e:?}"))?;
            ensure(
                first.as_ref().get_object().raw() == material.raw()
                    && first.as_ref().get_weight() == 0.25,
                "the returned view lost its blendable",
            )?;
        }

        Ok(())
    })())
}

fn engine_struct_passed_by_reference(_: &Context) -> Outcome {
    done((|| {
        let mut guid = KismetGuidLibrary::new_guid();
        ensure(
            KismetGuidLibrary::is_valid_guid(&guid),
            "a new guid is not valid",
        )?;

        KismetGuidLibrary::invalidate_guid(&mut guid);
        ensure(
            !KismetGuidLibrary::is_valid_guid(&guid),
            "invalidate_guid did not change the guid",
        )
    })())
}

const STATIC_MESH_ACTOR: &str = "/Script/Engine.StaticMeshActor";

fn engine_soft_class_reference_crosses_as_a_path(_: &Context) -> Outcome {
    done((|| {
        let loaded =
            KismetSystemLibrary::load_class_asset_blocking(&SoftClassRef::new(STATIC_MESH_ACTOR));
        ensure(
            loaded.raw() == StaticMeshActor::static_class(),
            "LoadClassAsset_Blocking did not load the class",
        )?;

        let typed = SoftClassRef::<Actor>::new(STATIC_MESH_ACTOR)
            .load_synchronous()
            .map_err(|e| format!("load: {e:?}"))?;
        ensure(
            typed.raw() == StaticMeshActor::static_class(),
            "SoftClassRef::load_synchronous gave another class",
        )?;

        let soft = KismetSystemLibrary::conv_class_to_soft_class_reference(
            SubclassOf::<StaticMeshActor>::base().upcast_to::<bindings::core_ue::Object>(),
        );
        ensure(
            soft.path() == STATIC_MESH_ACTOR,
            format!("path {:?}", soft.path()),
        )
    })())
}

fn engine_soft_class_property_keeps_the_path(_: &Context) -> Outcome {
    done((|| {
        let input = InputSettings::get_input_settings()
            .checked()
            .map_err(|e| format!("input settings: {e:?}"))?;
        let class: SoftClassRef<PlayerInput> = input.get_default_player_input_class();
        ensure(
            !class.is_null(),
            "the default player input class has no path",
        )?;

        let loaded = class
            .load_synchronous()
            .map_err(|e| format!("load {:?}: {e:?}", class.path()))?;
        ensure(
            !loaded.is_null(),
            format!("{:?} does not resolve to a class", class.path()),
        )
    })())
}

fn engine_text_round_trip(_: &Context) -> Outcome {
    done((|| {
        let text = KismetTextLibrary::conv_string_to_text("ação");
        ensure(text.to_string() == "ação", format!("got {text}"))?;

        let upper = KismetTextLibrary::text_to_upper(&text);
        ensure(upper.to_string() == "AÇÃO", format!("got {upper}"))?;
        ensure(
            KismetTextLibrary::conv_text_to_string("ñü") == "ñü",
            "Conv_TextToString lost the text",
        )
    })())
}

fn engine_text_keeps_its_localization(context: &Context) -> Outcome {
    done((|| {
        let text = Text::localized("RustealTests", "Greeting", "Hello");
        let (found, namespace, key) = KismetTextLibrary::get_text_id(&text);
        ensure(
            found && namespace == "RustealTests" && key == "Greeting",
            format!("id {found} {namespace:?} {key:?}"),
        )?;

        let label = add_component::<TextRenderComponent>(context)?
            .checked()
            .map_err(|e| format!("checked: {e:?}"))?;
        label.set_text(text.clone());

        let read = label.get_text();
        ensure(read == text, format!("read {read}"))?;
        let (found, namespace, key) = KismetTextLibrary::get_text_id(read);
        ensure(
            found && namespace == "RustealTests" && key == "Greeting",
            format!("after the property, id {found} {namespace:?} {key:?}"),
        )
    })())
}

fn engine_structs_without_static_struct(_: &Context) -> Outcome {
    done((|| {
        let span = KismetMathLibrary::from_minutes(1.5);
        let seconds = KismetMathLibrary::get_total_seconds(&span);
        ensure(seconds == 90.0, format!("{seconds} seconds"))?;

        let a = OwnedStruct::<FIntPoint>::new();
        a.as_ref().set_x(2);
        a.as_ref().set_y(3);
        let b = OwnedStruct::<FIntPoint>::new();
        b.as_ref().set_x(10);
        b.as_ref().set_y(20);

        let sum = KismetMathLibrary::add_int_point_int_point(&a, &b);
        ensure(
            (sum.as_ref().get_x(), sum.as_ref().get_y()) == (12, 23),
            format!("sum ({}, {})", sum.as_ref().get_x(), sum.as_ref().get_y()),
        )
    })())
}

fn instanced_struct_holds_a_struct(_: &Context) -> Outcome {
    done((|| {
        let guid = KismetGuidLibrary::new_guid();
        let instanced = FInstancedStruct::make(&guid);
        let held = instanced.as_ref();
        ensure(held.is_set(), "the instanced struct is empty")?;
        ensure(
            held.script_struct() == FGuid::static_struct(),
            "the instanced struct holds another type",
        )?;

        let back = held.get::<FGuid>().ok_or("get::<FGuid> gave nothing")?;
        ensure(
            KismetGuidLibrary::equal_equal_guid_guid(&guid, &back),
            "the guid changed",
        )?;
        ensure(
            held.get::<FVector>().is_none(),
            "get::<FVector> gave a value",
        )?;

        let copy = instanced.clone();
        held.reset();
        ensure(!held.is_set(), "reset left a value")?;
        ensure(
            copy.as_ref().get::<FGuid>().is_some(),
            "the clone lost its value with the original",
        )
    })())
}

fn engine_optional_fields(_: &Context) -> Outcome {
    done((|| {
        let toolbar = OwnedStruct::<FToolBarStyle>::new();
        let toolbar = toolbar.as_ref();
        ensure(
            toolbar.get_vertical_alignment_override().is_none(),
            "a new toolbar style has an alignment",
        )?;
        toolbar.set_vertical_alignment_override(Some(EVerticalAlignment::VAlign_Center));
        ensure(
            toolbar.get_vertical_alignment_override() == Some(EVerticalAlignment::VAlign_Center),
            "the alignment was not kept",
        )?;
        toolbar.set_vertical_alignment_override(None);
        ensure(
            toolbar.get_vertical_alignment_override().is_none(),
            "the alignment was not cleared",
        )?;

        let wrap = OwnedStruct::<FWrapButtonStyle>::new();
        let wrap = wrap.as_ref();
        wrap.set_separator_thickness(Some(2.5));
        ensure(
            wrap.get_separator_thickness() == Some(2.5),
            format!("thickness {:?}", wrap.get_separator_thickness()),
        )?;

        let brush = OwnedStruct::<FSlateBrush>::new();
        wrap.set_separator_brush(Some(&brush));
        ensure(
            wrap.get_separator_brush().is_some(),
            "the separator brush was not kept",
        )
    })())
}

fn engine_function_takes_an_interface(context: &Context) -> Outcome {
    done((|| {
        let material = default_material()?;
        let blendable = material
            .cast::<BlendableInterface>()
            .map_err(|e| format!("cast: {e:?}"))?;
        let transform = FTransform::from_transform(Transform::IDENTITY);
        let volume = context
            .world
            .spawn_actor::<PostProcessVolume>(&transform)
            .map_err(|e| format!("spawn: {e:?}"))?
            .checked()
            .map_err(|e| format!("checked: {e:?}"))?;

        volume.add_or_update_blendable(blendable, Some(0.5));

        let settings = volume.get_settings();
        let blendables = settings.as_ref().get_weighted_blendables();
        let array = blendables.as_ref().array();
        let len = array.len().map_err(|e| format!("len: {e:?}"))?;
        ensure(len == 1, format!("{len} blendables"))?;
        let first = array.get(0).map_err(|e| format!("get: {e:?}"))?;
        let object = first.as_ref().get_object();
        ensure(
            object.raw() == material.raw(),
            "the blendable is not the material",
        )?;
        ensure(
            first.as_ref().get_weight() == 0.5,
            format!("weight {}", first.as_ref().get_weight()),
        )
    })())
}

fn engine_function_takes_any_object_as_iinterface(context: &Context) -> Outcome {
    done((|| {
        let actor = spawn(context)?;
        let object = KismetSystemLibrary::conv_interface_to_object(actor.upcast().upcast());
        ensure(
            object.raw() == actor.raw(),
            "Conv_InterfaceToObject returned another object",
        )
    })())
}

fn engine_function_takes_a_closure(_: &Context) -> Outcome {
    let fired = Arc::new(Mutex::new(0));
    let sink = fired.clone();

    let _handle = KismetSystemLibrary::k2_set_timer_delegate(
        move || *sink.lock().unwrap() += 1,
        0.05,
        false,
        None,
        None,
        None,
    );

    KismetSystemLibrary::collect_garbage();

    Outcome::Pending(Box::new(move || {
        let count = *fired.lock().unwrap();
        (count > 0).then(|| ensure(count == 1, format!("the timer fired {count} times")))
    }))
}

fn engine_member_function_takes_a_closure_with_arguments(context: &Context) -> Outcome {
    let started = (|| {
        let timeline = add_component::<TimelineComponent>(context)?
            .checked()
            .map_err(|e| format!("checked: {e:?}"))?;
        let curve = rusteal_runtime::runtime::world::new_object_raw(
            timeline.raw(),
            CurveFloat::static_class(),
        )
        .and_then(UObjectRef::<CurveFloat>::from_handle)
        .map_err(|e| format!("curve: {e:?}"))?;
        let checked = curve.checked().map_err(|e| format!("checked: {e:?}"))?;

        let rich = checked.get_float_curve();
        let keys = rich.as_ref().keys();

        for (time, value) in [(0.0, 0.0), (0.1, 1.0)] {
            let key = OwnedStruct::<FRichCurveKey>::new();
            key.as_ref().set_time(time);
            key.as_ref().set_value(value);
            keys.push(&key).map_err(|e| format!("key: {e:?}"))?;
        }

        checked.set_float_curve(&rich);

        let seen = Arc::new(Mutex::new(-1.0f32));
        let sink = seen.clone();
        timeline.add_interp_float(
            curve,
            move |value| *sink.lock().unwrap() = value,
            None,
            None,
        );
        timeline.play();

        Ok::<_, String>(seen)
    })();

    let seen = match started {
        Ok(seen) => seen,
        Err(e) => return done(Err(e)),
    };

    Outcome::Pending(Box::new(move || {
        let value = *seen.lock().unwrap();
        (value >= 0.999).then_some(Ok(()))
    }))
}

fn test_actor(context: &Context) -> Result<TestActor, String> {
    TestActor::from_obj(spawn(context)?).map_err(|e| format!("{e:?}"))
}

fn rust_integer_properties(context: &Context) -> Outcome {
    done((|| {
        let actor = test_actor(context)?;
        ensure(
            actor.wide() == 40000,
            format!("default wide {}", actor.wide()),
        )?;
        ensure(
            actor.big() == 3_000_000_000,
            format!("default big {}", actor.big()),
        )?;

        actor.set_small(-5);
        actor.set_short(-12345);
        actor.set_huge(u64::MAX);
        ensure(actor.small() == -5, format!("small {}", actor.small()))?;
        ensure(actor.short() == -12345, format!("short {}", actor.short()))?;
        ensure(actor.huge() == u64::MAX, format!("huge {}", actor.huge()))
    })())
}

fn rust_text_and_string_properties(context: &Context) -> Outcome {
    done((|| {
        let actor = test_actor(context)?;
        ensure(
            actor.caption().to_string() == "Hello",
            format!("default caption {}", actor.caption()),
        )?;
        ensure(
            actor.title() == "Hero",
            format!("default title {}", actor.title()),
        )?;

        actor.set_caption(Text::localized("RustealTests", "Caption", "Over the actor"));
        let (found, namespace, key) = KismetTextLibrary::get_text_id(actor.caption());
        ensure(
            found && namespace == "RustealTests" && key == "Caption",
            format!("caption id {found} {namespace:?} {key:?}"),
        )
    })())
}

fn rust_reference_properties(context: &Context) -> Outcome {
    done((|| {
        let actor = test_actor(context)?;
        let other_ref = spawn(context)?;
        let other = TestActor::from_obj(other_ref).map_err(|e| format!("{e:?}"))?;

        actor.set_target(TWeakObjectPtr::from_ref(&other_ref.upcast()));
        ensure(
            actor.target().get().map(|a| a.raw()) == Some(other_ref.raw()),
            "the weak target does not reach the actor",
        )?;

        actor.set_spawn_class(&SoftClassRef::new(STATIC_MESH_ACTOR));
        ensure(
            actor.spawn_class().path() == STATIC_MESH_ACTOR,
            format!("spawn class {:?}", actor.spawn_class().path()),
        )?;

        ensure(actor.partner().is_none(), "a new partner is set")?;
        let partner = other_ref
            .interface::<dyn Pingable>()
            .map_err(|e| format!("interface: {e:?}"))?;
        actor.set_partner(Some(partner));
        ensure(
            actor.partner().map(|p| p.raw()) == Some(other_ref.raw()),
            "the partner was not kept",
        )?;
        actor.set_partner(None);
        ensure(actor.partner().is_none(), "the partner was not cleared")?;

        other_ref
            .checked()
            .map_err(|e| format!("checked: {e:?}"))?
            .k2_destroy_actor();
        let _ = other;
        ensure(
            actor.target().get().is_none(),
            "the weak target outlived the actor",
        )
    })())
}

fn rust_container_properties(context: &Context) -> Outcome {
    done((|| {
        let actor = test_actor(context)?;
        let e = |e| format!("{e:?}");

        let scores = actor.scores();
        scores.add(&"ação".to_string(), &1).map_err(e)?;
        scores.add(&"b".to_string(), &2).map_err(e)?;
        ensure(scores.len().map_err(e)? == 2, "two scores")?;
        ensure(
            scores.find(&"ação".to_string()).map_err(e)? == 1,
            "score of ação",
        )?;

        let seen = actor.seen();
        seen.add(&FName::new("x")).map_err(e)?;
        seen.add(&FName::new("x")).map_err(e)?;
        ensure(seen.len().map_err(e)? == 1, "a set keeps one x")?;
        ensure(seen.contains(&FName::new("x")).map_err(e)?, "the set has x")?;

        let long = "ñ-".repeat(4000);
        let names = actor.names();
        names.push(&long).map_err(e)?;
        names.push(&"short".to_string()).map_err(e)?;
        ensure(names.get(0).map_err(e)? == long, "a long string was cut")?;
        ensure(
            names.to_vec().map_err(e)? == vec![long.clone(), "short".to_string()],
            "to_vec lost a string",
        )?;

        let assets = actor.assets();
        assets
            .push(&SoftObjectRef::new(DEFAULT_MATERIAL))
            .map_err(e)?;
        ensure(
            assets.get(0).map_err(e)?.path() == DEFAULT_MATERIAL,
            "the soft reference lost its path",
        )?;

        let captions = actor.captions();
        captions
            .push(&Text::localized("RustealTests", "Line", "A line"))
            .map_err(e)?;
        let (found, _, key) = KismetTextLibrary::get_text_id(captions.get(0).map_err(e)?);
        ensure(found && key == "Line", format!("caption key {key:?}"))
    })())
}

fn every_other_property_kind_round_trips(context: &Context) -> Outcome {
    done((|| {
        let e = |e| format!("{e:?}");
        let actor = test_actor(context)?;
        let other = spawn(context)?;
        let mesh_class = SubclassOf::<StaticMeshActor>::base().upcast_to::<Actor>();

        actor.set_buddy(other.upcast());
        actor.set_buddy_class(mesh_class);
        actor.set_tag(FName::new("Leader"));
        actor.set_mood(Mood::Wary);
        actor.set_channel(ECollisionChannel::ECC_Camera);
        actor.set_skin(&SoftObjectRef::new(DEFAULT_MATERIAL));
        actor.set_spot(&FVector::from_dvec3(glam::DVec3::new(1.0, 2.0, 3.0)));
        actor.set_level(200);
        actor.set_elapsed(1.5e300);
        actor.set_serial(i64::MIN);

        let gear = OwnedStruct::<Loadout>::new();
        gear.as_ref().set_title("Medic");
        actor.set_gear(&gear);

        ensure(
            actor.buddy().raw() == other.raw(),
            "the object was not kept",
        )?;
        ensure(
            actor.buddy_class().raw() == mesh_class.raw(),
            "the class was not kept",
        )?;
        ensure(actor.tag() == "Leader", format!("tag {}", actor.tag()))?;
        ensure(
            actor.mood() == Mood::Wary,
            format!("mood {:?}", actor.mood()),
        )?;
        ensure(
            actor.channel() == ECollisionChannel::ECC_Camera,
            format!("channel {:?}", actor.channel()),
        )?;
        ensure(
            actor.skin().path() == DEFAULT_MATERIAL,
            "the soft path was lost",
        )?;
        ensure(
            actor.spot().to_dvec3() == glam::DVec3::new(1.0, 2.0, 3.0),
            format!("spot {:?}", actor.spot().to_dvec3()),
        )?;
        ensure(actor.level() == 200, format!("level {}", actor.level()))?;
        ensure(
            actor.elapsed() == 1.5e300,
            format!("elapsed {}", actor.elapsed()),
        )?;
        ensure(
            actor.serial() == i64::MIN,
            format!("serial {}", actor.serial()),
        )?;
        ensure(
            actor.gear().as_ref().title().to_string() == "Medic",
            "the struct was not copied",
        )?;

        let shown = OwnedStruct::<Loadout>::new();
        shown.as_ref().set_title("Scout");

        let mut call = DynamicCall::new(&actor.as_ref(), "Mirror").map_err(e)?;
        call.set_value("Buddy", &actor.as_ref().upcast())
            .map_err(e)?;
        call.set_value("Class", &SubclassOf::<Actor>::base())
            .map_err(e)?;
        call.set("Mood", Mood::Angry).map_err(e)?;
        call.set("Channel", ECollisionChannel::ECC_Visibility)
            .map_err(e)?;
        call.set_value(
            "Skin",
            &SoftObjectRef::<Material>::new("/Game/M_Skin.M_Skin"),
        )
        .map_err(e)?;
        call.set_value("Gear", &shown).map_err(e)?;
        call.set("Level", 7u8).map_err(e)?;
        call.set("Serial", -9i64).map_err(e)?;
        let echoed: String = call.call().map_err(e)?.value("ReturnValue").map_err(e)?;

        ensure(
            echoed == "Angry ECC_Visibility /Game/M_Skin.M_Skin 7 -9",
            format!("the function saw {echoed:?}"),
        )?;
        ensure(
            actor.buddy().raw() == actor.as_ref().raw(),
            "the object parameter",
        )?;
        ensure(
            actor.buddy_class().raw() == SubclassOf::<Actor>::base().raw(),
            "the class parameter",
        )?;
        ensure(actor.mood() == Mood::Angry, "the enum parameter")?;
        ensure(
            actor.gear().as_ref().title().to_string() == "Scout",
            "the struct parameter",
        )?;

        let quality = OwnedStruct::<FPerQualityLevelInt>::new();
        quality.as_ref().per_quality().add(&2, &40).map_err(e)?;
        let copy = quality.clone();
        ensure(
            copy.as_ref().per_quality().find(&2).map_err(e)? == 40,
            "an engine map field was not kept",
        )?;

        let filter = OwnedStruct::<FARFilter>::new();
        let set = filter.as_ref().recursive_classes_exclusion_set();
        set.add(&FName::new("Actor").handle()).map_err(e)?;
        set.add(&FName::new("Actor").handle()).map_err(e)?;
        ensure(
            set.len().map_err(e)? == 1,
            "an engine set field kept duplicates",
        )
    })())
}

fn rust_struct_fields(_: &Context) -> Outcome {
    done((|| {
        let e = |e| format!("{e:?}");
        let loadout = OwnedStruct::<Loadout>::new();
        let fields = loadout.as_ref();

        fields.set_title("Scout");
        fields.set_level(60000);
        fields.counts().add(&FName::new("Ammo"), &30).map_err(e)?;

        let copy = loadout.clone();
        drop(loadout);
        let fields = copy.as_ref();
        ensure(fields.title().to_string() == "Scout", "the title was lost")?;
        ensure(fields.level() == 60000, format!("level {}", fields.level()))?;
        ensure(
            fields.counts().find(&FName::new("Ammo")).map_err(e)? == 30,
            "the map was not copied with the struct",
        )
    })())
}

fn ufunction_takes_and_returns_any_type(context: &Context) -> Outcome {
    done((|| {
        let e = |e| format!("{e:?}");
        let actor = spawn(context)?;
        let other = spawn(context)?;

        let mut call = DynamicCall::new(&actor, "Greet").map_err(e)?;
        call.set_value("Name", &"Ada".to_string()).map_err(e)?;
        call.set_value("Title", &Text::from("Dr.")).map_err(e)?;
        let greeting: String = call.call().map_err(e)?.value("ReturnValue").map_err(e)?;
        ensure(greeting == "Dr. Ada", format!("greeting {greeting:?}"))?;

        let mut call = DynamicCall::new(&actor, "Tally").map_err(e)?;
        let words: Vec<String> = ["a", "b", "a"].map(String::from).to_vec();
        call.set_value("Words", &words).map_err(e)?;
        let counts: std::collections::HashMap<String, i32> =
            call.call().map_err(e)?.value("ReturnValue").map_err(e)?;
        ensure(
            counts.get("a") == Some(&2) && counts.get("b") == Some(&1),
            format!("counts {counts:?}"),
        )?;

        let mut call = DynamicCall::new(&actor, "Unique").map_err(e)?;
        let names: std::collections::HashSet<FName> =
            [FName::new("x"), FName::new("y")].into_iter().collect();
        call.set_value("Names", &names).map_err(e)?;
        let unique: u32 = call.call().map_err(e)?.value("ReturnValue").map_err(e)?;
        ensure(unique == 2, format!("unique {unique}"))?;

        let mut call = DynamicCall::new(&actor, "Widen").map_err(e)?;
        call.set_value("Value", &60000u16).map_err(e)?;
        call.set_value("Delta", &-7i8).map_err(e)?;
        let wide: u64 = call.call().map_err(e)?.value("ReturnValue").map_err(e)?;
        ensure(wide == 59993, format!("wide {wide}"))?;

        let mut call = DynamicCall::new(&actor, "CaptionFor").map_err(e)?;
        call.set_value("Key", &FName::new("Banner")).map_err(e)?;
        let caption: Text = call.call().map_err(e)?.value("ReturnValue").map_err(e)?;
        let (found, _, key) = KismetTextLibrary::get_text_id(&caption);
        ensure(found && key == "Banner", format!("caption key {key:?}"))?;

        let mut call = DynamicCall::new(&actor, "Scale").map_err(e)?;
        call.set_value(
            "Vector",
            &FVector::from_dvec3(rusteal_runtime::glam::DVec3::new(1.0, 2.0, 3.0)),
        )
        .map_err(e)?;
        call.set_value("By", &2.0f64).map_err(e)?;
        let scaled: OwnedStruct<FVector> =
            call.call().map_err(e)?.value("ReturnValue").map_err(e)?;
        ensure(
            scaled.to_dvec3() == rusteal_runtime::glam::DVec3::new(2.0, 4.0, 6.0),
            format!("scaled {:?}", scaled.to_dvec3()),
        )?;

        let mut call = DynamicCall::new(&actor, "HasPartner").map_err(e)?;
        let partner = other.interface::<dyn Pingable>().map_err(e)?;
        call.set_value("Partner", &Some(partner)).map_err(e)?;
        let has: bool = call.call().map_err(e)?.value("ReturnValue").map_err(e)?;
        ensure(has, "the partner did not arrive")?;

        let mut call = DynamicCall::new(&actor, "ClassPath").map_err(e)?;
        call.set_value("Class", &SoftClassRef::<Actor>::new(STATIC_MESH_ACTOR))
            .map_err(e)?;
        let path: String = call.call().map_err(e)?.value("ReturnValue").map_err(e)?;
        ensure(path == STATIC_MESH_ACTOR, format!("path {path:?}"))
    })())
}

fn interface_function_takes_any_type(context: &Context) -> Outcome {
    done((|| {
        let actor = spawn(context)?;
        let target = actor
            .interface::<dyn Pingable>()
            .map_err(|e| format!("{e:?}"))?;
        let text = target.describe("Ada".to_string(), vec![FName::new("a"), FName::new("b")]);
        ensure(text.to_string() == "Ada: 2", format!("described {text}"))
    })())
}

fn ufunction_out_parameters(context: &Context) -> Outcome {
    done((|| {
        let e = |e| format!("{e:?}");
        let actor = spawn(context)?;
        let mut call = DynamicCall::new(&actor, "SplitName").map_err(e)?;
        call.set_value("Full", &"Ada King Lovelace".to_string())
            .map_err(e)?;
        let result = call.call().map_err(e)?;

        let ok: bool = result.value("ReturnValue").map_err(e)?;
        let first: String = result.value("First").map_err(e)?;
        let rest: Vec<String> = result.value("Rest").map_err(e)?;
        let count: i32 = result.value("Count").map_err(e)?;

        ensure(
            ok && first == "Ada" && rest == ["King", "Lovelace"] && count == 3,
            format!("{ok} {first:?} {rest:?} {count}"),
        )
    })())
}

fn native_event_dispatches_to_the_override(context: &Context) -> Outcome {
    done((|| {
        let e = |e| format!("{e:?}");
        let actor = test_actor(context)?;
        ensure(
            actor.reaction(5) == 10,
            format!("native {}", actor.reaction(5)),
        )?;

        let transform = FTransform::from_transform(Transform::IDENTITY);
        let child = context
            .world
            .spawn_actor::<ChildTestActor>(&transform)
            .map_err(e)?;
        let as_parent = TestActor::from_obj(child.upcast()).map_err(e)?;
        ensure(
            as_parent.reaction(5) == 15,
            format!("the override was not called: {}", as_parent.reaction(5)),
        )?;
        ensure(
            as_parent.reaction_implementation(5) == 10,
            "the parent's implementation changed",
        )
    })())
}

fn override_takes_containers_and_soft_references(context: &Context) -> Outcome {
    done((|| {
        let e = |e| format!("{e:?}");
        let transform = FTransform::from_transform(Transform::IDENTITY);
        let child = context
            .world
            .spawn_actor::<ChildTestActor>(&transform)
            .map_err(e)?;
        let as_parent = TestActor::from_obj(child.upcast()).map_err(e)?;

        let summary = as_parent.summary(
            vec!["Sword".into(), "Shield".into()],
            SoftObjectRef::new("/Engine/BasicShapes/Cube.Cube"),
            HashMap::from([(FName::new("Gold"), 3), (FName::new("Iron"), 4)]),
        );

        ensure(
            summary == "Sword+Shield of /Engine/BasicShapes/Cube.Cube counting 7",
            format!("the override answered {summary:?}"),
        )
    })())
}

fn static_functions_of_a_library(_: &Context) -> Outcome {
    done((|| {
        let e = |e| format!("{e:?}");
        ensure(TestLibrary::add_three(1, 2, 3) == 6, "add_three from Rust")?;

        let library = SubclassOf::<TestLibrary>::base()
            .default_object()
            .map_err(e)?;

        let mut call = DynamicCall::new(&library, "AddThree").map_err(e)?;
        call.set_value("A", &1i32).map_err(e)?;
        call.set_value("B", &20i32).map_err(e)?;
        call.set_value("C", &300i32).map_err(e)?;
        let sum: i32 = call.call().map_err(e)?.value("ReturnValue").map_err(e)?;
        ensure(sum == 321, format!("sum {sum}"))?;

        let mut call = DynamicCall::new(&library, "Shout").map_err(e)?;
        call.set_value("Words", &"ação".to_string()).map_err(e)?;
        let loud: String = call.call().map_err(e)?.value("ReturnValue").map_err(e)?;
        ensure(loud == "AÇÃO", format!("loud {loud:?}"))
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
