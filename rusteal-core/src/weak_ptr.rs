use std::marker::PhantomData;

use rusteal_ffi::{FWeakObjectHandle, UObjectHandle};

use crate::containers::ContainerElement;
use crate::error::RustealResult;
use crate::ffi_dispatch;
use crate::object_ref::{Checked, ObjectPointer, UObjectRef};
use crate::traits::UeClass;

/// A reference to an object that does not keep it alive, and says when it is gone: `get()` is
/// `None` once the object is destroyed. It is a plain value, unlike [`UObjectRef`], so it can be
/// kept in fields, closures and tasks, and sent to other threads; it resolves on the game thread.
pub struct TWeakObjectPtr<T: UeClass> {
    handle: FWeakObjectHandle,
    _marker: PhantomData<fn() -> T>,
}

// By hand, so that they hold whatever `T` is: a class is not itself `Copy` or `Eq`.
impl<T: UeClass> Clone for TWeakObjectPtr<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: UeClass> Copy for TWeakObjectPtr<T> {}

impl<T: UeClass> PartialEq for TWeakObjectPtr<T> {
    fn eq(&self, other: &Self) -> bool {
        self.handle == other.handle
    }
}

impl<T: UeClass> Eq for TWeakObjectPtr<T> {}

impl<T: UeClass> std::hash::Hash for TWeakObjectPtr<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.handle.hash(state);
    }
}

impl<T: UeClass> std::fmt::Debug for TWeakObjectPtr<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TWeakObjectPtr")
            .field("index", &self.handle.object_index)
            .field("serial", &self.handle.object_serial_number)
            .finish()
    }
}

impl<T: UeClass> TWeakObjectPtr<T> {
    pub fn from_ref(obj: &UObjectRef<T>) -> Self {
        TWeakObjectPtr {
            handle: obj.weak_handle(),
            _marker: PhantomData,
        }
    }

    /// The object, while it lives.
    pub fn get(&self) -> Option<UObjectRef<T>> {
        let obj = UObjectRef::<T>::from_weak(self.handle);

        obj.is_valid().then_some(obj)
    }

    pub fn is_valid(&self) -> bool {
        UObjectRef::<T>::from_weak(self.handle).is_valid()
    }

    #[track_caller]
    pub fn checked(&self) -> RustealResult<Checked<T>> {
        UObjectRef::<T>::from_weak(self.handle).checked()
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
        UObjectRef::<T>::from_weak(self.handle).raw()
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

#[cfg(test)]
mod tests {
    use super::*;

    struct Class;

    impl UeClass for Class {
        fn static_class() -> rusteal_ffi::UClassHandle {
            rusteal_ffi::UClassHandle::null()
        }
    }

    fn copy_send_sync<T: Copy + Eq + std::hash::Hash + Send + Sync + 'static>() {}

    #[test]
    fn a_weak_pointer_to_any_class_is_a_plain_value_for_any_thread() {
        copy_send_sync::<TWeakObjectPtr<Class>>();
    }
}
