use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::sync::{Mutex, OnceLock, RwLock};

use crate::module::{self, Module, ModuleId};
use crate::{lock_or_recover, read_or_recover, write_or_recover};

pub struct ClassRegistration {
    pub krate: &'static str,
    pub name: &'static str,
    pub type_id: u64,
    pub shape: u64,
    pub depends_on: &'static [u64],
    pub create: fn(last_try: bool, shape: u64) -> bool,
    pub register: fn(),
    pub finalize: fn(),
    pub after_defaults: fn(),
}
inventory::collect!(ClassRegistration);

pub struct StructRegistration {
    pub krate: &'static str,
    pub name: &'static str,
    pub create: fn(),
    pub register: fn(),
    pub finalize: fn(),
}
inventory::collect!(StructRegistration);

pub struct EnumRegistration {
    pub krate: &'static str,
    pub name: &'static str,
    pub create: fn(),
}
inventory::collect!(EnumRegistration);

pub struct InterfaceRegistration {
    pub krate: &'static str,
    pub name: &'static str,
    pub create: fn(),
    pub register: fn(),
    pub finalize: fn(),
}
inventory::collect!(InterfaceRegistration);

pub struct ClassFunctionRegistration {
    pub krate: &'static str,
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
            e.__log_in(
                crate::logging::LOG_WARNING,
                &format!("{class}'s class defaults"),
            );
        }
    }
}

pub fn register_module(module: &'static Module) {
    let owned = |krate: &str| module.owns(krate);

    for ereg in inventory::iter::<EnumRegistration>
        .into_iter()
        .filter(|r| owned(r.krate))
    {
        (ereg.create)();
    }

    let interfaces: Vec<&InterfaceRegistration> = inventory::iter::<InterfaceRegistration>
        .into_iter()
        .filter(|r| owned(r.krate))
        .collect();

    for ireg in &interfaces {
        (ireg.create)();
    }

    let all: Vec<&'static ClassRegistration> = inventory::iter::<ClassRegistration>
        .into_iter()
        .filter(|r| owned(r.krate))
        .collect();

    let class_count = all.len() as u32;
    let local_ids: Vec<u64> = all.iter().map(|reg| reg.type_id).collect();
    let (waiting, mut pending) = split_waiting(all, &local_ids);
    let ordered = create_classes(&mut pending);

    for reg in pending {
        (reg.create)(true, class_shape(reg));
    }

    let structs: Vec<&StructRegistration> = inventory::iter::<StructRegistration>
        .into_iter()
        .filter(|r| owned(r.krate))
        .collect();

    for sreg in &structs {
        (sreg.create)();
    }

    for sreg in &structs {
        (sreg.register)();
    }

    for sreg in &structs {
        (sreg.finalize)();
    }

    for ireg in &interfaces {
        (ireg.register)();
        (ireg.finalize)();
    }

    let blocks = complete_classes(&ordered, &owned);
    let deferred = create_waiting(module, waiting);

    if !deferred.is_empty() {
        let names: Vec<&str> = deferred.iter().map(|reg| reg.name).collect();

        log(
            0,
            &format!(
                "[Rusteal] {}: {} for a parent class in another library: {}",
                module.krate(),
                if deferred.len() == 1 {
                    "one class waits"
                } else {
                    "classes wait"
                },
                names.join(", ")
            ),
        );

        lock_or_recover(&DEFERRED).extend(deferred.into_iter().map(|reg| (module.id(), reg)));
    }

    let owner = module::current();

    let total_funcs = read_or_recover(func_registry())
        .iter()
        .flatten()
        .filter(|entry| entry.module == owner)
        .count();

    let msg = format!(
        "[Rusteal] {}: registered {class_count} classes, {blocks} impl blocks, {total_funcs} function callbacks",
        module.krate(),
    );

    let bytes = msg.as_bytes();

    unsafe {
        crate::ffi_dispatch::logging_log(0, bytes.as_ptr(), bytes.len() as u32);
    }
}

static DEFERRED: Mutex<Vec<(ModuleId, &'static ClassRegistration)>> = Mutex::new(Vec::new());

fn create_classes(
    pending: &mut Vec<&'static ClassRegistration>,
) -> Vec<&'static ClassRegistration> {
    let mut ordered = Vec::with_capacity(pending.len());

    loop {
        let before = pending.len();

        pending.retain(|reg| {
            if (reg.create)(false, class_shape(reg)) {
                ordered.push(*reg);

                false
            } else {
                true
            }
        });

        if pending.is_empty() || pending.len() == before {
            break;
        }
    }

    ordered
}

type Split = (
    Vec<&'static ClassRegistration>,
    Vec<&'static ClassRegistration>,
);

fn split_waiting(pending: Vec<&'static ClassRegistration>, local_ids: &[u64]) -> Split {
    let parent = |reg: &ClassRegistration| reg.depends_on.first().copied().unwrap_or(0);

    let mut waiting: Vec<u64> = pending
        .iter()
        .filter(|reg| parent(reg) != 0 && !local_ids.contains(&parent(reg)))
        .map(|reg| reg.type_id)
        .collect();

    loop {
        let before = waiting.len();

        for reg in &pending {
            if !waiting.contains(&reg.type_id)
                && reg.depends_on.iter().any(|id| waiting.contains(id))
            {
                waiting.push(reg.type_id);
            }
        }

        if waiting.len() == before {
            break;
        }
    }

    pending
        .into_iter()
        .partition(|reg| waiting.contains(&reg.type_id))
}

fn complete_classes(
    classes: &[&'static ClassRegistration],
    owned: &impl Fn(&str) -> bool,
) -> usize {
    for reg in classes {
        (reg.register)();
    }

    let ordered = dependency_order(classes);

    let mut blocks: Vec<&ClassFunctionRegistration> = inventory::iter::<ClassFunctionRegistration>
        .into_iter()
        .filter(|r| owned(r.krate) && ordered.iter().any(|reg| reg.type_id == r.type_id))
        .collect();

    blocks.sort_by_key(|block| {
        ordered
            .iter()
            .position(|reg| reg.type_id == block.type_id)
            .unwrap_or(usize::MAX)
    });

    for freg in &blocks {
        (freg.register_functions)();
    }

    for reg in &ordered {
        (reg.finalize)();

        for freg in &blocks {
            if freg.type_id == reg.type_id
                && let Some(class_defaults) = freg.class_defaults
            {
                class_defaults();
            }
        }

        (reg.after_defaults)();
    }

    blocks.len()
}

/// Creates the classes of `module` that waited for a parent in another library, now that one
/// more library has registered; with `last_try`, the ones still waiting are reported as errors.
/// Returns how many still wait.
pub fn retry_deferred(module: &'static Module, last_try: bool) -> usize {
    let pending: Vec<&'static ClassRegistration> = {
        let mut deferred = lock_or_recover(&DEFERRED);
        let (mine, others): (Vec<_>, Vec<_>) =
            deferred.drain(..).partition(|(id, _)| *id == module.id());
        *deferred = others;
        mine.into_iter().map(|(_, reg)| reg).collect()
    };

    if pending.is_empty() {
        return 0;
    }

    let left = create_waiting(module, pending);

    if last_try {
        for reg in &left {
            (reg.create)(true, class_shape(reg));
        }

        return 0;
    }

    let count = left.len();
    lock_or_recover(&DEFERRED).extend(left.into_iter().map(|reg| (module.id(), reg)));
    count
}

fn create_waiting(
    module: &'static Module,
    mut waiting: Vec<&'static ClassRegistration>,
) -> Vec<&'static ClassRegistration> {
    let mut created: Vec<&'static ClassRegistration> = Vec::new();

    loop {
        let before = waiting.len();
        let waiting_ids: Vec<u64> = waiting.iter().map(|reg| reg.type_id).collect();

        waiting.retain(|reg| {
            let blocked = reg
                .depends_on
                .iter()
                .any(|id| waiting_ids.contains(id) && *id != reg.type_id);

            if !blocked && (reg.create)(false, class_shape(reg)) {
                created.push(*reg);
                false
            } else {
                true
            }
        });

        if waiting.is_empty() || waiting.len() == before {
            break;
        }
    }

    if !created.is_empty() {
        let owned = |krate: &str| module.owns(krate);
        complete_classes(&created, &owned);

        let names: Vec<&str> = created.iter().map(|reg| reg.name).collect();
        log(
            0,
            &format!(
                "[Rusteal] {}: created {}, whose parent is in another library",
                module.krate(),
                names.join(", ")
            ),
        );
    }

    waiting
}

pub fn manifest(owned: impl Fn(&str) -> bool) -> Vec<String> {
    let mut entries = Vec::new();

    for reg in inventory::iter::<ClassRegistration> {
        if owned(reg.krate) {
            entries.push(format!("class:{}:{}", reg.krate, reg.name));
        }
    }

    for reg in inventory::iter::<ClassFunctionRegistration> {
        if owned(reg.krate) {
            entries.push(format!("impl:{}:{:016x}", reg.krate, reg.type_id));
        }
    }

    for reg in inventory::iter::<StructRegistration> {
        if owned(reg.krate) {
            entries.push(format!("struct:{}:{}", reg.krate, reg.name));
        }
    }

    for reg in inventory::iter::<EnumRegistration> {
        if owned(reg.krate) {
            entries.push(format!("enum:{}:{}", reg.krate, reg.name));
        }
    }

    for reg in inventory::iter::<InterfaceRegistration> {
        if owned(reg.krate) {
            entries.push(format!("interface:{}:{}", reg.krate, reg.name));
        }
    }

    entries
}

pub fn missing(expected: &[&str], linked: &[String]) -> Vec<String> {
    let mut linked: Vec<&str> = linked.iter().map(String::as_str).collect();
    let mut missing = Vec::new();

    for entry in expected {
        match linked.iter().position(|seen| seen == entry) {
            Some(found) => {
                linked.swap_remove(found);
            }
            None => missing.push(entry.to_string()),
        }
    }

    missing
}

fn dependency_order<'a>(classes: &[&'a ClassRegistration]) -> Vec<&'a ClassRegistration> {
    let ids: Vec<u64> = classes.iter().map(|class| class.type_id).collect();
    let mut state = vec![0u8; classes.len()];
    let mut out = Vec::with_capacity(classes.len());

    fn visit<'a>(
        index: usize,
        classes: &[&'a ClassRegistration],
        ids: &[u64],
        state: &mut [u8],
        out: &mut Vec<&'a ClassRegistration>,
    ) {
        if state[index] != 0 {
            return;
        }

        state[index] = 1;

        for dependency in classes[index].depends_on {
            if let Some(found) = ids.iter().position(|id| id == dependency) {
                visit(found, classes, ids, state, out);
            }
        }

        state[index] = 2;
        out.push(classes[index]);
    }

    for index in 0..classes.len() {
        visit(index, classes, &ids, &mut state, &mut out);
    }

    out
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

struct TypeEntry {
    module: ModuleId,
    info: RustTypeInfo,
}

struct FunctionEntry {
    callback: ReifyFunctionCallback,
    type_id: u64,
    module: ModuleId,
}

static TYPE_REGISTRY: OnceLock<Mutex<HashMap<u64, TypeEntry>>> = OnceLock::new();
static FUNC_REGISTRY: OnceLock<RwLock<Vec<Option<FunctionEntry>>>> = OnceLock::new();
static INSTANCE_DATA: OnceLock<RwLock<HashMap<u64, Vec<InstanceEntry>>>> = OnceLock::new();

struct InstanceEntry {
    data: *mut u8,
    type_id: u64,
}

unsafe impl Send for InstanceEntry {}
unsafe impl Sync for InstanceEntry {}

fn type_registry() -> &'static Mutex<HashMap<u64, TypeEntry>> {
    TYPE_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

fn func_registry() -> &'static RwLock<Vec<Option<FunctionEntry>>> {
    FUNC_REGISTRY.get_or_init(|| RwLock::new(Vec::new()))
}

fn instance_data() -> &'static RwLock<HashMap<u64, Vec<InstanceEntry>>> {
    INSTANCE_DATA.get_or_init(|| RwLock::new(HashMap::new()))
}

fn log(level: u8, msg: &str) {
    if crate::api::is_api_initialized() {
        unsafe {
            crate::ffi_dispatch::logging_log(level, msg.as_ptr(), msg.len() as u32);
        }
    }
}

pub fn register_type(type_id: u64, info: RustTypeInfo) {
    let module = module::current();
    let mut types = lock_or_recover(type_registry());

    if let Some(existing) = types.get(&type_id)
        && existing.module != module
    {
        let other = module::get(existing.module).map_or("another library", |m| m.krate());

        log(
            2,
            &format!(
                "[Rusteal] {}: its type id {type_id} is already {other}'s {}; the class is not registered twice",
                info.name, existing.info.name
            ),
        );

        return;
    }

    types.insert(type_id, TypeEntry { module, info });
}

pub fn register_function<F>(type_id: u64, f: F) -> u64
where
    F: Fn(UObjectHandle, *mut u8, NativePtr) + Send + Sync + 'static,
{
    let mut vec = write_or_recover(func_registry());
    let id = vec.len() as u64;

    vec.push(Some(FunctionEntry {
        callback: Arc::new(f),
        type_id,
        module: module::current(),
    }));

    id
}

pub fn construct_instance(obj: UObjectHandle, type_id: u64) {
    let types = lock_or_recover(type_registry());

    let Some(entry) = types.get(&type_id) else {
        log(
            1,
            &format!("[Rusteal] construct_instance: unknown type_id {type_id}"),
        );
        return;
    };

    let data = (entry.info.construct_fn)();
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

    if let Some(entry) = types.get(&type_id) {
        unsafe {
            (entry.info.drop_fn)(data);
        }
    }
}

fn type_module(type_id: u64) -> Option<ModuleId> {
    lock_or_recover(type_registry())
        .get(&type_id)
        .map(|entry| entry.module)
}

pub fn drop_instance(obj: UObjectHandle, _type_id: u64) {
    let module = module::current();
    let key = obj.to_addr();

    let dropped: Vec<InstanceEntry> = {
        let mut map = write_or_recover(instance_data());

        let Some(entries) = map.get_mut(&key) else {
            return;
        };

        let (mine, others): (Vec<InstanceEntry>, Vec<InstanceEntry>) = std::mem::take(entries)
            .into_iter()
            .partition(|entry| type_module(entry.type_id).is_none_or(|owner| owner == module));

        if others.is_empty() {
            map.remove(&key);
        } else {
            *entries = others;
        }

        mine
    };

    for entry in dropped {
        drop_data(entry.type_id, entry.data);
    }
}

/// Whether a Rust function was called on a thread other than the game thread, where only a
/// `#[ufunction(ThreadSafe)]` runs; the first call of each is logged as an error.
#[doc(hidden)]
pub fn refused_off_game_thread(class: &str, function: &str) -> bool {
    if crate::task::is_game_thread() {
        return false;
    }

    static REPORTED: Mutex<Option<HashSet<String>>> = Mutex::new(None);

    let name = format!("{class}::{function}");
    let first = lock_or_recover(&REPORTED)
        .get_or_insert_with(HashSet::new)
        .insert(name.clone());

    if first {
        log(
            crate::LOG_ERROR,
            &format!(
                "[Rusteal] {name} was called on a worker thread and did not run: a function the \
                 engine calls off the game thread is a #[ufunction(ThreadSafe)] taking the class's \
                 ThreadSafe view"
            ),
        );
    }

    true
}

pub fn invoke_function(callback_id: u64, obj: UObjectHandle, params: NativePtr) {
    let func = {
        let vec = read_or_recover(func_registry());
        vec.get(callback_id as usize)
            .and_then(Option::as_ref)
            .map(|entry| (entry.callback.clone(), entry.type_id))
    };

    if let Some((func, type_id)) = func {
        let rust_data = get_instance_data(obj, type_id);
        func(obj, rust_data, params);
    } else {
        let vec_len = read_or_recover(func_registry()).len();

        log(
            1,
            &format!(
                "[Rusteal] invoke_function: callback_id {callback_id} not found (registry size = {vec_len})"
            ),
        );
    }
}

pub fn clear_module(module: ModuleId) {
    if let Some(instances) = INSTANCE_DATA.get() {
        let owned: Vec<u64> = lock_or_recover(type_registry())
            .iter()
            .filter(|(_, entry)| entry.module == module)
            .map(|(id, _)| *id)
            .collect();

        let dropped: Vec<InstanceEntry> = {
            let mut map = write_or_recover(instances);
            let mut dropped = Vec::new();

            map.retain(|_, entries| {
                let (mine, others): (Vec<InstanceEntry>, Vec<InstanceEntry>) =
                    std::mem::take(entries)
                        .into_iter()
                        .partition(|entry| owned.contains(&entry.type_id));

                dropped.extend(mine);
                *entries = others;
                !entries.is_empty()
            });

            dropped
        };

        for entry in dropped {
            drop_data(entry.type_id, entry.data);
        }
    }

    if let Some(funcs) = FUNC_REGISTRY.get() {
        let mut funcs = write_or_recover(funcs);

        for slot in funcs.iter_mut() {
            if slot.as_ref().is_some_and(|entry| entry.module == module) {
                *slot = None;
            }
        }

        if funcs.iter().all(Option::is_none) {
            funcs.clear();
        }
    }

    if let Some(types) = TYPE_REGISTRY.get() {
        lock_or_recover(types).retain(|_, entry| entry.module != module);
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

#[cfg(test)]
mod order_tests {
    use super::*;

    fn noop() {}

    fn create(_last_try: bool, _shape: u64) -> bool {
        true
    }

    const fn class(type_id: u64, depends_on: &'static [u64]) -> ClassRegistration {
        ClassRegistration {
            krate: "test",
            name: "Test",
            type_id,
            shape: 0,
            depends_on,
            create,
            register: noop,
            finalize: noop,
            after_defaults: noop,
        }
    }

    #[test]
    fn component_and_parent_classes_come_first() {
        static GRABBABLE: ClassRegistration = class(1, &[100, 2]);
        static GRAB: ClassRegistration = class(2, &[101]);
        static PISTOL: ClassRegistration = class(3, &[100, 2, 4]);
        static PROJECTILE: ClassRegistration = class(4, &[100]);

        let order: Vec<u64> = dependency_order(&[&GRABBABLE, &PISTOL, &GRAB, &PROJECTILE])
            .iter()
            .map(|class| class.type_id)
            .collect();

        let position = |id| order.iter().position(|o| *o == id).unwrap();
        assert!(position(2) < position(1));
        assert!(position(2) < position(3));
        assert!(position(4) < position(3));
        assert_eq!(order.len(), 4);
    }

    #[test]
    fn cycles_still_order_every_class() {
        static A: ClassRegistration = class(1, &[2]);
        static B: ClassRegistration = class(2, &[1]);

        assert_eq!(dependency_order(&[&A, &B]).len(), 2);
    }
}

#[cfg(test)]
mod manifest_tests {
    use super::missing;

    #[test]
    fn missing_registrations_are_counted() {
        let linked = vec!["class:game:Item".to_string(), "impl:game:01".to_string()];

        assert!(missing(&["class:game:Item", "impl:game:01"], &linked).is_empty());

        assert_eq!(
            missing(
                &[
                    "class:game:Item",
                    "impl:game:01",
                    "impl:game:01",
                    "enum:game:E"
                ],
                &linked
            ),
            ["impl:game:01", "enum:game:E"]
        );
    }
}

#[cfg(test)]
mod module_tests {
    use super::*;
    use crate::module::{FuncTable, Module};
    use std::sync::atomic::{AtomicUsize, Ordering};

    static FUNCS: FuncTable = FuncTable::new(0, 0);
    static GAME: Module = Module::new("reify_game", "", &FUNCS);
    static PLUGIN: Module = Module::new("reify_plugin", "", &FUNCS);
    static DROPS: AtomicUsize = AtomicUsize::new(0);

    fn info(name: &'static str) -> RustTypeInfo {
        RustTypeInfo {
            name,
            construct_fn: || Box::into_raw(Box::new(0u32)) as *mut u8,
            drop_fn: |ptr| {
                DROPS.fetch_add(1, Ordering::Relaxed);
                drop(unsafe { Box::from_raw(ptr as *mut u32) });
            },
        }
    }

    #[test]
    fn libraries_drop_and_clear_only_their_own_data() {
        let object = UObjectHandle(0x5150 as *mut std::ffi::c_void);
        let (game_type, plugin_type) = (
            crate::module::type_id("reify_game::Item"),
            crate::module::type_id("reify_plugin::Item"),
        );

        let game_fn = {
            let _scope = GAME.enter();
            register_type(game_type, info("Item"));
            construct_instance(object, game_type);
            register_function(game_type, |_, _, _| {})
        };

        let plugin_fn = {
            let _scope = PLUGIN.enter();
            register_type(plugin_type, info("Item"));
            construct_instance(object, plugin_type);
            register_function(plugin_type, |_, _, _| {})
        };

        assert_ne!(game_fn, plugin_fn);
        assert!(!get_instance_data(object, game_type).is_null());
        assert!(!get_instance_data(object, plugin_type).is_null());

        {
            let _scope = PLUGIN.enter();
            drop_instance(object, plugin_type);
        }

        assert!(get_instance_data(object, plugin_type).is_null());
        assert!(
            !get_instance_data(object, game_type).is_null(),
            "the plugin dropped the game's data"
        );

        {
            let _scope = PLUGIN.enter();
            construct_instance(object, plugin_type);
        }

        clear_module(GAME.id());
        assert!(get_instance_data(object, game_type).is_null());
        assert!(!get_instance_data(object, plugin_type).is_null());
        assert!(type_module(game_type).is_none());

        let plugin_callback_alive = read_or_recover(func_registry())
            .get(plugin_fn as usize)
            .is_some_and(Option::is_some);

        assert!(
            plugin_callback_alive,
            "clearing the game removed the plugin's functions"
        );

        clear_module(PLUGIN.id());
        assert!(get_instance_data(object, plugin_type).is_null());
        assert_eq!(DROPS.load(Ordering::Relaxed), 3);
    }
}
