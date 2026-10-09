use std::marker::PhantomData;

use rusteal_ffi::FWeakObjectHandle;

use crate::ffi_dispatch;
use crate::object_ref::UObjectRef;
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
