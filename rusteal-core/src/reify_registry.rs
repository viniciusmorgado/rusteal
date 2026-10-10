use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock, RwLock};

use crate::{lock_or_recover, read_or_recover, write_or_recover};

pub struct ClassRegistration {
    pub type_id: u64,
    pub shape: u64,
    pub create: fn(last_try: bool, shape: u64) -> bool,
    pub register: fn(),
    pub finalize: fn(),
    pub after_defaults: fn(),
}
inventory::collect!(ClassRegistration);

pub struct StructRegistration {
    pub create: fn(),
    pub register: fn(),
    pub finalize: fn(),
}
inventory::collect!(StructRegistration);

pub struct ClassFunctionRegistration {
    pub type_id: u64,
    pub shape: u64,
    pub register_functions: fn(),
    pub class_defaults: Option<fn()>,
}
inventory::collect!(ClassFunctionRegistration);

pub trait ClassDefaultsOutcome {
    fn report(self, class: &str);
}

impl ClassDefaultsOutcome for () {
    fn report(self, _class: &str) {}
}

impl ClassDefaultsOutcome for crate::error::RustealResult<()> {
    fn report(self, class: &str) {
        if let Err(e) = self {
            let msg = format!("[Rusteal] {class}: class defaults failed: {e}");
            let bytes = msg.as_bytes();

            unsafe {
                crate::ffi_dispatch::logging_log(1, bytes.as_ptr(), bytes.len() as u32);
            }
        }
    }
}

pub fn register_all_from_inventory() {
    let mut pending: Vec<&ClassRegistration> =
        inventory::iter::<ClassRegistration>.into_iter().collect();

    let class_count = pending.len() as u32;
    let mut ordered: Vec<&ClassRegistration> = Vec::with_capacity(pending.len());

    loop {
        let before = pending.len();

        pending.retain(|reg| {
            if (reg.create)(false, class_shape(reg)) {
                ordered.push(reg);

                false
            } else {
                true
            }
        });

        if pending.is_empty() || pending.len() == before {
            break;
        }
    }

    for reg in pending {
        (reg.create)(true, class_shape(reg));
    }

    for sreg in inventory::iter::<StructRegistration> {
        (sreg.create)();
    }

    for sreg in inventory::iter::<StructRegistration> {
        (sreg.register)();
    }

    for sreg in inventory::iter::<StructRegistration> {
        (sreg.finalize)();
    }

    for reg in &ordered {
        (reg.register)();
    }

    let mut func_reg_count = 0u32;

    for freg in inventory::iter::<ClassFunctionRegistration> {
        (freg.register_functions)();
        func_reg_count += 1;
    }

    for reg in &ordered {
        (reg.finalize)();

        for freg in inventory::iter::<ClassFunctionRegistration> {
            if freg.type_id == reg.type_id
                && let Some(class_defaults) = freg.class_defaults
            {
                class_defaults();
            }
        }

        (reg.after_defaults)();
    }

    let total_funcs = read_or_recover(func_registry()).len();

    let msg = format!(
        "[Rusteal] register_all_from_inventory: {class_count} classes, {func_reg_count} impl blocks, {total_funcs} function callbacks",
    );

    let bytes = msg.as_bytes();

    unsafe {
        crate::ffi_dispatch::logging_log(0, bytes.as_ptr(), bytes.len() as u32);
    }
}

fn class_shape(class: &ClassRegistration) -> u64 {
    let mut blocks: Vec<u64> = inventory::iter::<ClassFunctionRegistration>
        .into_iter()
        .filter(|block| block.type_id == class.type_id)
        .map(|block| block.shape)
        .collect();

    blocks.sort_unstable();
    combine_shapes(class.shape, &blocks)
}

fn combine_shapes(class: u64, blocks: &[u64]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;

    for part in std::iter::once(class).chain(blocks.iter().copied()) {
        for byte in part.to_le_bytes() {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
    }

    hash
}

use rusteal_ffi::UObjectHandle;

pub struct RustTypeInfo {
    pub name: &'static str,
    pub construct_fn: fn() -> *mut u8,
    pub drop_fn: unsafe fn(*mut u8),
}

use crate::ffi_dispatch::NativePtr;

type ReifyFunctionCallback = Arc<dyn Fn(UObjectHandle, *mut u8, NativePtr) + Send + Sync>;

struct FunctionEntry {
    callback: ReifyFunctionCallback,
    type_id: u64,
}

static TYPE_REGISTRY: OnceLock<Mutex<HashMap<u64, RustTypeInfo>>> = OnceLock::new();
static FUNC_REGISTRY: OnceLock<RwLock<Vec<FunctionEntry>>> = OnceLock::new();
static INSTANCE_DATA: OnceLock<RwLock<HashMap<u64, Vec<InstanceEntry>>>> = OnceLock::new();

struct InstanceEntry {
    data: *mut u8,
    type_id: u64,
}

unsafe impl Send for InstanceEntry {}
unsafe impl Sync for InstanceEntry {}

fn type_registry() -> &'static Mutex<HashMap<u64, RustTypeInfo>> {
    TYPE_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

fn func_registry() -> &'static RwLock<Vec<FunctionEntry>> {
    FUNC_REGISTRY.get_or_init(|| RwLock::new(Vec::new()))
}

fn instance_data() -> &'static RwLock<HashMap<u64, Vec<InstanceEntry>>> {
    INSTANCE_DATA.get_or_init(|| RwLock::new(HashMap::new()))
}

pub fn register_type(type_id: u64, info: RustTypeInfo) {
    lock_or_recover(type_registry()).insert(type_id, info);
}

pub fn register_function<F>(type_id: u64, f: F) -> u64
where
    F: Fn(UObjectHandle, *mut u8, NativePtr) + Send + Sync + 'static,
{
    let mut vec = write_or_recover(func_registry());
    let id = vec.len() as u64;

    vec.push(FunctionEntry {
        callback: Arc::new(f),
        type_id,
    });

    id
}

pub fn construct_instance(obj: UObjectHandle, type_id: u64) {
    let types = lock_or_recover(type_registry());

    let Some(info) = types.get(&type_id) else {
        if crate::api::is_api_initialized() {
            let msg = format!("[Rusteal] construct_instance: unknown type_id {type_id}");
            let bytes = msg.as_bytes();

            unsafe {
                crate::ffi_dispatch::logging_log(1, bytes.as_ptr(), bytes.len() as u32);
            }
        }

        return;
    };

    let data = (info.construct_fn)();
    drop(types);

    let key = obj.to_addr();

    let old = {
        let mut map = write_or_recover(instance_data());
        let entries = map.entry(key).or_default();
        match entries.iter_mut().find(|e| e.type_id == type_id) {
            Some(entry) => Some(std::mem::replace(&mut entry.data, data)),
            None => {
                entries.push(InstanceEntry { data, type_id });

                None
            }
        }
    };

    if let Some(old) = old {
        drop_data(type_id, old);
    }
}

fn drop_data(type_id: u64, data: *mut u8) {
    let types = lock_or_recover(type_registry());

    if let Some(info) = types.get(&type_id) {
        unsafe {
            (info.drop_fn)(data);
        }
    }
}

pub fn drop_instance(obj: UObjectHandle, _type_id: u64) {
    let key = obj.to_addr();
    let entries = write_or_recover(instance_data()).remove(&key);

    for entry in entries.into_iter().flatten() {
        drop_data(entry.type_id, entry.data);
    }
}

pub fn invoke_function(callback_id: u64, obj: UObjectHandle, params: NativePtr) {
    let func = {
        let vec = read_or_recover(func_registry());
        vec.get(callback_id as usize)
            .map(|entry| (entry.callback.clone(), entry.type_id))
    };

    if let Some((func, type_id)) = func {
        let rust_data = get_instance_data(obj, type_id);
        func(obj, rust_data, params);
    } else if crate::api::is_api_initialized() {
        let vec_len = read_or_recover(func_registry()).len();

        let msg = format!(
            "[Rusteal] invoke_function: callback_id {callback_id} not found (registry size = {vec_len})",
        );

        let bytes = msg.as_bytes();

        unsafe {
            crate::ffi_dispatch::logging_log(1, bytes.as_ptr(), bytes.len() as u32);
        }
    }
}

pub fn clear_all() {
    if let Some(instances) = INSTANCE_DATA.get() {
        let mut map = write_or_recover(instances);
        let types = lock_or_recover(type_registry());

        for entry in map.drain().flat_map(|(_, entries)| entries) {
            if let Some(info) = types.get(&entry.type_id) {
                unsafe {
                    (info.drop_fn)(entry.data);
                }
            }
        }

        drop(types);
    }

    if let Some(funcs) = FUNC_REGISTRY.get() {
        write_or_recover(funcs).clear();
    }

    if let Some(types) = TYPE_REGISTRY.get() {
        lock_or_recover(types).clear();
    }
}

pub fn get_instance_data(obj: UObjectHandle, type_id: u64) -> *mut u8 {
    let key = obj.to_addr();

    read_or_recover(instance_data())
        .get(&key)
        .and_then(|entries| entries.iter().find(|e| e.type_id == type_id))
        .map(|e| e.data)
        .unwrap_or(std::ptr::null_mut())
}

#[cfg(test)]
mod tests {
    use super::combine_shapes;

    #[test]
    fn every_part_of_a_class_makes_its_shape() {
        let shape = combine_shapes(1, &[2, 3]);
        assert_eq!(shape, combine_shapes(1, &[2, 3]));
        assert_ne!(shape, combine_shapes(9, &[2, 3]));
        assert_ne!(shape, combine_shapes(1, &[2, 4]));
        assert_ne!(shape, combine_shapes(1, &[2]));
    }
}
