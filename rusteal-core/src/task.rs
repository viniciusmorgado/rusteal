use std::panic::AssertUnwindSafe;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};
use std::thread::ThreadId;
use std::time::{Duration, Instant};

use crate::lock_or_recover;

type Job = Box<dyn FnOnce() + Send>;
type TickFn = Box<dyn FnMut(f32) + Send>;

static QUEUE: Mutex<Vec<Job>> = Mutex::new(Vec::new());
static TICKERS: Mutex<Vec<(u64, TickFn)>> = Mutex::new(Vec::new());
static REMOVED: Mutex<Vec<u64>> = Mutex::new(Vec::new());
static NEXT_TICKER: AtomicU64 = AtomicU64::new(1);
static RUNNING: AtomicUsize = AtomicUsize::new(0);
static GAME_THREAD: OnceLock<ThreadId> = OnceLock::new();

fn log(level: u8, message: &str) {
    if crate::api::is_api_initialized() {
        unsafe {
            crate::ffi_dispatch::logging_log(level, message.as_ptr(), message.len() as u32);
        }
    }
}

pub fn is_game_thread() -> bool {
    GAME_THREAD
        .get()
        .is_some_and(|id| *id == std::thread::current().id())
}

pub fn run_on_game_thread(f: impl FnOnce() + Send + 'static) {
    lock_or_recover(&QUEUE).push(Box::new(f));
}

pub fn spawn<T: Send + 'static>(
    work: impl FnOnce() -> T + Send + 'static,
    done: impl FnOnce(T) + Send + 'static,
) {
    RUNNING.fetch_add(1, Ordering::SeqCst);

    let started = std::thread::Builder::new()
        .name("rusteal-task".into())
        .spawn(move || {
            let result = std::panic::catch_unwind(AssertUnwindSafe(work));

            match result {
                Ok(value) => run_on_game_thread(move || done(value)),
                Err(_) => run_on_game_thread(|| {
                    log(crate::LOG_ERROR, "[Rusteal] task::spawn: the work panicked")
                }),
            }

            RUNNING.fetch_sub(1, Ordering::SeqCst);
        });

    if started.is_err() {
        RUNNING.fetch_sub(1, Ordering::SeqCst);

        log(
            crate::LOG_ERROR,
            "[Rusteal] task::spawn: could not start a thread",
        );
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use = "keep the handle to remove the hook, or drop it to keep the hook forever"]
pub struct TickHandle(u64);

impl TickHandle {
    pub fn remove(self) {
        let mut tickers = lock_or_recover(&TICKERS);
        let before = tickers.len();
        tickers.retain(|(id, _)| *id != self.0);

        if tickers.len() == before {
            lock_or_recover(&REMOVED).push(self.0);
        }
    }
}

pub fn on_tick(f: impl FnMut(f32) + Send + 'static) -> TickHandle {
    let id = NEXT_TICKER.fetch_add(1, Ordering::Relaxed);
    lock_or_recover(&TICKERS).push((id, Box::new(f)));
    TickHandle(id)
}

#[doc(hidden)]
pub fn mark_game_thread() {
    let _ = GAME_THREAD.set(std::thread::current().id());
}

#[doc(hidden)]
pub fn tick(delta_seconds: f32) {
    mark_game_thread();

    let jobs = std::mem::take(&mut *lock_or_recover(&QUEUE));

    for job in jobs {
        if std::panic::catch_unwind(AssertUnwindSafe(job)).is_err() {
            log(
                crate::LOG_ERROR,
                "[Rusteal] a closure run on the game thread panicked",
            );
        }
    }

    let mut running = std::mem::take(&mut *lock_or_recover(&TICKERS));

    for (_, hook) in running.iter_mut() {
        if std::panic::catch_unwind(AssertUnwindSafe(|| hook(delta_seconds))).is_err() {
            log(crate::LOG_ERROR, "[Rusteal] a tick hook panicked");
        }
    }

    let removed = std::mem::take(&mut *lock_or_recover(&REMOVED));
    running.retain(|(id, _)| !removed.contains(id));
    let mut tickers = lock_or_recover(&TICKERS);
    running.append(&mut tickers);
    *tickers = running;
}

#[doc(hidden)]
pub fn shutdown() {
    let started = Instant::now();
    let mut warned = false;

    while RUNNING.load(Ordering::SeqCst) > 0 {
        if !warned && started.elapsed() > Duration::from_secs(1) {
            warned = true;

            log(
                crate::LOG_WARNING,
                &format!(
                    "[Rusteal] waiting for {} background task(s) before unloading",
                    RUNNING.load(Ordering::SeqCst)
                ),
            );
        }

        std::thread::sleep(Duration::from_millis(1));
    }

    lock_or_recover(&QUEUE).clear();
    lock_or_recover(&TICKERS).clear();
    lock_or_recover(&REMOVED).clear();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn work_comes_back_on_the_ticking_thread() {
        let seen = Arc::new(Mutex::new(Vec::<String>::new()));

        let log = seen.clone();

        spawn(
            || 6 * 7,
            move |answer| log.lock().unwrap().push(format!("done {answer}")),
        );

        let log = seen.clone();
        let hook = on_tick(move |dt| log.lock().unwrap().push(format!("tick {dt}")));

        let deadline = Instant::now() + Duration::from_secs(5);

        while !seen.lock().unwrap().iter().any(|s| s == "done 42") {
            assert!(Instant::now() < deadline, "the work never came back");
            tick(0.5);
            std::thread::sleep(Duration::from_millis(1));
        }

        assert!(is_game_thread());
        assert!(seen.lock().unwrap().iter().any(|s| s == "tick 0.5"));

        hook.remove();
        seen.lock().unwrap().clear();
        tick(0.25);
        assert!(seen.lock().unwrap().is_empty(), "a removed hook ran");

        let log = seen.clone();
        run_on_game_thread(move || log.lock().unwrap().push("queued".into()));
        shutdown();
        tick(0.1);
        assert!(seen.lock().unwrap().is_empty(), "shutdown kept the queue");
    }
}
