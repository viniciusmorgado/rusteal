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
