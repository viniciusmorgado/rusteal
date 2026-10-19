#![allow(clippy::missing_safety_doc)]

pub use rusteal_core as runtime;
pub use rusteal_ffi as ffi;
pub use rusteal_macros::{uclass, uclass_impl, uenum, uinterface, ustruct};
pub use rusteal_ue_flags as ue_flags;

#[doc(hidden)]
pub extern crate inventory as __inventory;

pub mod prelude;
mod ue_log;

#[cfg(feature = "engine-allocator")]
mod engine_allocator;

#[cfg(feature = "engine-allocator")]
pub use engine_allocator::EngineAllocator;

#[cfg(feature = "rayon")]
pub mod rayon;

pub use glam;

use std::panic::AssertUnwindSafe;

use runtime::Module;

#[doc(hidden)]
pub mod callbacks {
    use crate::{ffi, runtime};
    use runtime::Module;

    pub fn drop_rust_instance(module: &'static Module, handle: ffi::UObjectHandle, type_id: u64) {
        let _scope = module.enter();

        runtime::ffi_boundary((), || {
            runtime::reify_registry::drop_instance(handle, type_id);
        });
    }

    pub fn invoke_rust_function(
        module: &'static Module,
        callback_id: u64,
        obj: ffi::UObjectHandle,
        params: *mut u8,
    ) {
        let _scope = module.enter();

        runtime::ffi_boundary((), || {
            runtime::reify_registry::invoke_function(callback_id, obj, params);
        });
    }

    pub fn invoke_delegate_callback(module: &'static Module, callback_id: u64, params: *mut u8) {
        let _scope = module.enter();

        runtime::ffi_boundary((), || {
            runtime::delegate_registry::invoke(callback_id, params);
        });
    }

    pub fn release_delegate_callback(module: &'static Module, callback_id: u64) {
        let _scope = module.enter();

        runtime::ffi_boundary((), || {
            runtime::delegate_registry::unregister_callback(callback_id);
        });
    }

    pub fn construct_rust_instance(module: &'static Module, obj: ffi::UObjectHandle, type_id: u64) {
        let _scope = module.enter();

        runtime::ffi_boundary((), || {
            runtime::reify_registry::construct_instance(obj, type_id);
        });
    }

    pub fn notify_pinned_destroyed(module: &'static Module, handle: ffi::UObjectHandle) {
        let _scope = module.enter();

        runtime::ffi_boundary((), || {
            runtime::pinned::notify_pinned_destroyed(handle);
        });
    }

    pub fn on_tick(module: &'static Module, delta_seconds: f32) {
        let _scope = module.enter();

        runtime::ffi_boundary((), || {
            runtime::task::tick(delta_seconds);
        });
    }

    pub fn on_shutdown(module: &'static Module) {
        crate::shutdown(module);
    }
}

enum Start {
    Started,
    Refused,
    Failed,
}

pub unsafe fn init(
    module: &'static Module,
    callbacks: &'static ffi::RustealRustCallbacks,
    api_table: *const ffi::RustealApiTable,
) -> *const ffi::RustealRustCallbacks {
    let start = std::panic::catch_unwind(AssertUnwindSafe(|| unsafe { start(module, api_table) }))
        .unwrap_or(Start::Failed);

    match start {
        Start::Started => callbacks as *const ffi::RustealRustCallbacks,
        Start::Refused => std::ptr::null(),
        Start::Failed => {
            shutdown(module);
            std::ptr::null()
        }
    }
}

unsafe fn start(module: &'static Module, api_table: *const ffi::RustealApiTable) -> Start {
    if api_table.is_null() {
        return Start::Refused;
    }

    let table = unsafe { &*api_table };

    if table.version != ffi::RUSTEAL_VERSION {
        return Start::Refused;
    }

    runtime::init_api(api_table);

    if !module.begin_start() {
        log(
            runtime::LOG_ERROR,
            &format!(
                "[Rusteal] {}: the library is already initialized",
                module.krate()
            ),
        );

        return Start::Refused;
    }

    let _scope = module.enter();
    runtime::error::set_source_root(module.source_root());
    runtime::task::mark_game_thread();
    runtime::ffi_guard::install_panic_hook();
    ue_log::install();

    if let Err(reason) = module.funcs().bind(table) {
        log(
            runtime::LOG_ERROR,
            &format!(
                "[Rusteal] {}: {reason}: the bindings and the C++ wrappers come from different \
                 code generation runs. Rebuild the project with `rusteal build --all`.",
                module.krate()
            ),
        );

        return Start::Failed;
    }

    log(
        runtime::LOG_DISPLAY,
        &format!("[Rusteal] {}: Rust side initialized", module.krate()),
    );

    runtime::reify_registry::register_module(module);
    run_load_hooks(module);

    if let Some(expected) = module.expected() {
        let linked = manifest_of(|krate| module.owns(krate));
        let missing = runtime::reify_registry::missing(expected, &linked);

        if !missing.is_empty() {
            log(
                runtime::LOG_ERROR,
                &format!(
                    "[Rusteal] {}: {} of its registrations were not linked into the game: {}",
                    module.krate(),
                    missing.len(),
                    missing.join(", ")
                ),
            );

            return Start::Failed;
        }
    }

    module.finish_start();

    Start::Started
}

fn manifest_of(owned: impl Fn(&str) -> bool) -> Vec<String> {
    let mut entries = runtime::reify_registry::manifest(&owned);

    for hook in inventory::iter::<LoadHook> {
        if owned(hook.krate) {
            entries.push(format!("hook:{}", hook.krate));
        }
    }

    entries
}

#[doc(hidden)]
pub fn manifest() -> Vec<String> {
    manifest_of(|_| true)
}

fn log(level: u8, msg: &str) {
    if runtime::api::is_api_initialized() {
        unsafe {
            runtime::ffi_dispatch::logging_log(level, msg.as_ptr(), msg.len() as u32);
        }
    }
}

#[doc(hidden)]
pub struct LoadHook {
    pub krate: &'static str,
    pub run: fn(),
}
inventory::collect!(LoadHook);

fn run_load_hooks(module: &Module) {
    for hook in inventory::iter::<LoadHook> {
        if module.owns(hook.krate) {
            (hook.run)();
        }
    }
}

#[macro_export]
macro_rules! on_load {
    ($function:path) => {
        $crate::__inventory::submit! {
            $crate::LoadHook {
                krate: env!("CARGO_CRATE_NAME"),
                run: $function,
            }
        }
    };
}

pub fn shutdown(module: &'static Module) {
    let _ = std::panic::catch_unwind(AssertUnwindSafe(|| {
        if !module.begin_stop() {
            return;
        }

        let _scope = module.enter();
        let id = module.id();

        runtime::ffi_boundary((), runtime::task::shutdown);

        #[cfg(feature = "rayon")]
        runtime::ffi_boundary((), || rayon::shutdown(id));
        runtime::ffi_boundary((), || runtime::reify_registry::clear_module(id));
        runtime::ffi_boundary((), || runtime::delegate_registry::clear_module(id));
        runtime::ffi_boundary((), || runtime::pinned::clear_module(id));

        module.finish_stop();
    }));
}

#[macro_export]
macro_rules! entry {
    () => {
        $crate::entry!(bindings);
    };
    ($bindings:ident) => {
        #[doc(hidden)]
        pub static __RUSTEAL_MODULE: $crate::runtime::Module = $crate::runtime::Module::new(
            env!("CARGO_CRATE_NAME"),
            env!("CARGO_MANIFEST_DIR"),
            &$bindings::__RUSTEAL_FUNCS,
        );

        #[doc(hidden)]
        pub static __RUSTEAL_EXPORTS: $crate::ffi::RustealModuleExports = {
            extern "C" fn drop_rust_instance(
                handle: $crate::ffi::UObjectHandle,
                type_id: u64,
                _rust_data: *mut u8,
            ) {
                $crate::callbacks::drop_rust_instance(&__RUSTEAL_MODULE, handle, type_id)
            }

            extern "C" fn invoke_rust_function(
                callback_id: u64,
                obj: $crate::ffi::UObjectHandle,
                params: *mut u8,
            ) {
                $crate::callbacks::invoke_rust_function(&__RUSTEAL_MODULE, callback_id, obj, params)
            }

            extern "C" fn invoke_delegate_callback(callback_id: u64, params: *mut u8) {
                $crate::callbacks::invoke_delegate_callback(&__RUSTEAL_MODULE, callback_id, params)
            }

            extern "C" fn on_shutdown() {
                $crate::callbacks::on_shutdown(&__RUSTEAL_MODULE)
            }

            extern "C" fn construct_rust_instance(
                obj: $crate::ffi::UObjectHandle,
                type_id: u64,
                _is_cdo: bool,
            ) {
                $crate::callbacks::construct_rust_instance(&__RUSTEAL_MODULE, obj, type_id)
            }

            extern "C" fn notify_pinned_destroyed(handle: $crate::ffi::UObjectHandle) {
                $crate::callbacks::notify_pinned_destroyed(&__RUSTEAL_MODULE, handle)
            }

            extern "C" fn on_tick(delta_seconds: f32) {
                $crate::callbacks::on_tick(&__RUSTEAL_MODULE, delta_seconds)
            }

            extern "C" fn release_delegate_callback(callback_id: u64) {
                $crate::callbacks::release_delegate_callback(&__RUSTEAL_MODULE, callback_id)
            }

            static CALLBACKS: $crate::ffi::RustealRustCallbacks =
                $crate::ffi::RustealRustCallbacks {
                    drop_rust_instance,
                    invoke_rust_function,
                    invoke_delegate_callback,
                    on_shutdown,
                    construct_rust_instance,
                    notify_pinned_destroyed,
                    on_tick,
                    release_delegate_callback,
                };

            extern "C" fn init(
                api_table: *const $crate::ffi::RustealApiTable,
            ) -> *const $crate::ffi::RustealRustCallbacks {
                unsafe { $crate::init(&__RUSTEAL_MODULE, &CALLBACKS, api_table) }
            }

            extern "C" fn shutdown() {
                $crate::shutdown(&__RUSTEAL_MODULE)
            }

            extern "C" fn callbacks() -> *const $crate::ffi::RustealRustCallbacks {
                &CALLBACKS
            }

            $crate::ffi::RustealModuleExports {
                version: $crate::ffi::RUSTEAL_VERSION,
                init,
                shutdown,
                callbacks,
            }
        };

        $crate::__native_exports!();
    };
}

#[cfg(feature = "static-link")]
#[doc(hidden)]
#[macro_export]
macro_rules! __native_exports {
    () => {};
}

#[macro_export]
macro_rules! static_modules {
    ($(
        $name:literal => $krate:ident {
            crates: [$($owned:literal),* $(,)?],
            expected: [$($entry:literal),* $(,)?] $(,)?
        }
    ),* $(,)?) => {
        #[unsafe(no_mangle)]
        pub extern "C" fn rusteal_static_modules(
            count: *mut u32,
        ) -> *const $crate::ffi::RustealStaticModule {
            static MODULES: &[$crate::ffi::RustealStaticModule] = &[$(
                $crate::ffi::RustealStaticModule {
                    name: $name.as_ptr(),
                    name_len: $name.len() as u32,
                    exports: &$krate::__RUSTEAL_EXPORTS,
                }
            ),*];

            $(
                $krate::__RUSTEAL_MODULE.claim_crates(&[$($owned),*]);
                $krate::__RUSTEAL_MODULE.expect(&[$($entry),*]);
            )*

            if !count.is_null() {
                unsafe { *count = MODULES.len() as u32 };
            }

            MODULES.as_ptr()
        }
    };
}

#[cfg(not(feature = "static-link"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __native_exports {
    () => {
        mod __rusteal_native_entry {
            #[unsafe(no_mangle)]
            pub extern "C" fn rusteal_init(
                api_table: *const $crate::ffi::RustealApiTable,
            ) -> *const $crate::ffi::RustealRustCallbacks {
                (super::__RUSTEAL_EXPORTS.init)(api_table)
            }

            #[unsafe(no_mangle)]
            pub extern "C" fn rusteal_shutdown() {
                (super::__RUSTEAL_EXPORTS.shutdown)()
            }

            #[unsafe(no_mangle)]
            pub extern "C" fn rusteal_version() -> u32 {
                super::__RUSTEAL_EXPORTS.version
            }

            #[unsafe(no_mangle)]
            pub extern "C" fn rusteal_callbacks() -> *const $crate::ffi::RustealRustCallbacks {
                (super::__RUSTEAL_EXPORTS.callbacks)()
            }
        }
    };
}
