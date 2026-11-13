use std::time::Duration;

use crate::delegate_registry::{register_callback, unregister_callback};
use crate::error::{RustealResult, check_ffi};
use crate::ffi_dispatch;
use crate::object_ref::UObjectRef;
use crate::traits::UeClass;

/// A tick function of its own, registered in a tick group of the world of the object it was added
/// for, with [`add_tick_function_raw`] (the bindings' `add_tick_function` takes the group as an
/// `ETickingGroup`).
///
/// Dropping it leaves it ticking; [`TickFunction::remove`] stops it. It is removed by itself when
/// its object is destroyed, when its world goes away, or when the library unloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TickFunction {
    id: u64,
}

impl TickFunction {
    pub fn remove(&self) {
        unsafe { ffi_dispatch::world_remove_tick_function(self.id) };
    }

    #[track_caller]
    pub fn set_enabled(&self, enabled: bool) -> RustealResult<()> {
        check_ffi(unsafe { ffi_dispatch::world_tick_function_set_enabled(self.id, enabled) })
    }

    /// Ticks every `interval` instead of every frame; zero is every frame.
    #[track_caller]
    pub fn set_interval(&self, interval: Duration) -> RustealResult<()> {
        check_ffi(unsafe {
            ffi_dispatch::world_tick_function_set_interval(self.id, interval.as_secs_f32())
        })
    }

    /// Ticks after the primary tick of `object`, an actor or an actor component, in a frame where
    /// both tick.
    #[track_caller]
    pub fn add_prerequisite(&self, object: &UObjectRef<impl UeClass>) -> RustealResult<()> {
        let object = object.checked()?.raw();

        check_ffi(unsafe { ffi_dispatch::world_tick_function_add_prerequisite(self.id, object, 0) })
    }

    /// Ticks after `other`, in a frame where both tick.
    #[track_caller]
    pub fn add_prerequisite_function(&self, other: &TickFunction) -> RustealResult<()> {
        check_ffi(unsafe {
            ffi_dispatch::world_tick_function_add_prerequisite(
                self.id,
                rusteal_ffi::UObjectHandle::null(),
                other.id,
            )
        })
    }
}

#[doc(hidden)]
#[track_caller]
pub fn add_tick_function_raw(
    object: &UObjectRef<impl UeClass>,
    group: u8,
    mut callback: impl FnMut(f32) + Send + 'static,
) -> RustealResult<TickFunction> {
    let object = object.checked()?.raw();

    let id = register_callback(move |params| {
        let delta = if params.is_null() {
            0.0
        } else {
            unsafe { *(params as *const f32) }
        };

        callback(delta);
    });

    if let Err(e) = check_ffi(unsafe { ffi_dispatch::world_add_tick_function(object, group, id) }) {
        unregister_callback(id);
        return Err(e);
    }

    Ok(TickFunction { id })
}

#[doc(hidden)]
pub fn live_tick_functions() -> u32 {
    unsafe { ffi_dispatch::world_live_tick_functions() }
}
