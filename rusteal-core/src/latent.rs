use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};

use crate::delegate_registry::{register_callback, unregister_callback};
use crate::lock_or_recover;
use crate::sync::Mutex;

#[derive(Default)]
struct State {
    done: bool,
    waker: Option<Waker>,
}

struct Finish(Arc<Mutex<State>>);

impl Finish {
    fn finish(&self) {
        let waker = {
            let mut state = lock_or_recover(&self.0);
            state.done = true;
            state.waker.take()
        };

        if let Some(waker) = waker {
            waker.wake();
        }
    }
}

impl Drop for Finish {
    fn drop(&mut self) {
        self.finish();
    }
}

/// The completion of an engine latent function, the generated `async` bindings await: it
/// resolves when the engine's latent action finishes, or when the engine drops it.
///
/// Dropping it before then aborts the engine's latent action.
#[doc(hidden)]
pub struct LatentCall {
    id: u64,
    state: Arc<Mutex<State>>,
}

impl LatentCall {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let state = Arc::new(Mutex::new(State::default()));
        let finish = Finish(state.clone());
        let id = register_callback(move |_| finish.finish());

        LatentCall { id, state }
    }

    pub fn id(&self) -> u64 {
        self.id
    }
}

impl Future for LatentCall {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let mut state = lock_or_recover(&self.state);

        if state.done {
            return Poll::Ready(());
        }

        state.waker = Some(cx.waker().clone());
        Poll::Pending
    }
}

impl Drop for LatentCall {
    fn drop(&mut self) {
        if lock_or_recover(&self.state).done {
            return;
        }

        if crate::api::is_api_initialized() {
            let _ = unsafe { crate::ffi_dispatch::delegate_latent_cancel(self.id) };
        }

        unregister_callback(self.id);
    }
}

#[doc(hidden)]
pub fn live_latent_proxies() -> u32 {
    unsafe { crate::ffi_dispatch::delegate_live_latent_proxies() }
}

struct FinishAction {
    action: Arc<std::sync::atomic::AtomicU64>,
    completed: bool,
}

impl Drop for FinishAction {
    fn drop(&mut self) {
        let action = self.action.load(std::sync::atomic::Ordering::SeqCst);

        if action != 0 && crate::api::is_api_initialized() {
            unsafe { crate::ffi_dispatch::delegate_latent_function_finish(action, self.completed) };
        }
    }
}

/// Runs an async `#[ufunction]` called as a Blueprint latent node: the node's output fires when
/// `future` finishes; the future is dropped, and the output does not fire, when the node is
/// aborted or `owner` is destroyed.
///
/// A second call while the node's action runs does nothing, as a Blueprint latent node does.
///
/// # Safety
/// `info` points to the `FLatentActionInfo` parameter of the call.
#[doc(hidden)]
pub unsafe fn start_latent_function(
    owner: rusteal_ffi::UObjectHandle,
    info: *const u8,
    future: impl Future<Output = ()> + 'static,
) {
    let action = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let finish = FinishAction {
        action: action.clone(),
        completed: false,
    };

    let task = crate::executor::spawn_owned_by(owner, async move {
        let mut finish = finish;
        future.await;
        finish.completed = true;
    });

    let abort = task.abort_handle();
    let abort_id = register_callback(move |_| abort.abort());

    let id = unsafe { crate::ffi_dispatch::delegate_latent_function_begin(owner, info, abort_id) };

    if id == 0 {
        unregister_callback(abort_id);
        task.abort();
        return;
    }

    action.store(id, std::sync::atomic::Ordering::SeqCst);
}

#[doc(hidden)]
pub fn live_latent_functions() -> u32 {
    unsafe { crate::ffi_dispatch::delegate_live_latent_functions() }
}

/// The `UScriptStruct` of `FLatentActionInfo`, the hidden parameter of a latent `#[ufunction]`.
#[doc(hidden)]
pub fn latent_info_struct() -> rusteal_ffi::UStructHandle {
    let name = "LatentActionInfo";
    unsafe { crate::ffi_dispatch::reflection_find_struct(name.as_ptr(), name.len() as u32) }
}
