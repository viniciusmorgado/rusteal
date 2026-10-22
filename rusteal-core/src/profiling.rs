//! Rust code in Unreal's profilers: CPU scopes in Unreal Insights, cycle stats in `stat Rusteal`,
//! and memory tags in the low-level memory tracker (`-llm`).

use std::marker::PhantomData;

use crate::ffi_dispatch;

/// A CPU event in Unreal Insights' timing view, from its creation to its drop, on the thread
/// that made it. [`trace_scope!`](crate::trace_scope) makes one with a name registered once.
#[must_use = "the event ends when the scope is dropped"]
pub struct CpuScope {
    _thread: PhantomData<*const ()>,
}

impl CpuScope {
    /// An event named `name`; [`trace_scope!`](crate::trace_scope) is cheaper for a fixed name.
    pub fn new(name: &str) -> Self {
        Self::of_type(event_type(name))
    }

    #[doc(hidden)]
    pub fn of_type(event_type: u32) -> Self {
        unsafe { ffi_dispatch::world_trace_begin_event(event_type) };

        CpuScope {
            _thread: PhantomData,
        }
    }
}

impl Drop for CpuScope {
    fn drop(&mut self) {
        unsafe { ffi_dispatch::world_trace_end_event() };
    }
}

#[doc(hidden)]
pub fn event_type(name: &str) -> u32 {
    unsafe { ffi_dispatch::world_trace_event_type(name.as_ptr(), name.len() as u32) }
}

/// Times the rest of the block as a CPU event named `$name` in Unreal Insights:
/// `trace_scope!("Pathfinding");`. The name is registered with the trace the first time.
#[macro_export]
macro_rules! trace_scope {
    ($name:expr) => {
        let __rusteal_trace_scope = {
            static EVENT_TYPE: ::std::sync::OnceLock<u32> = ::std::sync::OnceLock::new();
            $crate::profiling::CpuScope::of_type(
                *EVENT_TYPE.get_or_init(|| $crate::profiling::event_type($name)),
            )
        };
    };
}

/// A cycle stat of the `Rusteal` stat group, which `stat Rusteal` shows; make it once and keep it.
#[derive(Clone, Copy, Debug)]
pub struct Stat(u64);

impl Stat {
    pub fn new(name: &str) -> Self {
        Stat(unsafe { ffi_dispatch::world_stat_create(name.as_ptr(), name.len() as u32) })
    }

    /// Counts the time until the returned scope is dropped.
    pub fn scope(self) -> StatScope {
        StatScope {
            counter: unsafe { ffi_dispatch::world_stat_begin(self.0) },
            _thread: PhantomData,
        }
    }
}

#[must_use = "the stat stops counting when the scope is dropped"]
pub struct StatScope {
    counter: u64,
    _thread: PhantomData<*const ()>,
}

impl Drop for StatScope {
    fn drop(&mut self) {
        unsafe { ffi_dispatch::world_stat_end(self.counter) };
    }
}

/// Counts the rest of the block in the stat `$name` of `stat Rusteal`: `stat_scope!("AI tick");`.
#[macro_export]
macro_rules! stat_scope {
    ($name:expr) => {
        let __rusteal_stat_scope = {
            static STAT: ::std::sync::OnceLock<$crate::profiling::Stat> =
                ::std::sync::OnceLock::new();
            STAT.get_or_init(|| $crate::profiling::Stat::new($name))
                .scope()
        };
    };
}

/// Puts the engine allocations made until the returned scope is dropped under the tag `tag` of
/// the low-level memory tracker, when the game runs with `-llm`.
pub fn memory_tag(tag: &str) -> MemoryScope {
    MemoryScope {
        scope: unsafe { ffi_dispatch::world_memory_scope_begin(tag.as_ptr(), tag.len() as u32) },
        _thread: PhantomData,
    }
}

#[must_use = "the tag ends when the scope is dropped"]
pub struct MemoryScope {
    scope: u64,
    _thread: PhantomData<*const ()>,
}

impl Drop for MemoryScope {
    fn drop(&mut self) {
        if self.scope != 0 {
            unsafe { ffi_dispatch::world_memory_scope_end(self.scope) };
        }
    }
}
