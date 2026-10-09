#![allow(clippy::missing_safety_doc)]

pub use rusteal_core as runtime;
pub use rusteal_ffi as ffi;
pub use rusteal_macros::{uclass, uclass_impl, ustruct};
pub use rusteal_ue_flags as ue_flags;

#[doc(hidden)]
pub extern crate inventory as __inventory;

pub mod prelude;

pub use glam;

extern "C" fn real_drop_rust_instance(
    handle: ffi::UObjectHandle,
    type_id: u64,
    _rust_data: *mut u8,
) {
    runtime::ffi_boundary((), || {
        runtime::reify_registry::drop_instance(handle, type_id);
    });
}

extern "C" fn real_invoke_rust_function(
    callback_id: u64,
    obj: ffi::UObjectHandle,
    params: *mut u8,
) {
    runtime::ffi_boundary((), || {
        runtime::reify_registry::invoke_function(callback_id, obj, params);
    });
}

extern "C" fn real_invoke_delegate_callback(callback_id: u64, params: *mut u8) {
    runtime::ffi_boundary((), || {
        runtime::delegate_registry::invoke(callback_id, params);
    });
}

extern "C" fn real_construct_rust_instance(obj: ffi::UObjectHandle, type_id: u64, _is_cdo: bool) {
    runtime::ffi_boundary((), || {
        runtime::reify_registry::construct_instance(obj, type_id);
    });
}

extern "C" fn real_on_shutdown() {
    runtime::ffi_boundary((), || {
        runtime::task::shutdown();
        runtime::reify_registry::clear_all();
        runtime::delegate_registry::clear_all();
        runtime::pinned::clear_all();
    });
}

extern "C" fn real_notify_pinned_destroyed(handle: ffi::UObjectHandle) {
    runtime::ffi_boundary((), || {
        runtime::pinned::notify_pinned_destroyed(handle);
    });
}

extern "C" fn real_on_tick(delta_seconds: f32) {
    runtime::ffi_boundary((), || {
        runtime::task::tick(delta_seconds);
    });
}

#[doc(hidden)]
pub static __CALLBACKS: ffi::RustealRustCallbacks = ffi::RustealRustCallbacks {
    drop_rust_instance: real_drop_rust_instance,
    invoke_rust_function: real_invoke_rust_function,
    invoke_delegate_callback: real_invoke_delegate_callback,
    on_shutdown: real_on_shutdown,
    construct_rust_instance: real_construct_rust_instance,
    notify_pinned_destroyed: real_notify_pinned_destroyed,
    on_tick: real_on_tick,
};

pub unsafe fn init(api_table: *const ffi::RustealApiTable) -> *const ffi::RustealRustCallbacks {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if api_table.is_null() {
            return std::ptr::null();
        }

        if unsafe { (*api_table).version } != ffi::RUSTEAL_VERSION {
            return std::ptr::null();
        }

        runtime::init_api(api_table);
        runtime::task::mark_game_thread();

        log_greeting();
        register_all_classes();
        run_load_hooks();

        &__CALLBACKS as *const ffi::RustealRustCallbacks
    }))
    .unwrap_or(std::ptr::null())
}

fn log_greeting() {
    let msg = "[Rusteal] Rust side initialized";
    let bytes = msg.as_bytes();

    unsafe {
        runtime::ffi_dispatch::logging_log(0, bytes.as_ptr(), bytes.len() as u32);
    }
}

fn register_all_classes() {
    runtime::reify_registry::register_all_from_inventory();
}

#[doc(hidden)]
pub struct LoadHook(pub fn());
inventory::collect!(LoadHook);

fn run_load_hooks() {
    for hook in inventory::iter::<LoadHook> {
        (hook.0)();
    }
}

#[macro_export]
macro_rules! on_load {
    ($function:path) => {
        $crate::__inventory::submit! { $crate::LoadHook($function) }
    };
}

pub fn shutdown() {
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        (__CALLBACKS.on_shutdown)();
    }));
}

#[macro_export]
macro_rules! entry {
    () => {
        mod __rusteal_native_entry {
            #[unsafe(no_mangle)]
            pub extern "C" fn rusteal_init(
                api_table: *const $crate::ffi::RustealApiTable,
            ) -> *const $crate::ffi::RustealRustCallbacks {
                unsafe { $crate::init(api_table) }
            }

            #[unsafe(no_mangle)]
            pub extern "C" fn rusteal_shutdown() {
                $crate::shutdown()
            }

            #[unsafe(no_mangle)]
            pub extern "C" fn rusteal_version() -> u32 {
                $crate::ffi::RUSTEAL_VERSION
            }

            #[unsafe(no_mangle)]
            pub extern "C" fn rusteal_callbacks() -> *const $crate::ffi::RustealRustCallbacks {
                &$crate::__CALLBACKS
            }
        }
    };
}
