use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use crate::lock_or_recover;

use rusteal_ffi::{FPropertyHandle, RustealErrorCode, UObjectHandle};

use crate::error::{RustealResult, check_ffi};
use crate::ffi_dispatch::NativePtr;

type DelegateCallback = Option<Box<dyn FnMut(NativePtr) + Send>>;

static NEXT_ID: AtomicU64 = AtomicU64::new(1);
static REGISTRY: OnceLock<Mutex<HashMap<u64, DelegateCallback>>> = OnceLock::new();

fn registry() -> &'static Mutex<HashMap<u64, DelegateCallback>> {
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn register_callback(f: impl FnMut(NativePtr) + Send + 'static) -> u64 {
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    lock_or_recover(registry()).insert(id, Some(Box::new(f)));
    id
}

pub fn unregister_callback(id: u64) {
    lock_or_recover(registry()).remove(&id);
}

pub fn clear_all() {
    if let Some(reg) = REGISTRY.get() {
        lock_or_recover(reg).clear();
    }

    NEXT_ID.store(1, Ordering::Relaxed);
}

pub fn invoke(callback_id: u64, params: NativePtr) {
    let mut cb = {
        let mut reg = lock_or_recover(registry());
        reg.get_mut(&callback_id).and_then(|slot| slot.take())
    };

    if let Some(ref mut f) = cb {
        f(params);
    }

    if let Some(f) = cb {
        let mut reg = lock_or_recover(registry());

        if let Some(slot) = reg.get_mut(&callback_id)
            && slot.is_none()
        {
            *slot = Some(f);
        }
    }
}

pub struct DelegateBinding {
    callback_id: u64,
    owner: UObjectHandle,
    prop: FPropertyHandle,
    is_multicast: bool,
}

impl DelegateBinding {
    pub fn new(
        callback_id: u64,
        owner: UObjectHandle,
        prop: FPropertyHandle,
        is_multicast: bool,
    ) -> Self {
        Self {
            callback_id,
            owner,
            prop,
            is_multicast,
        }
    }

    pub fn callback_id(&self) -> u64 {
        self.callback_id
    }

    pub fn unbind(self) {}

    pub fn detach(self) {
        std::mem::forget(self);
    }
}

impl Drop for DelegateBinding {
    fn drop(&mut self) {
        unregister_callback(self.callback_id);

        if crate::api::is_api_initialized() {
            unsafe {
                if self.is_multicast {
                    let _ = crate::ffi_dispatch::delegate_remove_multicast(
                        self.owner,
                        self.prop,
                        self.callback_id,
                    );
                } else {
                    let _ = crate::ffi_dispatch::delegate_unbind_delegate(self.owner, self.prop);
                }
            }
        }
    }
}

pub fn bind_unicast(
    owner: UObjectHandle,
    prop: FPropertyHandle,
    callback: impl FnMut(NativePtr) + Send + 'static,
) -> RustealResult<DelegateBinding> {
    let id = register_callback(callback);
    let result = unsafe { crate::ffi_dispatch::delegate_bind_delegate(owner, prop, id) };

    if result != RustealErrorCode::Ok {
        unregister_callback(id);
        check_ffi(result)?;
    }

    Ok(DelegateBinding::new(id, owner, prop, false))
}

pub fn bind_multicast(
    owner: UObjectHandle,
    prop: FPropertyHandle,
    callback: impl FnMut(NativePtr) + Send + 'static,
) -> RustealResult<DelegateBinding> {
    let id = register_callback(callback);
    let result = unsafe { crate::ffi_dispatch::delegate_add_multicast(owner, prop, id) };

    if result != RustealErrorCode::Ok {
        unregister_callback(id);
        check_ffi(result)?;
    }

    Ok(DelegateBinding::new(id, owner, prop, true))
}
