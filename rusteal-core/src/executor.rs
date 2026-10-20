use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::future::Future;
use std::panic::AssertUnwindSafe;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::task::{Context, Poll, Wake, Waker};

use rusteal_ffi::FWeakObjectHandle;

use crate::lock_or_recover;
use crate::module::{self, ModuleId};
use crate::object_ref::UObjectRef;
use crate::sync::{Mutex, thread_local};
use crate::traits::UeClass;

type LocalFuture = Pin<Box<dyn Future<Output = ()>>>;

struct Task {
    future: LocalFuture,
    owner: Option<FWeakObjectHandle>,
    abort: Rc<RefCell<bool>>,
}

#[derive(Default)]
struct Executor {
    tasks: HashMap<u64, Task>,
}

thread_local! {
    static EXECUTORS: RefCell<BTreeMap<ModuleId, Executor>> = const { RefCell::new(BTreeMap::new()) };
}

static READY: Mutex<BTreeMap<ModuleId, Vec<u64>>> = Mutex::new(BTreeMap::new());
static NEXT_TASK: AtomicU64 = AtomicU64::new(1);

struct TaskWaker {
    module: ModuleId,
    id: u64,
}

impl Wake for TaskWaker {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        lock_or_recover(&READY)
            .entry(self.module)
            .or_default()
            .push(self.id);
    }
}

struct JoinState<T> {
    result: Option<T>,
    done: bool,
    waiter: Option<Waker>,
}

/// A task started with [`spawn_local`]: await it for its result, `None` when it was cancelled.
///
/// Dropping the handle does not cancel the task; [`JoinHandle::abort`] does.
#[must_use = "await the handle for the result, or drop it to let the task run on its own"]
pub struct JoinHandle<T> {
    state: Rc<RefCell<JoinState<T>>>,
    abort: Rc<RefCell<bool>>,
    module: ModuleId,
    id: u64,
}

impl<T> JoinHandle<T> {
    /// Cancels the task: its future is dropped before it is polled again.
    pub fn abort(&self) {
        *self.abort.borrow_mut() = true;
        let removed = EXECUTORS.with(|executors| {
            executors
                .borrow_mut()
                .get_mut(&self.module)
                .and_then(|executor| executor.tasks.remove(&self.id))
        });
        drop(removed);
    }

    pub fn is_finished(&self) -> bool {
        self.state.borrow().done
    }
}

impl<T> Future for JoinHandle<T> {
    type Output = Option<T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<T>> {
        let mut state = self.state.borrow_mut();

        if state.done {
            return Poll::Ready(state.result.take());
        }

        state.waiter = Some(cx.waker().clone());
        Poll::Pending
    }
}

struct Finish<T> {
    state: Rc<RefCell<JoinState<T>>>,
}

impl<T> Drop for Finish<T> {
    fn drop(&mut self) {
        let waiter = {
            let mut state = self.state.borrow_mut();
            state.done = true;
            state.waiter.take()
        };

        if let Some(waiter) = waiter {
            waiter.wake();
        }
    }
}

/// Runs `future` on the game thread, polled from the engine's tick, until it finishes or `owner`
/// is destroyed, whichever comes first.
pub fn spawn_local<T: 'static>(
    owner: &UObjectRef<impl UeClass>,
    future: impl Future<Output = T> + 'static,
) -> JoinHandle<T> {
    let weak = unsafe { crate::ffi_dispatch::core_make_weak(owner.raw()) };
    spawn_with(Some(weak), future)
}

/// Runs `future` on the game thread until it finishes or the library unloads.
pub fn spawn_local_unowned<T: 'static>(future: impl Future<Output = T> + 'static) -> JoinHandle<T> {
    spawn_with(None, future)
}

fn spawn_with<T: 'static>(
    owner: Option<FWeakObjectHandle>,
    future: impl Future<Output = T> + 'static,
) -> JoinHandle<T> {
    let module = module::current();
    let id = NEXT_TASK.fetch_add(1, Ordering::Relaxed);

    let state = Rc::new(RefCell::new(JoinState {
        result: None,
        done: false,
        waiter: None,
    }));

    let abort = Rc::new(RefCell::new(false));

    if crate::task::engine() && !crate::task::is_game_thread() {
        crate::task::log(
            crate::LOG_ERROR,
            "[Rusteal] task::spawn_local runs on the game thread: the task was dropped",
        );

        state.borrow_mut().done = true;

        return JoinHandle {
            state,
            abort,
            module,
            id,
        };
    }

    let finish = Finish {
        state: state.clone(),
    };

    let wrapped = async move {
        let finish = finish;
        let value = future.await;
        finish.state.borrow_mut().result = Some(value);
    };

    EXECUTORS.with(|executors| {
        executors
            .borrow_mut()
            .entry(module)
            .or_default()
            .tasks
            .insert(
                id,
                Task {
                    future: Box::pin(wrapped),
                    owner,
                    abort: abort.clone(),
                },
            );
    });

    lock_or_recover(&READY).entry(module).or_default().push(id);

    JoinHandle {
        state,
        abort,
        module,
        id,
    }
}

fn owner_alive(owner: Option<FWeakObjectHandle>) -> bool {
    match owner {
        None => true,
        Some(weak) => {
            !crate::api::is_api_initialized()
                || unsafe { crate::ffi_dispatch::core_is_weak_valid(weak) }
        }
    }
}

pub(crate) fn run(module: ModuleId) {
    let ready = lock_or_recover(&READY)
        .get_mut(&module)
        .map(std::mem::take)
        .unwrap_or_default();

    for id in ready {
        let task = EXECUTORS.with(|executors| {
            executors
                .borrow_mut()
                .get_mut(&module)
                .and_then(|executor| executor.tasks.remove(&id))
        });

        let Some(mut task) = task else {
            continue;
        };

        if *task.abort.borrow() || !owner_alive(task.owner) {
            drop(task);
            continue;
        }

        let waker = Waker::from(Arc::new(TaskWaker { module, id }));
        let mut cx = Context::from_waker(&waker);
        let polled =
            std::panic::catch_unwind(AssertUnwindSafe(|| task.future.as_mut().poll(&mut cx)));

        match polled {
            Ok(Poll::Ready(())) => {}
            Ok(Poll::Pending) => {
                if !*task.abort.borrow() {
                    EXECUTORS.with(|executors| {
                        executors
                            .borrow_mut()
                            .entry(module)
                            .or_default()
                            .tasks
                            .insert(id, task);
                    });
                }
            }
            Err(_) => crate::task::log(crate::LOG_ERROR, "[Rusteal] an async task panicked"),
        }
    }
}

pub(crate) fn sweep_dead_owners(module: ModuleId) {
    let dead: Vec<Task> = EXECUTORS.with(|executors| {
        let mut executors = executors.borrow_mut();

        let Some(executor) = executors.get_mut(&module) else {
            return Vec::new();
        };

        let ids: Vec<u64> = executor
            .tasks
            .iter()
            .filter(|(_, task)| !owner_alive(task.owner))
            .map(|(id, _)| *id)
            .collect();

        ids.iter()
            .filter_map(|id| executor.tasks.remove(id))
            .collect()
    });

    drop(dead);
}

pub(crate) fn shutdown(module: ModuleId) {
    let executor = EXECUTORS.with(|executors| executors.borrow_mut().remove(&module));
    drop(executor);
    lock_or_recover(&READY).remove(&module);
}

#[cfg(test)]
pub(crate) fn task_count(module: ModuleId) -> usize {
    EXECUTORS.with(|executors| {
        executors
            .borrow()
            .get(&module)
            .map_or(0, |executor| executor.tasks.len())
    })
}

/// Gives the other tasks a turn: resumes on the next frame.
pub fn yield_now() -> impl Future<Output = ()> {
    NextFrame { polled: false }
}

/// Resumes on the next frame.
pub fn next_frame() -> impl Future<Output = ()> {
    NextFrame { polled: false }
}

struct NextFrame {
    polled: bool,
}

impl Future for NextFrame {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.polled {
            return Poll::Ready(());
        }

        self.polled = true;
        cx.waker().wake_by_ref();
        Poll::Pending
    }
}

struct BlockingState<T> {
    result: Option<T>,
    failed: bool,
    waker: Option<Waker>,
}

/// Runs `work` on one of the engine's worker threads and resumes with its result on the game
/// thread; `None` when the work panicked.
pub fn run_blocking<T: Send + 'static>(
    work: impl FnOnce() -> T + Send + 'static,
) -> impl Future<Output = Option<T>> {
    let state = Arc::new(Mutex::new(BlockingState {
        result: None,
        failed: false,
        waker: None,
    }));

    let filled = state.clone();
    let worker = move || std::panic::catch_unwind(AssertUnwindSafe(work)).ok();

    crate::task::spawn(worker, move |result| {
        let waker = {
            let mut state = lock_or_recover(&filled);

            match result {
                Some(value) => state.result = Some(value),
                None => state.failed = true,
            }

            state.waker.take()
        };

        if let Some(waker) = waker {
            waker.wake();
        }
    });

    RunBlocking { state }
}

struct RunBlocking<T> {
    state: Arc<Mutex<BlockingState<T>>>,
}

impl<T> Future for RunBlocking<T> {
    type Output = Option<T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<T>> {
        let mut state = lock_or_recover(&self.state);

        if let Some(value) = state.result.take() {
            return Poll::Ready(Some(value));
        }

        if state.failed {
            return Poll::Ready(None);
        }

        state.waker = Some(cx.waker().clone());
        Poll::Pending
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::module::{FuncTable, Module};
    use crate::task::tick;
    use std::cell::Cell;
    use std::time::{Duration, Instant};

    static FUNCS: FuncTable = FuncTable::new(0, 0);
    static FRAMES: Module = Module::new("executor_frames", "", &FUNCS);
    static ABORT: Module = Module::new("executor_abort", "", &FUNCS);
    static BLOCKING: Module = Module::new("executor_blocking", "", &FUNCS);
    static UNLOAD: Module = Module::new("executor_unload", "", &FUNCS);
    static PANIC: Module = Module::new("executor_panic", "", &FUNCS);

    struct Dropped(Rc<Cell<bool>>);

    impl Drop for Dropped {
        fn drop(&mut self) {
            self.0.set(true);
        }
    }

    #[test]
    fn a_task_resumes_one_frame_per_await_and_gives_its_result() {
        let _scope = FRAMES.enter();
        let steps = Rc::new(Cell::new(0));
        let seen = steps.clone();

        let task = spawn_local_unowned(async move {
            for _ in 0..3 {
                seen.set(seen.get() + 1);
                next_frame().await;
            }

            "done"
        });

        let joined = Rc::new(RefCell::new(None));
        let sink = joined.clone();
        let _join = spawn_local_unowned(async move {
            *sink.borrow_mut() = task.await;
        });

        tick(0.0);
        assert_eq!(steps.get(), 1);
        tick(0.0);
        assert_eq!(steps.get(), 2);
        tick(0.0);
        assert_eq!(steps.get(), 3);
        assert!(joined.borrow().is_none());

        tick(0.0);
        tick(0.0);
        assert_eq!(*joined.borrow(), Some("done"));
        assert_eq!(task_count(FRAMES.id()), 0);
    }

    #[test]
    fn an_aborted_task_is_dropped_and_its_handle_gives_none() {
        let _scope = ABORT.enter();
        let dropped = Rc::new(Cell::new(false));
        let guard = Dropped(dropped.clone());

        let task = spawn_local_unowned(async move {
            let _guard = guard;

            loop {
                yield_now().await;
            }
        });

        tick(0.0);
        assert!(!dropped.get());

        task.abort();
        assert!(dropped.get(), "abort did not drop the future");
        assert!(task.is_finished());

        let joined = Rc::new(RefCell::new(Some(Some(()))));
        let sink = joined.clone();
        let _join = spawn_local_unowned(async move {
            *sink.borrow_mut() = Some(task.await);
        });

        tick(0.0);
        assert_eq!(*joined.borrow(), Some(None));
    }

    #[test]
    fn blocking_work_comes_back_to_the_task() {
        let _scope = BLOCKING.enter();
        let answer = Rc::new(Cell::new(0));
        let sink = answer.clone();

        let _task = spawn_local_unowned(async move {
            let value = run_blocking(|| 6 * 7).await.unwrap();
            let failed = run_blocking(|| -> i32 { panic!("on purpose") }).await;
            sink.set(if failed.is_none() { value } else { -1 });
        });

        let deadline = Instant::now() + Duration::from_secs(5);

        while answer.get() == 0 {
            assert!(
                Instant::now() < deadline,
                "the blocking work never came back"
            );
            tick(0.0);
            std::thread::sleep(Duration::from_millis(1));
        }

        assert_eq!(answer.get(), 42);
    }

    #[test]
    fn unloading_drops_every_task() {
        let _scope = UNLOAD.enter();
        let dropped = Rc::new(Cell::new(false));
        let guard = Dropped(dropped.clone());

        let _task = spawn_local_unowned(async move {
            let _guard = guard;
            std::future::pending::<()>().await;
        });

        tick(0.0);
        assert!(!dropped.get());

        crate::task::shutdown();
        assert!(dropped.get(), "shutdown kept the task");
        assert_eq!(task_count(UNLOAD.id()), 0);
    }

    #[test]
    fn a_panicking_task_does_not_stop_the_others() {
        let _scope = PANIC.enter();
        let ran = Rc::new(Cell::new(false));
        let sink = ran.clone();

        let _bad = spawn_local_unowned(async { panic!("on purpose") });
        let _good = spawn_local_unowned(async move {
            next_frame().await;
            sink.set(true);
        });

        tick(0.0);
        tick(0.0);
        assert!(ran.get());
        assert_eq!(task_count(PANIC.id()), 0);
    }
}
