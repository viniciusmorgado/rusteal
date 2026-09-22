// rusteal-runtime: what a game crate depends on. It carries the safe runtime,
// the FFI contract, the engine's reflection flags and the proc macros, and
// generates the library entry points through `rusteal_runtime::entry!()`.
//
// The Unreal types themselves are not here: they are generated per project by
// `rusteal build` into the game's own `bindings` crate, whose features decide
// which engine modules exist.

// Re-exports for proc macro path resolution and user access.
pub use rusteal_ffi as ffi;
pub use rusteal_core as runtime;
pub use rusteal_ue_flags as ue_flags;
pub use rusteal_macros::{uclass, uclass_impl};

// For proc macro generated inventory::submit! invocations.
#[doc(hidden)]
pub extern crate inventory as __inventory;

pub mod prelude;

// Re-export glam for convenience.
pub use glam;

// ---------------------------------------------------------------------------
// Callbacks (shared between init and entry! macro)
// ---------------------------------------------------------------------------

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

extern "C" fn real_construct_rust_instance(
    obj: ffi::UObjectHandle,
    type_id: u64,
    _is_cdo: bool,
) {
    runtime::ffi_boundary((), || {
        runtime::reify_registry::construct_instance(obj, type_id);
    });
}

extern "C" fn real_on_shutdown() {
    runtime::ffi_boundary((), || {
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

#[doc(hidden)]
pub static __CALLBACKS: ffi::RustealRustCallbacks = ffi::RustealRustCallbacks {
    drop_rust_instance: real_drop_rust_instance,
    invoke_rust_function: real_invoke_rust_function,
    invoke_delegate_callback: real_invoke_delegate_callback,
    on_shutdown: real_on_shutdown,
    construct_rust_instance: real_construct_rust_instance,
    notify_pinned_destroyed: real_notify_pinned_destroyed,
};

// ---------------------------------------------------------------------------
// Init / Shutdown (called from entry!() generated code)
// ---------------------------------------------------------------------------

/// Initialize the Rusteal runtime. Called by the `entry!()` generated `rusteal_init`.
///
/// Stores the API table, registers all reified classes, and returns the
/// callback table pointer. Returns null on failure, including a table from a
/// plugin of another Rusteal version.
///
/// # Safety
///
/// `api_table` is null or points to a table the plugin keeps alive for as long
/// as the library is loaded; at least its `version` field must be readable.
pub unsafe fn init(api_table: *const ffi::RustealApiTable) -> *const ffi::RustealRustCallbacks {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if api_table.is_null() {
            return std::ptr::null();
        }
        // The plugin writes its own version in the first field, readable
        // whatever the rest of the table looks like. Another version means
        // another table: refuse it before touching anything else.
        if unsafe { (*api_table).version } != ffi::RUSTEAL_VERSION {
            return std::ptr::null();
        }

        // Delegate API table storage to rusteal-core.
        runtime::init_api(api_table);

        log_greeting();
        register_all_classes();
        &__CALLBACKS as *const ffi::RustealRustCallbacks
    }))
    .unwrap_or(std::ptr::null())
}

/// Log the Rusteal greeting message.
fn log_greeting() {
    let msg = "[Rusteal] Rust side initialized";
    let bytes = msg.as_bytes();
    unsafe {
        runtime::ffi_dispatch::logging_log(0, bytes.as_ptr(), bytes.len() as u32);
    }
}

/// Register all Rust-defined UE classes via inventory auto-registration.
fn register_all_classes() {
    runtime::reify_registry::register_all_from_inventory();
}

/// Shut down the Rusteal runtime. Called by the `entry!()` generated `rusteal_shutdown`.
pub fn shutdown() {
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        (__CALLBACKS.on_shutdown)();
    }));
}

/// Generates DLL exports for the Rusteal runtime entry points.
///
/// Place this at the top of your cdylib crate's `lib.rs`:
/// ```ignore
/// rusteal_runtime::entry!();
/// ```
#[macro_export]
macro_rules! entry {
    () => {
        mod __rusteal_native_entry {
            #[unsafe(no_mangle)]
            pub extern "C" fn rusteal_init(
                api_table: *const $crate::ffi::RustealApiTable,
            ) -> *const $crate::ffi::RustealRustCallbacks {
                // SAFETY: called by the plugin with its own static table.
                unsafe { $crate::init(api_table) }
            }

            #[unsafe(no_mangle)]
            pub extern "C" fn rusteal_shutdown() {
                $crate::shutdown()
            }

            /// The Rusteal version this library was built with, encoded; the
            /// plugin compares it with its own before calling `rusteal_init`.
            #[unsafe(no_mangle)]
            pub extern "C" fn rusteal_version() -> u32 {
                $crate::ffi::RUSTEAL_VERSION
            }
        }
    };
}
