use std::marker::PhantomData;
use std::ops::Deref;
use std::panic::Location;

use rusteal_ffi::{UClassHandle, UObjectHandle};

use crate::error::{RustealError, RustealResult};
use crate::ffi_dispatch;
use crate::pinned::Pinned;
use crate::traits::{HasParent, UeClass, UeHandle, ValidHandle};

#[repr(transparent)]
pub struct UObjectRef<T: UeClass> {
    handle: UObjectHandle,
    _marker: PhantomData<*const T>,
}

impl<T: UeClass> Clone for UObjectRef<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: UeClass> Copy for UObjectRef<T> {}

impl<T: UeClass> PartialEq for UObjectRef<T> {
    fn eq(&self, other: &Self) -> bool {
        self.handle == other.handle
    }
}

impl<T: UeClass> Eq for UObjectRef<T> {}

impl<T: UeClass> std::hash::Hash for UObjectRef<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.handle.hash(state);
    }
}

unsafe impl<T: UeClass> Send for UObjectRef<T> {}

impl<T: UeClass> Default for UObjectRef<T> {
    fn default() -> Self {
        Self::null()
    }
}

impl<T: UeClass> UObjectRef<T> {
    #[inline]
    pub fn null() -> Self {
        UObjectRef {
            handle: UObjectHandle::null(),
            _marker: PhantomData,
        }
    }

    #[inline]
    pub unsafe fn from_raw(handle: UObjectHandle) -> Self {
        UObjectRef {
            handle,
            _marker: PhantomData,
        }
    }

    #[inline]
    pub fn raw(&self) -> UObjectHandle {
        self.handle
    }

    #[inline]
    pub fn is_valid(&self) -> bool {
        unsafe { ffi_dispatch::core_is_valid(self.handle) }
    }

    #[inline]
    #[track_caller]
    pub fn checked(&self) -> RustealResult<Checked<T>> {
        if self.is_valid() {
            Ok(Checked {
                handle: self.handle,
                _marker: PhantomData,
            })
        } else {
            Err(crate::error::destroyed::<T>(Location::caller()))
        }
    }

    #[track_caller]
    pub fn cast<U: UeClass>(self) -> RustealResult<UObjectRef<U>> {
        let h = self.checked()?.raw();
        let target = U::static_class();
        if unsafe { ffi_dispatch::core_is_a(h, target) } {
            Ok(UObjectRef {
                handle: self.handle,
                _marker: PhantomData,
            })
        } else {
            Err(crate::error::cast_failed(h, target, Location::caller()))
        }
    }

    #[track_caller]
    pub fn pin(self) -> RustealResult<Pinned<T>> {
        Pinned::new(self)
    }

    #[track_caller]
    pub fn get_name(&self) -> RustealResult<String> {
        let h = self.checked()?.raw();

        crate::error::object_name(h).ok_or_else(|| RustealError::object_destroyed().on_handle(h))
    }

    #[track_caller]
    pub fn get_class(&self) -> RustealResult<UClassHandle> {
        let h = self.checked()?.raw();

        Ok(unsafe { ffi_dispatch::core_get_class(h) })
    }

    #[track_caller]
    pub fn get_outer(&self) -> RustealResult<UObjectHandle> {
        let h = self.checked()?.raw();

        Ok(unsafe { ffi_dispatch::core_get_outer(h) })
    }

    #[inline]
    pub fn is_a<U: UeClass>(&self) -> bool {
        self.is_valid() && unsafe { ffi_dispatch::core_is_a(self.handle, U::static_class()) }
    }
}

impl<T: HasParent> UObjectRef<T> {
    #[inline]
    pub fn upcast(self) -> UObjectRef<T::Parent> {
        unsafe { UObjectRef::from_raw(self.handle) }
    }
}

impl<T: UeClass> UObjectRef<T> {
    #[inline]
    pub fn upcast_to<U: UeClass>(self) -> UObjectRef<U>
    where
        T: crate::traits::Inherits<U>,
    {
        unsafe { UObjectRef::from_raw(self.handle) }
    }
}

pub trait ObjectPointer: Sized {
    unsafe fn from_object_handle(handle: UObjectHandle) -> Self;

    fn object_handle(&self) -> UObjectHandle;
}

impl<T: UeClass> ObjectPointer for UObjectRef<T> {
    #[inline]
    unsafe fn from_object_handle(handle: UObjectHandle) -> Self {
        unsafe { UObjectRef::from_raw(handle) }
    }

    #[inline]
    fn object_handle(&self) -> UObjectHandle {
        self.handle
    }
}

impl<T: HasParent> Deref for UObjectRef<T> {
    type Target = UObjectRef<T::Parent>;
    #[inline]
    fn deref(&self) -> &UObjectRef<T::Parent> {
        unsafe { &*(self as *const _ as *const UObjectRef<T::Parent>) }
    }
}

impl<T: UeClass> UeHandle for UObjectRef<T> {
    #[inline]
    fn checked_handle(&self) -> RustealResult<UObjectHandle> {
        self.checked().map(|c| c.raw())
    }

    #[inline]
    fn raw_handle(&self) -> UObjectHandle {
        self.raw()
    }
}

impl<T: UeClass> std::fmt::Debug for UObjectRef<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UObjectRef")
            .field("handle", &self.handle)
            .field("valid", &self.is_valid())
            .finish()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Checked<T: UeClass> {
    handle: UObjectHandle,
    _marker: PhantomData<*const T>,
}

unsafe impl<T: UeClass> Send for Checked<T> {}

impl<T: UeClass> Checked<T> {
    #[inline]
    pub(crate) fn new_unchecked(handle: UObjectHandle) -> Self {
        Checked {
            handle,
            _marker: PhantomData,
        }
    }

    #[inline]
    pub fn raw(&self) -> UObjectHandle {
        self.handle
    }

    #[inline]
    pub fn as_ref(&self) -> UObjectRef<T> {
        unsafe { UObjectRef::from_raw(self.handle) }
    }

    #[inline]
    pub fn is_a<U: UeClass>(&self) -> bool {
        unsafe { ffi_dispatch::core_is_a(self.handle, U::static_class()) }
    }

    #[track_caller]
    pub fn cast<U: UeClass>(self) -> RustealResult<Checked<U>> {
        if self.is_a::<U>() {
            Ok(Checked::new_unchecked(self.handle))
        } else {
            Err(crate::error::cast_failed(
                self.handle,
                U::static_class(),
                Location::caller(),
            ))
        }
    }
}

impl<T: HasParent> Checked<T> {
    #[inline]
    pub fn upcast(self) -> Checked<T::Parent> {
        Checked::new_unchecked(self.handle)
    }
}

impl<T: HasParent> Deref for Checked<T> {
    type Target = Checked<T::Parent>;
    #[inline]
    fn deref(&self) -> &Checked<T::Parent> {
        unsafe { &*(self as *const _ as *const Checked<T::Parent>) }
    }
}

impl<T: UeClass> ValidHandle for Checked<T> {
    #[inline]
    fn handle(&self) -> UObjectHandle {
        self.handle
    }
}

impl<T: UeClass> std::fmt::Debug for Checked<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Checked")
            .field("handle", &self.handle)
            .finish()
    }
}
