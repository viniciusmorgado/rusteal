use std::collections::HashMap;
use std::marker::PhantomData;
use std::ops::Deref;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use crate::lock_or_recover;

use rusteal_ffi::UObjectHandle;

use crate::error::{RustealError, RustealResult};
use crate::ffi_dispatch;
use crate::object_ref::{Checked, UObjectRef};
use crate::traits::{HasParent, UeClass, UeHandle, ValidHandle};

fn alive_registry() -> &'static Mutex<HashMap<u64, Arc<AtomicBool>>> {
    static REGISTRY: OnceLock<Mutex<HashMap<u64, Arc<AtomicBool>>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn notify_pinned_destroyed(handle: UObjectHandle) {
    if let Ok(registry) = alive_registry().lock()
        && let Some(flag) = registry.get(&handle.to_addr())
    {
        flag.store(false, Ordering::Relaxed);
    }
}

pub fn clear_all() {
    if let Ok(mut registry) = alive_registry().lock() {
        registry.clear();
    }
}

#[repr(C)]
pub struct Pinned<T: UeClass> {
    handle: UObjectHandle,
    alive: Arc<AtomicBool>,
    _marker: PhantomData<*const T>,
}

unsafe impl<T: UeClass> Send for Pinned<T> {}

impl<T: UeClass> Pinned<T> {
    pub fn new(obj: UObjectRef<T>) -> RustealResult<Self> {
        if !obj.is_valid() {
            return Err(RustealError::ObjectDestroyed);
        }

        let alive = Arc::new(AtomicBool::new(true));

        lock_or_recover(alive_registry()).insert(obj.raw().to_addr(), alive.clone());

        unsafe {
            ffi_dispatch::lifecycle_add_gc_root(obj.raw());
            ffi_dispatch::lifecycle_register_pinned(obj.raw());
        }

        Ok(Pinned {
            handle: obj.raw(),
            alive,
            _marker: PhantomData,
        })
    }

    #[inline]
    pub fn is_alive(&self) -> bool {
        self.alive.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn handle(&self) -> UObjectHandle {
        self.handle
    }

    #[inline]
    pub fn as_ref(&self) -> UObjectRef<T> {
        unsafe { UObjectRef::from_raw(self.handle) }
    }

    #[inline]
    pub fn as_checked(&self) -> Checked<T> {
        debug_assert!(self.is_alive(), "Pinned object has been destroyed");

        Checked::new_unchecked(self.handle)
    }
}

impl<T: UeClass> Drop for Pinned<T> {
    fn drop(&mut self) {
        lock_or_recover(alive_registry()).remove(&self.handle.to_addr());

        unsafe {
            ffi_dispatch::lifecycle_unregister_pinned(self.handle);
            ffi_dispatch::lifecycle_remove_gc_root(self.handle);
        }
    }
}

impl<T: UeClass> ValidHandle for Pinned<T> {
    #[inline]
    fn handle(&self) -> UObjectHandle {
        debug_assert!(self.is_alive(), "Pinned object has been destroyed");

        self.handle
    }
}

impl<T: UeClass> UeHandle for Pinned<T> {
    #[inline]
    fn checked_handle(&self) -> RustealResult<UObjectHandle> {
        if self.alive.load(Ordering::Relaxed) {
            Ok(self.handle)
        } else {
            Err(RustealError::ObjectDestroyed)
        }
    }

    #[inline]
    fn raw_handle(&self) -> UObjectHandle {
        self.handle
    }
}

impl<T: UeClass> std::fmt::Debug for Pinned<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pinned")
            .field("handle", &self.handle)
            .field("alive", &self.is_alive())
            .finish()
    }
}

impl<T: HasParent> Deref for Pinned<T> {
    type Target = Pinned<T::Parent>;
    #[inline]
    fn deref(&self) -> &Pinned<T::Parent> {
        unsafe { &*(self as *const _ as *const Pinned<T::Parent>) }
    }
}
