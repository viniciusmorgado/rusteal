use std::marker::PhantomData;

use rusteal_ffi::{FWeakObjectHandle, UObjectHandle};

use crate::containers::ContainerElement;
use crate::error::RustealResult;
use crate::ffi_dispatch;
use crate::object_ref::{Checked, ObjectPointer, UObjectRef};
use crate::traits::UeClass;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct TWeakObjectPtr<T: UeClass> {
    handle: FWeakObjectHandle,
    _marker: PhantomData<*const T>,
}

unsafe impl<T: UeClass> Send for TWeakObjectPtr<T> {}

impl<T: UeClass> TWeakObjectPtr<T> {
    pub fn from_ref(obj: &UObjectRef<T>) -> Self {
        let handle = unsafe { ffi_dispatch::core_make_weak(obj.raw()) };

        TWeakObjectPtr {
            handle,
            _marker: PhantomData,
        }
    }

    pub fn get(&self) -> Option<UObjectRef<T>> {
        let obj = unsafe { ffi_dispatch::core_resolve_weak(self.handle) };

        if obj.is_null() {
            None
        } else {
            Some(unsafe { UObjectRef::from_raw(obj) })
        }
    }

    pub fn is_valid(&self) -> bool {
        unsafe { ffi_dispatch::core_is_weak_valid(self.handle) }
    }

    #[track_caller]
    pub fn checked(&self) -> RustealResult<Checked<T>> {
        unsafe { UObjectRef::<T>::from_raw(ffi_dispatch::core_resolve_weak(self.handle)) }.checked()
    }

    #[inline]
    pub fn handle(&self) -> FWeakObjectHandle {
        self.handle
    }
}

impl<T: UeClass> Default for TWeakObjectPtr<T> {
    fn default() -> Self {
        TWeakObjectPtr {
            handle: FWeakObjectHandle::default(),
            _marker: PhantomData,
        }
    }
}

impl<T: UeClass> ObjectPointer for TWeakObjectPtr<T> {
    #[inline]
    unsafe fn from_object_handle(handle: UObjectHandle) -> Self {
        TWeakObjectPtr {
            handle: unsafe { ffi_dispatch::core_make_weak(handle) },
            _marker: PhantomData,
        }
    }

    #[inline]
    fn object_handle(&self) -> UObjectHandle {
        unsafe { ffi_dispatch::core_resolve_weak(self.handle) }
    }
}

unsafe impl<T: UeClass> ContainerElement for TWeakObjectPtr<T> {
    const BUF_SIZE: u32 = std::mem::size_of::<UObjectHandle>() as u32;

    #[inline]
    unsafe fn read_from_buf(buf: *const u8, _written: u32) -> Self {
        unsafe { Self::from_object_handle((buf as *const UObjectHandle).read_unaligned()) }
    }

    #[inline]
    unsafe fn write_to_buf(&self, buf: *mut u8) -> u32 {
        unsafe {
            (buf as *mut UObjectHandle).write_unaligned(self.object_handle());
        }

        std::mem::size_of::<UObjectHandle>() as u32
    }
}
