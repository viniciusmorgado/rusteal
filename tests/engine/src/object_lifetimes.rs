//! Object references kept by index and serial number: a destroyed object reads as gone, and a new
//! object in its place is never taken for it.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use bindings::engine::{ActorExt, FLatentActionInfo, FLatentActionInfoExt, KismetSystemLibrary};
use bindings::enhanced_input::InputAction;
use bindings::manual::world_ext::new_object_transient;
use bindings::prelude::*;
use rusteal_runtime::runtime::TWeakObjectPtr;
use rusteal_runtime::runtime::{GameThread, UObjectRef, method, task, timer};

use crate::fixtures::TestActor;
use crate::runner::{Context, Outcome};

fn spawn<'a>(context: &Context<'a>) -> Result<UObjectRef<'a, TestActor>, String> {
    let transform = FTransform::from_transform(rusteal_runtime::runtime::Transform::IDENTITY);

    context
        .world
        .spawn_actor::<TestActor>(&transform)
        .map_err(|e| format!("spawn: {e:?}"))
}

fn destroy(actor: UObjectRef<'_, TestActor>) -> Result<(), String> {
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

    // The check runs on in later frames: it keeps the actor as a weak pointer.
    let actor = TWeakObjectPtr::from_ref(&actor);

    KismetSystemLibrary::collect_garbage();
    let mut frames = 0;

    Outcome::Pending(Box::new(move || {
        frames += 1;

        if frames < 3 {
            return None;
        }

        if actor.is_valid() || actor.checked().is_ok() || actor.get().is_some() {
            return Some(Err("a reference to a collected actor still resolves".into()));
        }

        Some(Ok(()))
    }))
}

pub fn a_new_object_in_a_reused_slot_is_not_the_old_one(_context: &Context) -> Outcome {
    let created = GameThread::current().and_then(|gt| {
        new_object_transient::<InputAction>(&gt).map(|old| TWeakObjectPtr::from_ref(&old))
    });

    let old = match created {
        Ok(old) => old,
        Err(e) => return Outcome::Done(Err(format!("new_object_transient: {e:?}"))),
    };

    let old_index = old.handle().object_index;

    KismetSystemLibrary::collect_garbage();
    let mut frames = 0;

    Outcome::Pending(Box::new(move || {
        frames += 1;

        if frames < 3 {
            return None;
        }

        if old.is_valid() {
            return Some(Err(
                "an unreferenced object outlived a garbage collection".into()
            ));
        }

        let gt = match GameThread::current() {
            Ok(gt) => gt,
            Err(e) => return Some(Err(format!("{e:?}"))),
        };

        let mut kept = Vec::new();

        for _ in 0..512 {
            let new = match new_object_transient::<InputAction>(&gt) {
                Ok(new) => new,
                Err(e) => return Some(Err(format!("new_object_transient: {e:?}"))),
            };

            if new.weak_handle().object_index == old_index {
                if new.weak_handle() == old.handle() || old.is_valid() || !new.is_valid() {
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
        let callback = method(&*this, move |_: &TestActor| {
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

pub fn an_engine_call_off_the_game_thread_is_refused(_context: &Context) -> Outcome {
    let refused = Arc::new(AtomicUsize::new(0));
    let answer = refused.clone();

    task::spawn(
        || std::panic::catch_unwind(|| KismetSystemLibrary::get_engine_version()).is_err(),
        move |panicked| answer.store(if panicked { 1 } else { 2 }, Ordering::Relaxed),
    );

    let mut frames = 0;

    Outcome::Pending(Box::new(move || {
        frames += 1;

        match refused.load(Ordering::Relaxed) {
            0 if frames < 200 => None,
            0 => Some(Err("the worker never came back".into())),
            // Release builds leave the check out.
            1 => Some(Ok(())),
            _ if !cfg!(debug_assertions) => Some(Ok(())),
            _ => Some(Err(
                "a debug build let a worker thread call the engine".into()
            )),
        }
    }))
}

pub fn a_weak_pointer_comes_back_to_the_game_thread(context: &Context) -> Outcome {
    let actor = match spawn(context) {
        Ok(actor) => actor,
        Err(e) => return Outcome::Done(Err(e)),
    };

    let weak = TWeakObjectPtr::from_ref(&actor);
    let seen = Arc::new(AtomicUsize::new(0));
    let answer = seen.clone();

    task::spawn(
        // The weak pointer crosses threads; the object it names stays where it is.
        move || weak,
        move |weak| {
            let alive = weak.get().is_some_and(|actor| actor.checked().is_ok());
            answer.store(if alive { 1 } else { 2 }, Ordering::Relaxed);
        },
    );

    let mut frames = 0;

    Outcome::Pending(Box::new(move || {
        frames += 1;

        match seen.load(Ordering::Relaxed) {
            0 if frames < 200 => None,
            0 => Some(Err("the worker never came back".into())),
            1 => Some(Ok(())),
            _ => Some(Err(
                "the weak pointer did not resolve back on the game thread".into(),
            )),
        }
    }))
}

/// Frames waited after a garbage collection before looking at what it kept.
const AFTER_COLLECTION: usize = 3;

pub fn two_pins_keep_an_object_each(_context: &Context) -> Outcome {
    let pinned = GameThread::current().and_then(|gt| {
        let object = new_object_transient::<InputAction>(&gt)?;
        Ok((
            object.pin()?,
            object.pin()?,
            TWeakObjectPtr::from_ref(&object),
        ))
    });

    let (first, second, weak) = match pinned {
        Ok(pinned) => pinned,
        Err(e) => return Outcome::Done(Err(format!("{e:?}"))),
    };

    drop(first);
    KismetSystemLibrary::collect_garbage();

    let mut second = Some(second);
    let mut frames = 0;
    let mut released_at = None;

    Outcome::Pending(Box::new(move || {
        frames += 1;

        if frames < AFTER_COLLECTION {
            return None;
        }

        match released_at {
            None => {
                if !weak.is_valid() {
                    return Some(Err("dropping one of two pins let the object go".into()));
                }

                second = None;
                KismetSystemLibrary::collect_garbage();
                released_at = Some(frames);
                None
            }
            Some(at) if frames < at + AFTER_COLLECTION => None,
            Some(_) => Some(if weak.is_valid() {
                Err("the object outlived its last pin".into())
            } else {
                drop(second.take());
                Ok(())
            }),
        }
    }))
}

pub fn an_owned_struct_keeps_its_objects(_context: &Context) -> Outcome {
    let kept = GameThread::current().and_then(|gt| {
        let object = new_object_transient::<InputAction>(&gt)?;
        let info = rusteal_runtime::runtime::OwnedStruct::<FLatentActionInfo>::new();
        info.as_ref().set_callback_target(object.upcast_to());
        Ok((info, TWeakObjectPtr::from_ref(&object)))
    });

    let (info, weak) = match kept {
        Ok(kept) => kept,
        Err(e) => return Outcome::Done(Err(format!("{e:?}"))),
    };

    KismetSystemLibrary::collect_garbage();

    let mut info = Some(info);
    let mut frames = 0;
    let mut released_at = None;

    Outcome::Pending(Box::new(move || {
        frames += 1;

        if frames < AFTER_COLLECTION {
            return None;
        }

        match released_at {
            None => {
                if !weak.is_valid() {
                    return Some(Err(
                        "an object held only by an owned struct was collected".into()
                    ));
                }

                info = None;
                KismetSystemLibrary::collect_garbage();
                released_at = Some(frames);
                None
            }
            Some(at) if frames < at + AFTER_COLLECTION => None,
            Some(_) => Some(if weak.is_valid() {
                Err("the object outlived the owned struct that held it".into())
            } else {
                drop(info.take());
                Ok(())
            }),
        }
    }))
}

pub fn a_pinned_actor_destroyed_reads_as_gone(context: &Context) -> Outcome {
    let pinned = spawn(context).and_then(|actor| {
        let pinned = actor.pin().map_err(|e| format!("pin: {e:?}"))?;
        destroy(actor)?;
        Ok(pinned)
    });

    let pinned = match pinned {
        Ok(pinned) => pinned,
        Err(e) => return Outcome::Done(Err(e)),
    };

    KismetSystemLibrary::collect_garbage();
    let mut frames = 0;

    Outcome::Pending(Box::new(move || {
        frames += 1;

        if frames < AFTER_COLLECTION {
            return None;
        }

        Some(if pinned.is_alive() || pinned.checked().is_ok() {
            Err("a pinned actor destroyed explicitly still reads as alive".into())
        } else {
            Ok(())
        })
    }))
}

pub fn unpinning_leaves_what_the_engine_keeps(context: &Context) -> Outcome {
    let world = TWeakObjectPtr::from_ref(&context.world);

    match context.world.pin() {
        Ok(pinned) => drop(pinned),
        Err(e) => return Outcome::Done(Err(format!("pin: {e:?}"))),
    }

    KismetSystemLibrary::collect_garbage();
    let mut frames = 0;

    Outcome::Pending(Box::new(move || {
        frames += 1;

        if frames < AFTER_COLLECTION {
            return None;
        }

        Some(if world.is_valid() {
            Ok(())
        } else {
            Err("dropping a pin of the world let the engine's world go".into())
        })
    }))
}
