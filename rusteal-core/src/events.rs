use std::future::Future;
use std::marker::PhantomData;

use rusteal_ffi::{RustealNativeEvent, RustealNativeEventArgs, UObjectHandle};

use crate::delegate_registry::{register_callback, unregister_callback};
use crate::error::{RustealResult, check_ffi};
use crate::ffi_dispatch;

/// A binding to an engine event that is not a reflected delegate (`FCoreDelegates`,
/// `FWorldDelegates`, `FGameModeEvents`...): dropping it unbinds the closure, `detach()` keeps it
/// bound until the library unloads.
#[must_use = "dropping the binding unbinds it at once; call detach() to keep it"]
pub struct EventBinding {
    id: u64,
}

impl EventBinding {
    pub fn detach(self) {
        std::mem::forget(self);
    }
}

impl Drop for EventBinding {
    fn drop(&mut self) {
        if crate::api::is_api_initialized() {
            let _ = unsafe { ffi_dispatch::world_unbind_native_event(self.id) };
        }

        unregister_callback(self.id);
    }
}

/// An engine event that is not a reflected delegate, with what it gives as `A`; the bindings
/// make one per event (`core_events::pre_exit()`, `world_events::level_added()`...).
pub struct NativeEvent<A> {
    kind: RustealNativeEvent,
    target: UObjectHandle,
    convert: fn(&RustealNativeEventArgs) -> A,
    _marker: PhantomData<fn() -> A>,
}

impl<A: Send + 'static> NativeEvent<A> {
    #[doc(hidden)]
    pub const fn new(
        kind: RustealNativeEvent,
        target: UObjectHandle,
        convert: fn(&RustealNativeEventArgs) -> A,
    ) -> Self {
        NativeEvent {
            kind,
            target,
            convert,
            _marker: PhantomData,
        }
    }

    /// Binds `callback`, called with what the event gives each time it fires.
    #[track_caller]
    pub fn add(&self, mut callback: impl FnMut(A) + Send + 'static) -> RustealResult<EventBinding> {
        let convert = self.convert;

        let id = register_callback(move |params| {
            if params.is_null() {
                return;
            }

            let args = unsafe { &*(params as *const RustealNativeEventArgs) };
            callback(convert(args));
        });

        let bound = check_ffi(unsafe {
            ffi_dispatch::world_bind_native_event(self.kind as u32, self.target, id)
        });

        if let Err(e) = bound {
            unregister_callback(id);
            return Err(e);
        }

        Ok(EventBinding { id })
    }

    /// The next time the event fires: it binds now, and unbinds once it has fired.
    #[track_caller]
    pub fn next(&self) -> impl Future<Output = RustealResult<A>> + 'static {
        let (sender, receiver) = crate::event::once();
        let binding = self.add(move |value| sender.send(value));

        async move {
            let binding = binding?;
            let value = receiver.await;
            drop(binding);
            value
        }
    }
}
