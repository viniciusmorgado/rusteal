use rusteal_core::OwnedStruct;
use rusteal_core::timer::{TimerHandle, set_timer_for_next_tick_raw, set_timer_raw};

use crate::engine::{FTimerHandle, KismetSystemLibrary};

fn random_in(variance: f64) -> f64 {
    let seed = rusteal_core::time::now_seconds().to_bits();
    let mixed = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15).rotate_left(29);
    let unit = (mixed >> 11) as f64 / (1u64 << 53) as f64;
    (unit * 2.0 - 1.0) * variance
}

fn engine_handle(timer: rusteal_core::RustealResult<TimerHandle>) -> OwnedStruct<FTimerHandle> {
    let raw = match timer {
        Ok(timer) => timer.raw(),
        Err(e) => {
            e.__log_in(rusteal_core::LOG_WARNING, "setting a timer");
            0
        }
    };

    OwnedStruct::from_bytes(raw.to_ne_bytes().to_vec())
}

impl KismetSystemLibrary {
    /// Blueprint's **Set Timer by Event**, calling `delegate`: the closure is freed when the timer
    /// is cleared, when a one-shot timer has fired, or when its world goes away.
    pub fn k2_set_timer_delegate(
        delegate: impl FnMut() + Send + 'static,
        time: f32,
        b_looping: bool,
        b_max_once_per_frame: Option<bool>,
        initial_start_delay: Option<f32>,
        initial_start_delay_variance: Option<f32>,
    ) -> OwnedStruct<FTimerHandle> {
        let variance = f64::from(initial_start_delay_variance.unwrap_or(0.0));
        let start = f64::from(initial_start_delay.unwrap_or(0.0))
            + if variance > 0.0 { random_in(variance) } else { 0.0 };

        engine_handle(set_timer_raw(
            rusteal_core::UObjectHandle::null(),
            time,
            b_looping,
            b_max_once_per_frame.unwrap_or(false),
            time + start as f32,
            delegate,
        ))
    }

    /// Blueprint's **Set Timer for Next Tick by Event**, calling `delegate` once.
    pub fn k2_set_timer_for_next_tick_delegate(
        mut delegate: impl FnMut() + Send + 'static,
    ) -> OwnedStruct<FTimerHandle> {
        engine_handle(set_timer_for_next_tick_raw(
            rusteal_core::UObjectHandle::null(),
            move || delegate(),
        ))
    }
}
