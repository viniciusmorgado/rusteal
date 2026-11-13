//! Object references kept by index and serial number: a destroyed object reads as gone, and a new
//! object in its place is never taken for it.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use bindings::engine::{ActorExt, KismetSystemLibrary};
use bindings::enhanced_input::InputAction;
use bindings::manual::world_ext::new_object_transient;
use bindings::prelude::*;
use rusteal_runtime::runtime::{UObjectRef, method, timer};

use crate::fixtures::TestActor;
use crate::runner::{Context, Outcome};

fn spawn(context: &Context) -> Result<UObjectRef<TestActor>, String> {
    let transform = FTransform::from_transform(rusteal_runtime::runtime::Transform::IDENTITY);

    context
        .world
        .spawn_actor::<TestActor>(&transform)
        .map_err(|e| format!("spawn: {e:?}"))
}

fn destroy(actor: UObjectRef<TestActor>) -> Result<(), String> {
    actor
        .checked()
        .map_err(|e| format!("checked: {e:?}"))?
        .k2_destroy_actor();

    Ok(())
}

pub fn a_destroyed_object_is_invalid_after_collection(context: &Context) -> Outcome {
    let actor = match spawn(context).and_then(|actor| destroy(actor).map(|()| actor)) {
        Ok(actor) => actor,
        Err(e) => return Outcome::Done(Err(e)),
    };

    if actor.is_valid() {
        return Outcome::Done(Err("a destroyed actor is still valid".into()));
    }

    KismetSystemLibrary::collect_garbage();
    let mut frames = 0;

    Outcome::Pending(Box::new(move || {
        frames += 1;

        if frames < 3 {
            return None;
        }

        if actor.is_valid() || actor.checked().is_ok() || actor.raw() != Default::default() {
            return Some(Err(
                "a reference to a collected actor still resolves".into()
            ));
        }

        Some(Ok(()))
    }))
}

pub fn a_new_object_in_a_reused_slot_is_not_the_old_one(_context: &Context) -> Outcome {
    let old = match new_object_transient::<InputAction>() {
        Ok(old) => old,
        Err(e) => return Outcome::Done(Err(format!("new_object_transient: {e:?}"))),
    };

    let old_index = old.weak_handle().object_index;

    KismetSystemLibrary::collect_garbage();
    let mut frames = 0;

    Outcome::Pending(Box::new(move || {
        frames += 1;

        if frames < 3 {
            return None;
        }

        if old.is_valid() {
            return Some(Err(
                "an unreferenced object outlived a garbage collection".into(),
            ));
        }

        let mut kept = Vec::new();

        for _ in 0..512 {
            let new = match new_object_transient::<InputAction>() {
                Ok(new) => new,
                Err(e) => return Some(Err(format!("new_object_transient: {e:?}"))),
            };

            if new.weak_handle().object_index == old_index {
                if new == old || old.is_valid() || !new.is_valid() {
                    return Some(Err(
                        "the object in the old one's slot was taken for it".into()
                    ));
                }

                return Some(Ok(()));
            }

            kept.push(new);
        }

        // The engine did not hand the slot back within 512 objects: the old reference staying
        // invalid is all there is to check.
        Some(Ok(()))
    }))
}

pub fn a_method_callback_skips_a_destroyed_object(context: &Context) -> Outcome {
    let hits = Arc::new(AtomicUsize::new(0));
    let counted = hits.clone();

    let callback = match spawn(context).and_then(|actor| {
        let this = TestActor::from_obj(actor).map_err(|e| format!("{e:?}"))?;
        let callback = method(&this, move |_: &TestActor| {
            counted.fetch_add(1, Ordering::Relaxed);
        });

        destroy(actor)?;

        Ok(callback)
    }) {
        Ok(callback) => callback,
        Err(e) => return Outcome::Done(Err(e)),
    };

    KismetSystemLibrary::collect_garbage();

    // New actors of the same class, any of which may land where the destroyed one was.
    for _ in 0..8 {
        if let Err(e) = spawn(context) {
            return Outcome::Done(Err(e));
        }
    }

    let timer = match timer::set_timer(&context.world, Duration::from_millis(1), true, callback) {
        Ok(timer) => timer,
        Err(e) => return Outcome::Done(Err(format!("set_timer: {e:?}"))),
    };

    let mut frames = 0;

    Outcome::Pending(Box::new(move || {
        frames += 1;

        if frames < 5 {
            return None;
        }

        timer.clear();
        let calls = hits.load(Ordering::Relaxed);

        Some(if calls == 0 {
            Ok(())
        } else {
            Err(format!(
                "a method bound to a destroyed actor ran {calls} times"
            ))
        })
    }))
}
