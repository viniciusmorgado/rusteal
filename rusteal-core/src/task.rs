use std::collections::BTreeMap;
use std::ffi::c_void;
use std::panic::AssertUnwindSafe;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use crate::lock_or_recover;
use crate::module::{self, ModuleId};
use crate::sync::Mutex;

pub use crate::executor::{
    JoinHandle, next_frame, run_blocking, spawn_local, spawn_local_unowned, yield_now,
};

type Job = Box<dyn FnOnce() + Send>;
type TickFn = Box<dyn FnMut(f32) + Send>;

#[derive(Default)]
struct ModuleTasks {
    queue: Vec<Job>,
    tickers: Vec<(u64, TickFn)>,
    removed: Vec<u64>,
    running: usize,
    detached: Vec<ThreadInner>,
}

enum ThreadInner {
    Engine(usize),
    Fallback(crate::std_fallback::FallbackThread),
}

impl ThreadInner {
    fn join(self) {
        match self {
            ThreadInner::Engine(handle) => unsafe {
                crate::ffi_dispatch::platform_join_thread(handle as *mut u8)
            },
            ThreadInner::Fallback(thread) => thread.join(),
        }
    }
}

pub struct EngineThread {
    inner: Option<ThreadInner>,
    module: ModuleId,
}

impl EngineThread {
    pub fn join(mut self) {
        if let Some(inner) = self.inner.take() {
            inner.join();
        }
    }
}

impl Drop for EngineThread {
    fn drop(&mut self) {
        if let Some(inner) = self.inner.take() {
            with_tasks(self.module, |tasks| tasks.detached.push(inner));
        }
    }
}

impl std::fmt::Debug for EngineThread {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EngineThread")
            .field("module", &self.module)
            .finish()
    }
}

#[track_caller]
pub fn spawn_thread(
    name: &str,
    f: impl FnOnce() + Send + 'static,
) -> crate::RustealResult<EngineThread> {
    let owner = module::current();

    let job: Job = Box::new(move || {
        let _scope = module::scope(owner);

        if std::panic::catch_unwind(AssertUnwindSafe(f)).is_err() {
            queue_for(
                owner,
                Box::new(|| {
                    log(
                        crate::LOG_ERROR,
                        "[Rusteal] task::spawn_thread: the thread panicked",
                    )
                }),
            );
        }
    });

    let inner = if engine() {
        let data = Box::into_raw(Box::new(job));
        let run = run_job as extern "C" fn(*mut u8) as *const c_void;

        let handle = unsafe {
            crate::ffi_dispatch::platform_create_thread(
                run,
                data as *mut u8,
                name.as_ptr(),
                name.len() as u32,
            )
        };

        if handle.is_null() {
            drop(unsafe { Box::from_raw(data) });
            None
        } else {
            Some(ThreadInner::Engine(handle as usize))
        }
    } else {
        crate::std_fallback::spawn_thread(name, job).map(ThreadInner::Fallback)
    };

    match inner {
        Some(inner) => Ok(EngineThread {
            inner: Some(inner),
            module: owner,
        }),
        None => Err(crate::RustealError::invalid_operation(
            "the engine could not start the thread",
        )),
    }
}

pub fn worker_count() -> usize {
    if engine() {
        unsafe { crate::ffi_dispatch::platform_worker_count() as usize }
    } else {
        crate::std_fallback::worker_count()
    }
}

static TASKS: Mutex<BTreeMap<ModuleId, ModuleTasks>> = Mutex::new(BTreeMap::new());
static NEXT_TICKER: AtomicU64 = AtomicU64::new(1);

fn with_tasks<R>(module: ModuleId, f: impl FnOnce(&mut ModuleTasks) -> R) -> R {
    f(lock_or_recover(&TASKS).entry(module).or_default())
}

pub(crate) fn log(level: u8, message: &str) {
    if crate::api::is_api_initialized() {
        unsafe {
            crate::ffi_dispatch::logging_log(level, message.as_ptr(), message.len() as u32);
        }
    }
}

pub(crate) fn engine() -> bool {
    crate::api::is_api_initialized() && !crate::api::api().platform.is_null()
}

pub fn is_game_thread() -> bool {
    if engine() {
        unsafe { crate::ffi_dispatch::platform_is_game_thread() }
    } else {
        crate::std_fallback::is_game_thread()
    }
}

extern "C" fn run_job(data: *mut u8) {
    let job = unsafe { Box::from_raw(data as *mut Job) };
    let _ = std::panic::catch_unwind(AssertUnwindSafe(job));
}

fn launch(job: Job) -> bool {
    if !engine() {
        return crate::std_fallback::spawn(job);
    }

    let data = Box::into_raw(Box::new(job));
    let run = run_job as extern "C" fn(*mut u8) as *const c_void;
    let launched = unsafe { crate::ffi_dispatch::platform_launch_task(run, data as *mut u8) };

    if !launched {
        drop(unsafe { Box::from_raw(data) });
    }

    launched
}

pub fn run_on_game_thread(f: impl FnOnce() + Send + 'static) {
    queue_for(module::current(), Box::new(f));
}

fn queue_for(module: ModuleId, job: Job) {
    with_tasks(module, |tasks| tasks.queue.push(job));
}

pub fn spawn<T: Send + 'static>(
    work: impl FnOnce() -> T + Send + 'static,
    done: impl FnOnce(T) + Send + 'static,
) {
    let owner = module::current();
    with_tasks(owner, |tasks| tasks.running += 1);

    let started = launch(Box::new(move || {
        let _scope = module::scope(owner);
        let result = std::panic::catch_unwind(AssertUnwindSafe(work));

        match result {
            Ok(value) => queue_for(owner, Box::new(move || done(value))),
            Err(_) => queue_for(
                owner,
                Box::new(|| log(crate::LOG_ERROR, "[Rusteal] task::spawn: the work panicked")),
            ),
        }

        with_tasks(owner, |tasks| tasks.running -= 1);
    }));

    if !started {
        with_tasks(owner, |tasks| tasks.running -= 1);

        log(
            crate::LOG_ERROR,
            "[Rusteal] task::spawn: the engine could not start the task",
        );
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use = "keep the handle to remove the hook, or drop it to keep the hook forever"]
pub struct TickHandle {
    module: ModuleId,
    id: u64,
}

impl TickHandle {
    pub fn remove(self) {
        with_tasks(self.module, |tasks| {
            let before = tasks.tickers.len();
            tasks.tickers.retain(|(id, _)| *id != self.id);

            if tasks.tickers.len() == before {
                tasks.removed.push(self.id);
            }
        });
    }
}

pub fn on_tick(f: impl FnMut(f32) + Send + 'static) -> TickHandle {
    let module = module::current();
    let id = NEXT_TICKER.fetch_add(1, Ordering::Relaxed);
    with_tasks(module, |tasks| tasks.tickers.push((id, Box::new(f))));
    TickHandle { module, id }
}

#[doc(hidden)]
pub fn mark_game_thread() {
    crate::std_fallback::mark_game_thread();
}

#[doc(hidden)]
pub fn tick(delta_seconds: f32) {
    mark_game_thread();

    let module = module::current();
    let jobs = with_tasks(module, |tasks| std::mem::take(&mut tasks.queue));

    for job in jobs {
        if std::panic::catch_unwind(AssertUnwindSafe(job)).is_err() {
            log(
                crate::LOG_ERROR,
                "[Rusteal] a closure run on the game thread panicked",
            );
        }
    }

    crate::executor::sweep_dead_owners(module);
    crate::executor::run(module);

    let mut running = with_tasks(module, |tasks| std::mem::take(&mut tasks.tickers));

    for (_, hook) in running.iter_mut() {
        if std::panic::catch_unwind(AssertUnwindSafe(|| hook(delta_seconds))).is_err() {
            log(crate::LOG_ERROR, "[Rusteal] a tick hook panicked");
        }
    }

    with_tasks(module, |tasks| {
        let removed = std::mem::take(&mut tasks.removed);
        running.retain(|(id, _)| !removed.contains(id));
        running.append(&mut tasks.tickers);
        tasks.tickers = running;
    });
}

#[doc(hidden)]
pub fn shutdown() {
    let module = module::current();
    crate::executor::shutdown(module);
    let started = crate::time::Instant::now();
    let mut warned = false;

    loop {
        let running = with_tasks(module, |tasks| tasks.running);

        if running == 0 {
            break;
        }

        if !warned && started.elapsed() > Duration::from_secs(1) {
            warned = true;

            log(
                crate::LOG_WARNING,
                &format!("[Rusteal] waiting for {running} background task(s) before unloading"),
            );
        }

        crate::time::sleep(Duration::from_millis(1));
    }

    let finished = lock_or_recover(&TASKS).remove(&module);

    if let Some(mut finished) = finished {
        for thread in finished.detached.drain(..) {
            thread.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::module::{FuncTable, Module};
    use std::sync::Arc;
    use std::time::Instant;

    static FUNCS: FuncTable = FuncTable::new(0, 0);
    static SOLO: Module = Module::new("task_solo", "", &FUNCS);
    static GAME: Module = Module::new("task_game", "", &FUNCS);
    static PLUGIN: Module = Module::new("task_plugin", "", &FUNCS);

    #[test]
    fn work_comes_back_on_the_ticking_thread() {
        let _scope = SOLO.enter();
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

    #[test]
    fn each_module_ticks_and_finishes_its_own_work() {
        let seen = Arc::new(Mutex::new(Vec::<String>::new()));

        let game_hook = {
            let _scope = GAME.enter();
            let log = seen.clone();
            on_tick(move |_| log.lock().unwrap().push("game hook".into()))
        };

        {
            let _scope = PLUGIN.enter();
            let log = seen.clone();
            let _ = on_tick(move |_| log.lock().unwrap().push("plugin hook".into()));
            let log = seen.clone();

            spawn(
                || "plugin work",
                move |what| log.lock().unwrap().push(format!("{what} done")),
            );
        }

        let deadline = Instant::now() + Duration::from_secs(5);

        loop {
            {
                let _scope = GAME.enter();
                tick(0.0);
            }

            assert!(
                !seen.lock().unwrap().iter().any(|s| s.starts_with("plugin")),
                "the game's tick ran the plugin's hooks or work"
            );

            let pending = with_tasks(PLUGIN.id(), |tasks| tasks.running + tasks.queue.len());

            if pending > 0 && with_tasks(PLUGIN.id(), |tasks| tasks.running) == 0 {
                break;
            }

            assert!(
                Instant::now() < deadline,
                "the plugin's work never finished"
            );
            std::thread::sleep(Duration::from_millis(1));
        }

        seen.lock().unwrap().clear();

        {
            let _scope = PLUGIN.enter();
            tick(0.0);
        }

        let plugin_ticked = seen.lock().unwrap().clone();
        assert_eq!(plugin_ticked, vec!["plugin work done", "plugin hook"]);

        {
            let _scope = PLUGIN.enter();
            shutdown();
        }

        seen.lock().unwrap().clear();

        {
            let _scope = GAME.enter();
            tick(0.0);
        }

        assert_eq!(
            *seen.lock().unwrap(),
            vec!["game hook"],
            "a plugin's shutdown stopped the game"
        );
        game_hook.remove();
    }

    static THREADS: Module = Module::new("task_threads", "", &FUNCS);

    #[test]
    fn threads_run_in_their_module_and_are_joined() {
        let _scope = THREADS.enter();
        let seen = Arc::new(Mutex::new(Vec::<String>::new()));

        let log = seen.clone();

        let joined = spawn_thread("rusteal-test", move || {
            log.lock()
                .unwrap()
                .push(format!("joined in {:?}", module::current()));
        })
        .unwrap();

        joined.join();

        assert_eq!(
            *seen.lock().unwrap(),
            vec![format!("joined in {:?}", THREADS.id())]
        );

        let log = seen.clone();

        drop(
            spawn_thread("rusteal-detached", move || {
                std::thread::sleep(Duration::from_millis(20));
                log.lock().unwrap().push("detached".into());
            })
            .unwrap(),
        );

        shutdown();
        assert!(
            seen.lock().unwrap().iter().any(|s| s == "detached"),
            "shutdown did not join the thread"
        );
        assert!(worker_count() >= 1);
    }
}
