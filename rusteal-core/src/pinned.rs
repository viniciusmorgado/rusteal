use std::marker::PhantomData;

use rusteal_ffi::{FWeakObjectHandle, UObjectHandle};

use crate::error::{RustealError, RustealResult};
use crate::ffi_dispatch;
use crate::object_ref::{Checked, UObjectRef, resolve_weak_handle};
use crate::traits::{UeClass, UeHandle, ValidHandle};

/// Keeps an object alive for Rust code that holds it outside any class, until it is dropped: the
/// garbage collector sees it as a reference, the way it sees a `#[uproperty]`.
///
/// Each `Pinned` keeps the object on its own, a clone too, so dropping one leaves the others. An
/// object destroyed explicitly (an actor's `k2_destroy_actor`) is gone all the same, and reads as
/// gone here. It stays on the game thread, where it is dropped.
pub struct Pinned<T: UeClass> {
    slot: u32,
    weak: FWeakObjectHandle,
    _marker: PhantomData<*const T>,
}

impl<T: UeClass> Pinned<T> {
    #[track_caller]
    pub fn new(obj: UObjectRef<'_, T>) -> RustealResult<Self> {
        let handle = obj.checked()?.raw();
        let slot = unsafe { ffi_dispatch::lifecycle_pin(handle) };

        if slot == 0 {
            return Err(RustealError::object_destroyed());
        }

        Ok(Pinned {
            slot,
            weak: obj.weak_handle(),
            _marker: PhantomData,
        })
    }

    /// Whether the object still exists: it was not destroyed explicitly.
    #[inline]
    pub fn is_alive(&self) -> bool {
        !resolve_weak_handle(self.weak).is_null()
    }

    /// The object, for as long as this `Pinned` is borrowed.
    #[inline]
    pub fn as_ref(&self) -> UObjectRef<'_, T> {
        UObjectRef::from_weak(self.weak)
    }

    /// The live object, to call the engine on.
    #[inline]
    #[track_caller]
    pub fn checked(&self) -> RustealResult<Checked<'_, T>> {
        self.as_ref().checked()
    }

    /// The live object; panics in debug builds if it was destroyed explicitly.
    #[inline]
    pub fn as_checked(&self) -> Checked<'_, T> {
        let handle = resolve_weak_handle(self.weak);
        debug_assert!(!handle.is_null(), "Pinned object has been destroyed");

        Checked::new_unchecked(handle)
    }
}

impl<T: UeClass> Clone for Pinned<T> {
    /// Another pin of the same object, kept on its own.
    fn clone(&self) -> Self {
        let handle = resolve_weak_handle(self.weak);
        let slot = if handle.is_null() {
            0
        } else {
            unsafe { ffi_dispatch::lifecycle_pin(handle) }
        };

        Pinned {
            slot,
            weak: self.weak,
            _marker: PhantomData,
        }
    }
}

impl<T: UeClass> Drop for Pinned<T> {
    fn drop(&mut self) {
        if self.slot != 0 && crate::api::is_api_initialized() {
            unsafe { ffi_dispatch::lifecycle_unpin(self.slot) };
        }
    }
}

impl<T: UeClass> ValidHandle for Pinned<T> {
    #[inline]
    fn handle(&self) -> UObjectHandle {
        let handle = resolve_weak_handle(self.weak);
        debug_assert!(!handle.is_null(), "Pinned object has been destroyed");

        handle
    }
}

impl<T: UeClass> UeHandle for Pinned<T> {
    #[inline]
    fn checked_handle(&self) -> RustealResult<UObjectHandle> {
        self.checked().map(|c| c.raw())
    }

    #[inline]
    fn raw_handle(&self) -> UObjectHandle {
        resolve_weak_handle(self.weak)
    }
}

impl<T: UeClass> std::fmt::Debug for Pinned<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pinned")
            .field("slot", &self.slot)
            .field("alive", &self.is_alive())
            .finish()
    }
}
