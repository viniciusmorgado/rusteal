pub(crate) use std::sync::{Mutex, MutexGuard, Once, RwLock, RwLockReadGuard, RwLockWriteGuard};
// Public so the exported profiling macros can name it from the crates that expand them.
pub use std::sync::OnceLock;
pub(crate) use std::thread_local;

pub(crate) fn lock_or_recover<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

pub(crate) fn read_or_recover<T>(m: &RwLock<T>) -> RwLockReadGuard<'_, T> {
    m.read().unwrap_or_else(|e| e.into_inner())
}

pub(crate) fn write_or_recover<T>(m: &RwLock<T>) -> RwLockWriteGuard<'_, T> {
    m.write().unwrap_or_else(|e| e.into_inner())
}

thread_local! {
    /// Whether this thread is the engine's game thread: 0 not asked yet, 1 yes, 2 no. Only the
    /// engine's answer is kept, which never changes for a thread.
    static GAME_THREAD: std::cell::Cell<u8> = const { std::cell::Cell::new(0) };
}

/// Whether the caller is on the game thread, asked of the engine once per thread. Without an
/// engine (tests, tools) there is no game thread and nothing in the engine to race with, so every
/// thread counts.
pub(crate) fn on_game_thread() -> bool {
    if !crate::task::engine() {
        return true;
    }

    GAME_THREAD.with(|known| match known.get() {
        1 => true,
        2 => false,
        _ => {
            let answer = crate::task::is_game_thread();
            known.set(if answer { 1 } else { 2 });
            answer
        }
    })
}

/// A value only the game thread may use or drop, such as a callback that captures object
/// references: it can sit in shared structures (the callback registry, the task queue), which
/// is what its `Send` and `Sync` allow, while everything it holds stays on the game thread.
///
/// It is made on the game thread, and only the game thread reaches what it holds. When another
/// thread ends up dropping it, the value is handed back to the game thread and dropped there.
pub struct GameThreadOnly<T: 'static>(std::mem::ManuallyDrop<T>);

unsafe impl<T: 'static> Send for GameThreadOnly<T> {}
unsafe impl<T: 'static> Sync for GameThreadOnly<T> {}

impl<T: 'static> GameThreadOnly<T> {
    /// Keeps `value`; panics off the game thread, where `value` could already hold what belongs
    /// to another thread.
    #[track_caller]
    pub fn new(value: T) -> Self {
        assert!(
            on_game_thread(),
            "Rusteal: this is kept for the game thread and must be made there"
        );

        GameThreadOnly(std::mem::ManuallyDrop::new(value))
    }

    /// What it holds; panics off the game thread.
    #[track_caller]
    pub fn get_mut(&mut self) -> &mut T {
        assert!(
            on_game_thread(),
            "Rusteal: this is kept for the game thread and used only there"
        );

        &mut self.0
    }

    /// What it holds, taken out; panics off the game thread.
    #[track_caller]
    pub fn into_inner(mut self) -> T {
        assert!(
            on_game_thread(),
            "Rusteal: this is kept for the game thread and used only there"
        );

        let value = unsafe { std::mem::ManuallyDrop::take(&mut self.0) };
        std::mem::forget(self);
        value
    }
}

impl<T: 'static> Drop for GameThreadOnly<T> {
    fn drop(&mut self) {
        if on_game_thread() {
            unsafe { std::mem::ManuallyDrop::drop(&mut self.0) };
            return;
        }

        let value = GameThreadOnly(std::mem::ManuallyDrop::new(unsafe {
            std::mem::ManuallyDrop::take(&mut self.0)
        }));
        crate::task::run_on_game_thread(move || drop(value));
    }
}

thread_local! {
    /// How many `#[ufunction(ThreadSafe)]` calls this thread is inside: there, reading and writing
    /// properties off the game thread is what the engine asked for.
    static THREAD_SAFE_CALLS: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

/// Marks the thread as inside a `#[ufunction(ThreadSafe)]` call until it is dropped; the
/// trampoline the macros generate makes one.
#[doc(hidden)]
#[must_use]
pub struct ThreadSafeScope(std::marker::PhantomData<*const ()>);

#[doc(hidden)]
pub fn thread_safe_scope() -> ThreadSafeScope {
    THREAD_SAFE_CALLS.with(|calls| calls.set(calls.get() + 1));
    ThreadSafeScope(std::marker::PhantomData)
}

impl Drop for ThreadSafeScope {
    fn drop(&mut self) {
        THREAD_SAFE_CALLS.with(|calls| calls.set(calls.get().saturating_sub(1)));
    }
}

/// The debug builds' check before an engine call that needs the game thread (`api` names it).
#[doc(hidden)]
#[track_caller]
pub fn assert_game_thread(api: &'static str) {
    assert!(
        on_game_thread(),
        "Rusteal: {api} called off the game thread. Engine objects belong to the game thread: \
         keep a TWeakObjectPtr and resolve it in task::run_on_game_thread, or do the work in \
         task::spawn's `done`"
    );
}

/// The same check for the calls a `#[ufunction(ThreadSafe)]` may make on a worker thread:
/// properties, containers, text and errors.
#[doc(hidden)]
#[track_caller]
pub fn assert_game_thread_or_thread_safe(api: &'static str) {
    assert!(
        on_game_thread() || THREAD_SAFE_CALLS.with(|calls| calls.get() > 0),
        "Rusteal: {api} called off the game thread, outside a #[ufunction(ThreadSafe)]. Engine \
         objects belong to the game thread: keep a TWeakObjectPtr and resolve it in \
         task::run_on_game_thread, or do the work in task::spawn's `done`"
    );
}
