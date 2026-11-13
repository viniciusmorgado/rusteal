use std::marker::PhantomData;

use rusteal_ffi::{FWeakObjectHandle, UObjectHandle};

use crate::dynamic_call::DynamicCall;
use crate::error::{RustealError, RustealResult};
use crate::ffi_dispatch;
use crate::object_ref::UObjectRef;
use crate::traits::{UeClass, UeInterface};

/// An object that implements the interface `I`, kept like [`UObjectRef`]: by its index and serial
/// number, valid while the object lives.
#[repr(transparent)]
pub struct ScriptInterface<I: UeInterface + ?Sized> {
    weak: FWeakObjectHandle,
    _marker: PhantomData<*const I>,
}

impl<I: UeInterface + ?Sized> Clone for ScriptInterface<I> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<I: UeInterface + ?Sized> Copy for ScriptInterface<I> {}

impl<I: UeInterface + ?Sized> PartialEq for ScriptInterface<I> {
    fn eq(&self, other: &Self) -> bool {
        self.weak == other.weak
    }
}

impl<I: UeInterface + ?Sized> Eq for ScriptInterface<I> {}

impl<I: UeInterface + ?Sized> std::fmt::Debug for ScriptInterface<I> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScriptInterface")
            .field("index", &self.weak.object_index)
            .field("serial", &self.weak.object_serial_number)
            .finish()
    }
}

impl<I: UeInterface + ?Sized> ScriptInterface<I> {
    #[track_caller]
    pub fn new(object: UObjectHandle) -> RustealResult<Self> {
        if !unsafe { ffi_dispatch::core_is_valid(object) } {
            return Err(RustealError::object_destroyed());
        }

        if !unsafe { ffi_dispatch::core_is_a(object, I::static_class()) } {
            return Err(RustealError::invalid_cast_to(object, I::static_class()));
        }

        Ok(ScriptInterface {
            weak: unsafe { crate::object_ref::weak_handle_of(object) },
            _marker: PhantomData,
        })
    }

    /// The object's address while it lives, null once it is destroyed.
    #[inline]
    pub fn raw(&self) -> UObjectHandle {
        crate::object_ref::resolve_weak_handle(self.weak)
    }

    #[inline]
    pub fn weak_handle(&self) -> FWeakObjectHandle {
        self.weak
    }

    #[inline]
    pub fn is_valid(&self) -> bool {
        !self.raw().is_null()
    }

    #[track_caller]
    pub fn object<T: UeClass>(&self) -> RustealResult<UObjectRef<T>> {
        UObjectRef::<T>::from_weak(self.weak).cast()
    }

    #[track_caller]
    pub fn call(&self, function: &str) -> RustealResult<DynamicCall> {
        let handle = self.raw();

        if handle.is_null() {
            return Err(RustealError::object_destroyed());
        }

        DynamicCall::on_object(handle, function)
    }
}
