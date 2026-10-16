use std::sync::OnceLock;
use std::thread::ThreadId;
use std::time::{Duration, Instant};

static START: OnceLock<Instant> = OnceLock::new();
static GAME_THREAD: OnceLock<ThreadId> = OnceLock::new();

pub(crate) fn seconds() -> f64 {
    START.get_or_init(Instant::now).elapsed().as_secs_f64()
}

pub(crate) fn sleep(duration: Duration) {
    std::thread::sleep(duration);
}

pub(crate) fn spawn(job: Box<dyn FnOnce() + Send>) -> bool {
    std::thread::Builder::new()
        .name("rusteal-task".into())
        .spawn(job)
        .is_ok()
}

pub(crate) fn mark_game_thread() {
    let _ = GAME_THREAD.set(std::thread::current().id());
}

pub(crate) fn is_game_thread() -> bool {
    GAME_THREAD
        .get()
        .is_some_and(|id| *id == std::thread::current().id())
}

pub(crate) struct FallbackThread(std::thread::JoinHandle<()>);

impl FallbackThread {
    pub(crate) fn join(self) {
        let _ = self.0.join();
    }
}

pub(crate) fn spawn_thread(name: &str, job: Box<dyn FnOnce() + Send>) -> Option<FallbackThread> {
    std::thread::Builder::new()
        .name(name.to_string())
        .spawn(job)
        .ok()
        .map(FallbackThread)
}

pub(crate) fn worker_count() -> usize {
    std::thread::available_parallelism().map_or(1, |n| n.get().saturating_sub(1).max(1))
}
