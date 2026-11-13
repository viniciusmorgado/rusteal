use rusteal_core::tick::{TickFunction, add_tick_function_raw};
use rusteal_core::{RustealResult, UObjectRef, UeClass};

use crate::engine::ETickingGroup;

/// Calls `callback` with the frame's delta time in `group` of the world of `object`, every frame
/// until the tick function is removed or `object` is destroyed; `add_prerequisite` orders it after
/// an actor's or a component's tick.
#[track_caller]
pub fn add_tick_function(
    object: &UObjectRef<impl UeClass>,
    group: ETickingGroup,
    callback: impl FnMut(f32) + Send + 'static,
) -> RustealResult<TickFunction> {
    add_tick_function_raw(object, group as u8, callback)
}
