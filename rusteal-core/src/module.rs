use std::cell::Cell;
use std::ffi::c_void;
use std::sync::atomic::{AtomicPtr, AtomicU8, Ordering};
use std::sync::{Mutex, OnceLock};

use rusteal_ffi::RustealApiTable;

use crate::lock_or_recover;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ModuleId(u32);

impl ModuleId {
    pub fn index(self) -> u32 {
        self.0
    }
}

pub struct FuncTable {
    table: AtomicPtr<*const c_void>,
    count: u32,
    fingerprint: u64,
}

impl FuncTable {
    pub const fn new(count: u32, fingerprint: u64) -> Self {
        Self {
            table: AtomicPtr::new(std::ptr::null_mut()),
            count,
            fingerprint,
        }
    }

    pub fn count(&self) -> u32 {
        self.count
    }

    pub fn fingerprint(&self) -> u64 {
        self.fingerprint
    }

    pub fn bind(&self, api: &RustealApiTable) -> Result<(), String> {
        if api.func_count != self.count {
            return Err(format!(
                "the bindings call {} engine functions and the plugin's C++ wrappers provide {}",
                self.count, api.func_count
            ));
        }

        if api.func_fingerprint != 0 && api.func_fingerprint != self.fingerprint {
            return Err(format!(
                "the bindings ({:016x}) and the plugin's C++ wrappers ({:016x}) describe different engine functions",
                self.fingerprint, api.func_fingerprint
            ));
        }

        if self.count > 0 && api.func_table.is_null() {
            return Err("the plugin gave no engine function table".to_string());
        }

        self.table
            .store(api.func_table as *mut *const c_void, Ordering::Release);

        Ok(())
    }

    #[inline]
    pub fn get(&self, id: u32) -> *const c_void {
        let table = self.table.load(Ordering::Acquire);

        assert!(
            !table.is_null(),
            "the engine function table is not bound: the library was not initialized by its entry!()"
        );

        assert!(
            id < self.count,
            "engine function {id} is out of the table's {} entries",
            self.count
        );

        unsafe { *table.add(id as usize) }
    }
}

const IDLE: u8 = 0;
const STARTING: u8 = 1;
const RUNNING: u8 = 2;
const STOPPING: u8 = 3;

pub struct Module {
    krate: &'static str,
    source_root: &'static str,
    funcs: &'static FuncTable,
    id: OnceLock<ModuleId>,
    crates: OnceLock<&'static [&'static str]>,
    state: AtomicU8,
}

impl Module {
    pub const fn new(
        krate: &'static str,
        source_root: &'static str,
        funcs: &'static FuncTable,
    ) -> Self {
        Self {
            krate,
            source_root,
            funcs,
            id: OnceLock::new(),
            crates: OnceLock::new(),
            state: AtomicU8::new(IDLE),
        }
    }

    pub fn krate(&self) -> &'static str {
        self.krate
    }

    pub fn source_root(&self) -> &'static str {
        self.source_root
    }

    pub fn funcs(&self) -> &'static FuncTable {
        self.funcs
    }

    pub fn id(&'static self) -> ModuleId {
        *self.id.get_or_init(|| {
            let mut modules = lock_or_recover(&MODULES);
            let id = ModuleId(modules.len() as u32);
            modules.push(self);
            id
        })
    }

    pub fn claim_crates(&self, crates: &'static [&'static str]) {
        let _ = self.crates.set(crates);
    }

    pub fn owns(&self, krate: &str) -> bool {
        match self.crates.get() {
            Some(crates) => crates.contains(&krate),
            None => true,
        }
    }

    pub fn is_running(&self) -> bool {
        self.state.load(Ordering::Acquire) == RUNNING
    }

    pub fn enter(&'static self) -> ModuleScope {
        scope(self.id())
    }

    pub fn begin_start(&self) -> bool {
        self.state
            .compare_exchange(IDLE, STARTING, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }

    pub fn finish_start(&self) {
        self.state.store(RUNNING, Ordering::Release);
    }

    pub fn begin_stop(&self) -> bool {
        let mut current = self.state.load(Ordering::Acquire);

        loop {
            if current != STARTING && current != RUNNING {
                return false;
            }

            match self.state.compare_exchange(
                current,
                STOPPING,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return true,
                Err(seen) => current = seen,
            }
        }
    }

    pub fn finish_stop(&self) {
        self.state.store(IDLE, Ordering::Release);
    }
}

static MODULES: Mutex<Vec<&'static Module>> = Mutex::new(Vec::new());

thread_local! {
    static CURRENT: Cell<Option<ModuleId>> = const { Cell::new(None) };
}

pub fn current() -> ModuleId {
    CURRENT.with(Cell::get).unwrap_or(ModuleId(0))
}

pub fn get(id: ModuleId) -> Option<&'static Module> {
    lock_or_recover(&MODULES).get(id.0 as usize).copied()
}

pub fn current_module() -> Option<&'static Module> {
    get(current())
}

#[must_use]
pub struct ModuleScope {
    previous: Option<ModuleId>,
    _not_send: std::marker::PhantomData<*const ()>,
}

pub fn scope(id: ModuleId) -> ModuleScope {
    let previous = CURRENT.with(|current| current.replace(Some(id)));

    ModuleScope {
        previous,
        _not_send: std::marker::PhantomData,
    }
}

impl Drop for ModuleScope {
    fn drop(&mut self) {
        CURRENT.with(|current| current.set(self.previous));
    }
}

pub const fn type_id(path: &str) -> u64 {
    let bytes = path.as_bytes();
    let mut hash: u64 = 0xcbf29ce484222325;
    let mut i = 0;

    while i < bytes.len() {
        hash ^= bytes[i] as u64;
        hash = hash.wrapping_mul(0x100000001b3);
        i += 1;
    }

    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    static FUNCS: FuncTable = FuncTable::new(2, 7);
    static GAME: Module = Module::new("game", "", &FUNCS);
    static PLUGIN: Module = Module::new("plugin", "", &FUNCS);

    #[test]
    fn scopes_nest_and_restore() {
        let game = GAME.id();
        let plugin = PLUGIN.id();
        assert_ne!(game, plugin);

        {
            let _outer = GAME.enter();
            assert_eq!(current(), game);

            {
                let _inner = PLUGIN.enter();
                assert_eq!(current(), plugin);
                assert_eq!(current_module().unwrap().krate(), "plugin");
            }

            assert_eq!(current(), game);
        }

        assert_eq!(GAME.id(), game, "an id is assigned once");
    }

    #[test]
    fn a_module_owns_every_crate_until_it_claims_some() {
        static FREE: Module = Module::new("free", "", &FUNCS);
        static CLAIMED: Module = Module::new("claimed", "", &FUNCS);
        assert!(FREE.owns("anything"));

        CLAIMED.claim_crates(&["claimed", "claimed_helpers"]);
        assert!(CLAIMED.owns("claimed_helpers"));
        assert!(!CLAIMED.owns("free"));
    }

    #[test]
    fn shutdown_runs_once_per_start() {
        static M: Module = Module::new("lifecycle", "", &FUNCS);

        assert!(!M.begin_stop(), "nothing to stop before a start");
        assert!(M.begin_start());
        assert!(!M.begin_start(), "a second start is refused");
        assert!(M.begin_stop(), "a partial start can be stopped");
        M.finish_stop();

        assert!(M.begin_start());
        M.finish_start();
        assert!(M.is_running());
        assert!(M.begin_stop());
        assert!(!M.begin_stop(), "a second stop is a no-op");
        M.finish_stop();
        assert!(!M.is_running());
    }

    #[test]
    fn type_ids_are_qualified_by_path() {
        assert_ne!(type_id("inventory::Item"), type_id("quest::Item"));
        assert_eq!(type_id("inventory::Item"), type_id("inventory::Item"));
        const AT_COMPILE_TIME: u64 = type_id("game::Player");
        assert_eq!(AT_COMPILE_TIME, type_id("game::Player"));
    }

    fn table(count: u32, fingerprint: u64, funcs: *const *const c_void) -> RustealApiTable {
        RustealApiTable {
            version: 0,
            core: std::ptr::null(),
            property: std::ptr::null(),
            reflection: std::ptr::null(),
            container: std::ptr::null(),
            delegate: std::ptr::null(),
            lifecycle: std::ptr::null(),
            reify: std::ptr::null(),
            world: std::ptr::null(),
            logging: std::ptr::null(),
            widget: std::ptr::null(),
            input: std::ptr::null(),
            console: std::ptr::null(),
            func_table: funcs,
            func_count: count,
            func_fingerprint: fingerprint,
        }
    }

    #[test]
    fn a_function_table_binds_only_to_its_own_wrappers() {
        static ENTRIES: [usize; 2] = [11, 22];
        let entries = ENTRIES.as_ptr() as *const *const c_void;
        let funcs = FuncTable::new(2, 7);

        assert!(funcs.bind(&table(3, 7, entries)).is_err(), "count differs");
        assert!(
            funcs.bind(&table(2, 8, entries)).is_err(),
            "fingerprint differs"
        );
        assert!(funcs.bind(&table(2, 7, std::ptr::null())).is_err());

        funcs
            .bind(&table(2, 0, entries))
            .expect("no fingerprint is accepted");
        funcs.bind(&table(2, 7, entries)).unwrap();
        assert_eq!(funcs.get(1) as usize, 22);
    }

    #[test]
    #[should_panic(expected = "out of the table")]
    fn ids_past_the_table_panic() {
        static ENTRIES: [usize; 1] = [1];
        let funcs = FuncTable::new(1, 0);

        funcs
            .bind(&table(1, 0, ENTRIES.as_ptr() as *const *const c_void))
            .unwrap();

        funcs.get(1);
    }
}
