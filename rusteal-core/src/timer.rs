use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use rusteal_ffi::FWeakObjectHandle;

use crate::delegate_registry::{register_callback, unregister_callback};
use crate::error::{RustealResult, check_ffi};
use crate::ffi_dispatch;
use crate::lock_or_recover;
use crate::object_ref::UObjectRef;
use crate::sync::Mutex;
use crate::traits::UeClass;

/// A timer of a world's timer manager, set with [`set_timer`] or [`set_timer_for_next_tick`].
///
/// Dropping the handle leaves the timer running, as a Blueprint timer does: [`TimerHandle::clear`]
/// stops it. The closure is freed when the timer is cleared, when a one-shot timer has fired, or
/// when its world goes away.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimerHandle {
    context: Option<FWeakObjectHandle>,
    handle: u64,
}

impl TimerHandle {
    /// The engine's `FTimerHandle`, as its 8 bytes.
    pub fn raw(&self) -> u64 {
        self.handle
    }

    fn context(&self) -> Option<rusteal_ffi::UObjectHandle> {
        match self.context {
            None => Some(rusteal_ffi::UObjectHandle::null()),
            Some(weak) => {
                let object = unsafe { ffi_dispatch::core_resolve_weak(weak) };
                (!object.is_null()).then_some(object)
            }
        }
    }

    pub fn clear(&self) {
        if let Some(context) = self.context() {
            let _ = unsafe { ffi_dispatch::world_clear_timer(context, self.handle) };
        }
    }

    #[track_caller]
    pub fn pause(&self) -> RustealResult<()> {
        self.set_paused(true)
    }

    #[track_caller]
    pub fn unpause(&self) -> RustealResult<()> {
        self.set_paused(false)
    }

    #[track_caller]
    fn set_paused(&self, paused: bool) -> RustealResult<()> {
        let context = self
            .context()
            .ok_or_else(crate::error::RustealError::object_destroyed)?;

        check_ffi(unsafe { ffi_dispatch::world_pause_timer(context, self.handle, paused) })
    }

    /// The time left before it fires, or `None` once it has stopped.
    pub fn remaining(&self) -> Option<Duration> {
        let context = self.context()?;
        let seconds = unsafe { ffi_dispatch::world_timer_remaining(context, self.handle) };
        (seconds >= 0.0).then(|| Duration::from_secs_f32(seconds))
    }

    pub fn is_active(&self) -> bool {
        self.remaining().is_some()
    }
}

/// Calls `callback` after `delay` in the world of `context`, and every `delay` after that when
/// `looping`; the time is the world's, so it stops while the game is paused.
#[track_caller]
pub fn set_timer(
    context: &UObjectRef<impl UeClass>,
    delay: Duration,
    looping: bool,
    callback: impl FnMut() + Send + 'static,
) -> RustealResult<TimerHandle> {
    let context = context.checked()?.raw();
    set_timer_raw(context, delay.as_secs_f32(), looping, false, -1.0, callback)
}

/// [`set_timer`] with a first call after `first_delay` instead of `delay`.
#[track_caller]
pub fn set_timer_with_first_delay(
    context: &UObjectRef<impl UeClass>,
    delay: Duration,
    looping: bool,
    first_delay: Option<Duration>,
    callback: impl FnMut() + Send + 'static,
) -> RustealResult<TimerHandle> {
    let context = context.checked()?.raw();
    let first = first_delay.map_or(-1.0, |d| d.as_secs_f32());
    set_timer_raw(
        context,
        delay.as_secs_f32(),
        looping,
        false,
        first,
        callback,
    )
}

/// A timer in the world of `context`, or of the current world when `context` is null; the engine
/// functions that take a timer delegate are built on it.
#[doc(hidden)]
#[track_caller]
pub fn set_timer_raw(
    context: rusteal_ffi::UObjectHandle,
    rate: f32,
    looping: bool,
    max_once_per_frame: bool,
    first_delay: f32,
    mut callback: impl FnMut() + Send + 'static,
) -> RustealResult<TimerHandle> {
    let id = register_callback(move |_| callback());
    let mut handle = 0u64;

    let result = check_ffi(unsafe {
        ffi_dispatch::world_set_timer(
            context,
            rate,
            looping,
            max_once_per_frame,
            first_delay,
            id,
            &mut handle,
        )
    });

    if let Err(e) = result {
        unregister_callback(id);
        return Err(e);
    }

    Ok(TimerHandle {
        context: weak_or_null(context),
        handle,
    })
}

fn weak_or_null(context: rusteal_ffi::UObjectHandle) -> Option<FWeakObjectHandle> {
    (!context.is_null()).then(|| unsafe { ffi_dispatch::core_make_weak(context) })
}

/// Calls `callback` once, on the next frame of the world of `context`.
#[track_caller]
pub fn set_timer_for_next_tick(
    context: &UObjectRef<impl UeClass>,
    callback: impl FnOnce() + Send + 'static,
) -> RustealResult<TimerHandle> {
    set_timer_for_next_tick_raw(context.checked()?.raw(), callback)
}

#[doc(hidden)]
#[track_caller]
pub fn set_timer_for_next_tick_raw(
    context: rusteal_ffi::UObjectHandle,
    callback: impl FnOnce() + Send + 'static,
) -> RustealResult<TimerHandle> {
    let mut callback = Some(callback);

    let id = register_callback(move |_| {
        if let Some(callback) = callback.take() {
            callback();
        }
    });

    let mut handle = 0u64;

    let result =
        check_ffi(unsafe { ffi_dispatch::world_set_timer_for_next_tick(context, id, &mut handle) });

    if let Err(e) = result {
        unregister_callback(id);
        return Err(e);
    }

    Ok(TimerHandle {
        context: weak_or_null(context),
        handle,
    })
}

#[doc(hidden)]
pub fn live_timer_callbacks() -> u32 {
    unsafe { ffi_dispatch::world_live_timer_callbacks() }
}

#[derive(Default)]
struct SleepState {
    fired: bool,
    waker: Option<Waker>,
}

struct Fire(Arc<Mutex<SleepState>>);

impl Fire {
    fn fire(&self) {
        let waker = {
            let mut state = lock_or_recover(&self.0);
            state.fired = true;
            state.waker.take()
        };

        if let Some(waker) = waker {
            waker.wake();
        }
    }
}

impl Drop for Fire {
    fn drop(&mut self) {
        self.fire();
    }
}

/// Waits `duration` of the world's time of `context`: it stops while the game is paused and
/// follows time dilation. Await it in a task started with `task::spawn_local`.
pub fn sleep(context: &UObjectRef<impl UeClass>, duration: Duration) -> Sleep {
    let state = Arc::new(Mutex::new(SleepState::default()));
    let fire = Fire(state.clone());

    let timer = if duration.is_zero() {
        set_timer_for_next_tick(context, move || fire.fire())
    } else {
        set_timer(context, duration, false, move || fire.fire())
    };

    Sleep {
        state,
        timer: timer.ok(),
    }
}

/// The future of [`sleep`]; dropping it clears its timer.
pub struct Sleep {
    state: Arc<Mutex<SleepState>>,
    timer: Option<TimerHandle>,
}

impl Future for Sleep {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.timer.is_none() {
            return Poll::Ready(());
        }

        let mut state = lock_or_recover(&self.state);

        if state.fired {
            return Poll::Ready(());
        }

        state.waker = Some(cx.waker().clone());
        Poll::Pending
    }
}

impl Drop for Sleep {
    fn drop(&mut self) {
        if let Some(timer) = self.timer.take()
            && !lock_or_recover(&self.state).fired
        {
            timer.clear();
        }
    }
}

/// Waits `duration` of the engine's clock, which goes on while the game is paused.
pub fn sleep_real(duration: Duration) -> SleepReal {
    SleepReal {
        until: crate::time::Instant::now() + duration,
    }
}

/// The future of [`sleep_real`].
pub struct SleepReal {
    until: crate::time::Instant,
}

impl Future for SleepReal {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if crate::time::Instant::now()
            .checked_duration_since(self.until)
            .is_some()
        {
            return Poll::Ready(());
        }

        cx.waker().wake_by_ref();
        Poll::Pending
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::module::{FuncTable, Module};
    use crate::task::{spawn_local_unowned, tick};
    use std::cell::Cell;
    use std::rc::Rc;
    use std::time::Instant;

    static FUNCS: FuncTable = FuncTable::new(0, 0);
    static REAL: Module = Module::new("timer_real", "", &FUNCS);

    #[test]
    fn sleep_real_resumes_after_its_duration() {
        let _scope = REAL.enter();
        let woke = Rc::new(Cell::new(false));
        let sink = woke.clone();

        let _task = spawn_local_unowned(async move {
            sleep_real(Duration::from_millis(20)).await;
            sink.set(true);
        });

        let started = Instant::now();

        while !woke.get() {
            assert!(
                started.elapsed() < Duration::from_secs(5),
                "sleep_real never resumed"
            );
            tick(0.0);
            std::thread::sleep(Duration::from_millis(1));
        }

        assert!(started.elapsed() >= Duration::from_millis(20));
    }
}
