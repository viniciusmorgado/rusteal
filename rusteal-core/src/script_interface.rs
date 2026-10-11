use std::marker::PhantomData;

use rusteal_ffi::UObjectHandle;

use crate::dynamic_call::DynamicCall;
use crate::error::{RustealError, RustealResult};
use crate::ffi_dispatch;
use crate::object_ref::UObjectRef;
use crate::traits::{UeClass, UeInterface};

#[repr(transparent)]
pub struct ScriptInterface<I: UeInterface + ?Sized> {
    handle: UObjectHandle,
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
        self.handle == other.handle
    }
}

impl<I: UeInterface + ?Sized> Eq for ScriptInterface<I> {}

impl<I: UeInterface + ?Sized> std::fmt::Debug for ScriptInterface<I> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScriptInterface")
            .field("handle", &self.handle)
            .finish()
    }
}

unsafe impl<I: UeInterface + ?Sized> Send for ScriptInterface<I> {}

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
            handle: object,
            _marker: PhantomData,
        })
    }

    #[inline]
    pub fn raw(&self) -> UObjectHandle {
        self.handle
    }

    #[inline]
    pub fn is_valid(&self) -> bool {
        unsafe { ffi_dispatch::core_is_valid(self.handle) }
    }

    pub fn object<T: UeClass>(&self) -> RustealResult<UObjectRef<T>> {
        unsafe { UObjectRef::<T>::from_raw(self.handle) }.cast()
    }

    #[track_caller]
    pub fn call(&self, function: &str) -> RustealResult<DynamicCall> {
        if !self.is_valid() {
            return Err(RustealError::object_destroyed());
        }

        DynamicCall::on_object(self.handle, function)
    }
}
