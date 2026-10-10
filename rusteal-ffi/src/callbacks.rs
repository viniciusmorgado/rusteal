use crate::handles::UObjectHandle;

#[repr(C)]
pub struct RustealRustCallbacks {
    pub drop_rust_instance: extern "C" fn(handle: UObjectHandle, type_id: u64, rust_data: *mut u8),

    pub invoke_rust_function: extern "C" fn(callback_id: u64, obj: UObjectHandle, params: *mut u8),

    pub invoke_delegate_callback: extern "C" fn(callback_id: u64, params: *mut u8),

    pub on_shutdown: extern "C" fn(),

    pub construct_rust_instance: extern "C" fn(obj: UObjectHandle, type_id: u64, is_cdo: bool),

    pub notify_pinned_destroyed: extern "C" fn(handle: UObjectHandle),

    pub on_tick: extern "C" fn(delta_seconds: f32),
}
