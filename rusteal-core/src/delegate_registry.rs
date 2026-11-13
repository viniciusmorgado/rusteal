use crate::sync::{Mutex, OnceLock};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::lock_or_recover;
use crate::module::{self, ModuleId};

use rusteal_ffi::{FPropertyHandle, RustealErrorCode, UObjectHandle};

use crate::error::{RustealResult, check_ffi};
use crate::ffi_dispatch::NativePtr;

type DelegateCallback = Option<Box<dyn FnMut(NativePtr) + Send>>;

struct Slot {
    module: ModuleId,
    callback: DelegateCallback,
}

static NEXT_ID: AtomicU64 = AtomicU64::new(1);
static REGISTRY: OnceLock<Mutex<HashMap<u64, Slot>>> = OnceLock::new();

fn registry() -> &'static Mutex<HashMap<u64, Slot>> {
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn register_callback(f: impl FnMut(NativePtr) + Send + 'static) -> u64 {
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);

    lock_or_recover(registry()).insert(
        id,
        Slot {
            module: module::current(),
            callback: Some(Box::new(f)),
        },
    );

    id
}

pub fn unregister_callback(id: u64) {
    lock_or_recover(registry()).remove(&id);
}

pub fn clear_module(module: ModuleId) {
    if let Some(reg) = REGISTRY.get() {
        let removed: Vec<Slot> = {
            let mut reg = lock_or_recover(reg);
            let ids: Vec<u64> = reg
                .iter()
                .filter(|(_, slot)| slot.module == module)
                .map(|(id, _)| *id)
                .collect();

            ids.iter().filter_map(|id| reg.remove(id)).collect()
        };

        drop(removed);
    }
}

pub fn invoke(callback_id: u64, params: NativePtr) {
    let mut cb = {
        let mut reg = lock_or_recover(registry());
        reg.get_mut(&callback_id)
            .and_then(|slot| slot.callback.take())
    };

    if let Some(ref mut f) = cb {
        f(params);
    }

    if let Some(f) = cb {
        let mut reg = lock_or_recover(registry());

        if let Some(slot) = reg.get_mut(&callback_id)
            && slot.callback.is_none()
        {
            slot.callback = Some(f);
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
                    let _ = crate::ffi_dispatch::delegate_unbind_delegate(
                        self.owner,
                        self.prop,
                        self.callback_id,
                    );
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::module::{FuncTable, Module};
    use std::sync::Arc;

    static FUNCS: FuncTable = FuncTable::new(0, 0);
    static GAME: Module = Module::new("delegate_game", "", &FUNCS);
    static PLUGIN: Module = Module::new("delegate_plugin", "", &FUNCS);

    #[test]
    fn a_module_clears_only_its_own_closures() {
        let calls = Arc::new(AtomicU64::new(0));

        let game = {
            let _scope = GAME.enter();
            let calls = calls.clone();
            register_callback(move |_| {
                calls.fetch_add(1, Ordering::Relaxed);
            })
        };

        let plugin = {
            let _scope = PLUGIN.enter();
            let calls = calls.clone();
            register_callback(move |_| {
                calls.fetch_add(10, Ordering::Relaxed);
            })
        };

        clear_module(PLUGIN.id());
        invoke(plugin, std::ptr::null_mut());
        invoke(game, std::ptr::null_mut());
        assert_eq!(calls.load(Ordering::Relaxed), 1);

        unregister_callback(game);
    }
}
