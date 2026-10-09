use std::ffi::c_void;

use crate::error::RustealErrorCode;
use crate::handles::*;
use crate::reify_types::RustealReifyPropExtra;

pub use crate::handles::FWeakObjectHandle;

#[repr(C)]
pub struct RustealApiTable {
    pub version: u32,

    pub core: *const RustealCoreApi,
    pub property: *const RustealPropertyApi,
    pub reflection: *const RustealReflectionApi,
    pub container: *const RustealContainerApi,
    pub delegate: *const RustealDelegateApi,
    pub lifecycle: *const RustealLifecycleApi,
    pub reify: *const RustealReifyApi,
    pub world: *const RustealWorldApi,
    pub logging: *const RustealLoggingApi,
    pub widget: *const RustealWidgetApi,
    pub input: *const RustealInputApi,
    pub console: *const RustealConsoleApi,

    pub func_table: *const *const c_void,
    pub func_count: u32,
}

unsafe impl Send for RustealApiTable {}
unsafe impl Sync for RustealApiTable {}

#[repr(C)]
pub struct RustealCoreApi {
    pub is_valid: unsafe extern "C" fn(obj: UObjectHandle) -> bool,

    pub get_name: unsafe extern "C" fn(
        obj: UObjectHandle,
        buf: *mut u8,
        buf_len: u32,
        out_len: *mut u32,
    ) -> RustealErrorCode,

    pub get_class: unsafe extern "C" fn(obj: UObjectHandle) -> UClassHandle,

    pub is_a: unsafe extern "C" fn(obj: UObjectHandle, target_class: UClassHandle) -> bool,

    pub get_outer: unsafe extern "C" fn(obj: UObjectHandle) -> UObjectHandle,

    pub make_fname: unsafe extern "C" fn(name_utf8: *const u8, name_len: u32) -> FNameHandle,

    pub fname_to_string: unsafe extern "C" fn(
        handle: FNameHandle,
        buf: *mut u8,
        buf_len: u32,
        out_len: *mut u32,
    ) -> RustealErrorCode,

    pub make_weak: unsafe extern "C" fn(obj: UObjectHandle) -> FWeakObjectHandle,

    pub resolve_weak: unsafe extern "C" fn(weak: FWeakObjectHandle) -> UObjectHandle,

    pub is_weak_valid: unsafe extern "C" fn(weak: FWeakObjectHandle) -> bool,
}

#[repr(C)]
pub struct RustealLoggingApi {
    pub log: unsafe extern "C" fn(level: u8, msg: *const u8, msg_len: u32),
}

#[repr(C)]
pub struct RustealLifecycleApi {
    pub add_gc_root: unsafe extern "C" fn(obj: UObjectHandle),
    pub remove_gc_root: unsafe extern "C" fn(obj: UObjectHandle),
    pub register_pinned: unsafe extern "C" fn(obj: UObjectHandle),
    pub unregister_pinned: unsafe extern "C" fn(obj: UObjectHandle),
}

#[repr(C)]
pub struct RustealPropertyApi {
    pub get_bool: unsafe extern "C" fn(
        obj: UObjectHandle,
        prop: FPropertyHandle,
        out: *mut bool,
    ) -> RustealErrorCode,
    pub set_bool: unsafe extern "C" fn(
        obj: UObjectHandle,
        prop: FPropertyHandle,
        val: bool,
    ) -> RustealErrorCode,

    pub get_i32: unsafe extern "C" fn(
        obj: UObjectHandle,
        prop: FPropertyHandle,
        out: *mut i32,
    ) -> RustealErrorCode,
    pub set_i32: unsafe extern "C" fn(
        obj: UObjectHandle,
        prop: FPropertyHandle,
        val: i32,
    ) -> RustealErrorCode,
    pub get_i64: unsafe extern "C" fn(
        obj: UObjectHandle,
        prop: FPropertyHandle,
        out: *mut i64,
    ) -> RustealErrorCode,
    pub set_i64: unsafe extern "C" fn(
        obj: UObjectHandle,
        prop: FPropertyHandle,
        val: i64,
    ) -> RustealErrorCode,
    pub get_u8: unsafe extern "C" fn(
        obj: UObjectHandle,
        prop: FPropertyHandle,
        out: *mut u8,
    ) -> RustealErrorCode,
    pub set_u8: unsafe extern "C" fn(
        obj: UObjectHandle,
        prop: FPropertyHandle,
        val: u8,
    ) -> RustealErrorCode,

    pub get_f32: unsafe extern "C" fn(
        obj: UObjectHandle,
        prop: FPropertyHandle,
        out: *mut f32,
    ) -> RustealErrorCode,
    pub set_f32: unsafe extern "C" fn(
        obj: UObjectHandle,
        prop: FPropertyHandle,
        val: f32,
    ) -> RustealErrorCode,
    pub get_f64: unsafe extern "C" fn(
        obj: UObjectHandle,
        prop: FPropertyHandle,
        out: *mut f64,
    ) -> RustealErrorCode,
    pub set_f64: unsafe extern "C" fn(
        obj: UObjectHandle,
        prop: FPropertyHandle,
        val: f64,
    ) -> RustealErrorCode,

    pub get_string: unsafe extern "C" fn(
        obj: UObjectHandle,
        prop: FPropertyHandle,
        buf: *mut u8,
        buf_len: u32,
        out_len: *mut u32,
    ) -> RustealErrorCode,
    pub set_string: unsafe extern "C" fn(
        obj: UObjectHandle,
        prop: FPropertyHandle,
        buf: *const u8,
        len: u32,
    ) -> RustealErrorCode,

    pub get_fname: unsafe extern "C" fn(
        obj: UObjectHandle,
        prop: FPropertyHandle,
        out: *mut FNameHandle,
    ) -> RustealErrorCode,
    pub set_fname: unsafe extern "C" fn(
        obj: UObjectHandle,
        prop: FPropertyHandle,
        val: FNameHandle,
    ) -> RustealErrorCode,

    pub get_object: unsafe extern "C" fn(
        obj: UObjectHandle,
        prop: FPropertyHandle,
        out: *mut UObjectHandle,
    ) -> RustealErrorCode,
    pub set_object: unsafe extern "C" fn(
        obj: UObjectHandle,
        prop: FPropertyHandle,
        val: UObjectHandle,
    ) -> RustealErrorCode,

    pub get_enum: unsafe extern "C" fn(
        obj: UObjectHandle,
        prop: FPropertyHandle,
        out: *mut i64,
    ) -> RustealErrorCode,
    pub set_enum: unsafe extern "C" fn(
        obj: UObjectHandle,
        prop: FPropertyHandle,
        val: i64,
    ) -> RustealErrorCode,

    pub get_struct: unsafe extern "C" fn(
        obj: UObjectHandle,
        prop: FPropertyHandle,
        out_buf: *mut u8,
        buf_size: u32,
    ) -> RustealErrorCode,
    pub set_struct: unsafe extern "C" fn(
        obj: UObjectHandle,
        prop: FPropertyHandle,
        in_buf: *const u8,
        buf_size: u32,
    ) -> RustealErrorCode,

    pub get_property_at: unsafe extern "C" fn(
        obj: UObjectHandle,
        prop: FPropertyHandle,
        index: u32,
        out_buf: *mut u8,
        buf_size: u32,
    ) -> RustealErrorCode,
    pub set_property_at: unsafe extern "C" fn(
        obj: UObjectHandle,
        prop: FPropertyHandle,
        index: u32,
        in_buf: *const u8,
        buf_size: u32,
    ) -> RustealErrorCode,

    pub get_soft_object_path: unsafe extern "C" fn(
        obj: UObjectHandle,
        prop: FPropertyHandle,
        buf: *mut u8,
        buf_len: u32,
        out_len: *mut u32,
    ) -> RustealErrorCode,
    pub set_soft_object_path: unsafe extern "C" fn(
        obj: UObjectHandle,
        prop: FPropertyHandle,
        buf: *const u8,
        len: u32,
    ) -> RustealErrorCode,
}

#[repr(C)]
pub struct RustealReflectionApi {
    pub find_class: unsafe extern "C" fn(name: *const u8, name_len: u32) -> UClassHandle,

    pub find_property: unsafe extern "C" fn(
        class: UClassHandle,
        name: *const u8,
        name_len: u32,
    ) -> FPropertyHandle,

    pub get_static_class: unsafe extern "C" fn(name: *const u8, name_len: u32) -> UClassHandle,

    pub get_property_size: unsafe extern "C" fn(prop: FPropertyHandle) -> u32,

    pub find_struct: unsafe extern "C" fn(name: *const u8, name_len: u32) -> UStructHandle,

    pub find_struct_property: unsafe extern "C" fn(
        ustruct: UStructHandle,
        name: *const u8,
        name_len: u32,
    ) -> FPropertyHandle,

    pub find_function:
        unsafe extern "C" fn(obj: UObjectHandle, name: *const u8, name_len: u32) -> UFunctionHandle,

    pub alloc_params: unsafe extern "C" fn(func: UFunctionHandle) -> *mut u8,

    pub free_params: unsafe extern "C" fn(func: UFunctionHandle, params: *mut u8),

    pub call_function: unsafe extern "C" fn(
        obj: UObjectHandle,
        func: UFunctionHandle,
        params: *mut u8,
    ) -> RustealErrorCode,

    pub get_function_param: unsafe extern "C" fn(
        func: UFunctionHandle,
        name: *const u8,
        name_len: u32,
    ) -> FPropertyHandle,

    pub get_property_offset: unsafe extern "C" fn(prop: FPropertyHandle) -> u32,

    pub find_function_by_class:
        unsafe extern "C" fn(cls: UClassHandle, name: *const u8, name_len: u32) -> UFunctionHandle,

    pub get_element_size: unsafe extern "C" fn(prop: FPropertyHandle) -> u32,

    pub get_struct_size: unsafe extern "C" fn(ustruct: UStructHandle) -> u32,

    pub initialize_struct:
        unsafe extern "C" fn(ustruct: UStructHandle, data: *mut u8) -> RustealErrorCode,

    pub destroy_struct:
        unsafe extern "C" fn(ustruct: UStructHandle, data: *mut u8) -> RustealErrorCode,

    pub copy_struct: unsafe extern "C" fn(
        ustruct: UStructHandle,
        dest: *mut u8,
        src: *const u8,
    ) -> RustealErrorCode,

    pub find_enum: unsafe extern "C" fn(name: *const u8, name_len: u32) -> UClassHandle,

    pub get_delegate_signature: unsafe extern "C" fn(prop: FPropertyHandle) -> UFunctionHandle,
}

#[repr(C)]
pub struct RustealContainerApi {
    pub array_len: unsafe extern "C" fn(obj: UObjectHandle, prop: FPropertyHandle) -> i32,
    pub array_get: unsafe extern "C" fn(
        obj: UObjectHandle,
        prop: FPropertyHandle,
        index: i32,
        out_buf: *mut u8,
        buf_size: u32,
        out_written: *mut u32,
    ) -> RustealErrorCode,
    pub array_set: unsafe extern "C" fn(
        obj: UObjectHandle,
        prop: FPropertyHandle,
        index: i32,
        in_buf: *const u8,
        buf_size: u32,
    ) -> RustealErrorCode,
    pub array_add: unsafe extern "C" fn(
        obj: UObjectHandle,
        prop: FPropertyHandle,
        in_buf: *const u8,
        buf_size: u32,
    ) -> RustealErrorCode,
    pub array_remove: unsafe extern "C" fn(
        obj: UObjectHandle,
        prop: FPropertyHandle,
        index: i32,
    ) -> RustealErrorCode,
    pub array_clear:
        unsafe extern "C" fn(obj: UObjectHandle, prop: FPropertyHandle) -> RustealErrorCode,
    pub array_element_size: unsafe extern "C" fn(prop: FPropertyHandle) -> u32,

    pub map_len: unsafe extern "C" fn(obj: UObjectHandle, prop: FPropertyHandle) -> i32,
    pub map_find: unsafe extern "C" fn(
        obj: UObjectHandle,
        prop: FPropertyHandle,
        key_buf: *const u8,
        key_size: u32,
        out_val_buf: *mut u8,
        val_size: u32,
        out_written: *mut u32,
    ) -> RustealErrorCode,
    pub map_add: unsafe extern "C" fn(
        obj: UObjectHandle,
        prop: FPropertyHandle,
        key_buf: *const u8,
        key_size: u32,
        val_buf: *const u8,
        val_size: u32,
    ) -> RustealErrorCode,
    pub map_remove: unsafe extern "C" fn(
        obj: UObjectHandle,
        prop: FPropertyHandle,
        key_buf: *const u8,
        key_size: u32,
    ) -> RustealErrorCode,
    pub map_clear:
        unsafe extern "C" fn(obj: UObjectHandle, prop: FPropertyHandle) -> RustealErrorCode,
    pub map_get_pair: unsafe extern "C" fn(
        obj: UObjectHandle,
        prop: FPropertyHandle,
        logical_index: i32,
        out_key_buf: *mut u8,
        key_buf_size: u32,
        out_key_written: *mut u32,
        out_val_buf: *mut u8,
        val_buf_size: u32,
        out_val_written: *mut u32,
    ) -> RustealErrorCode,

    pub set_len: unsafe extern "C" fn(obj: UObjectHandle, prop: FPropertyHandle) -> i32,
    pub set_contains: unsafe extern "C" fn(
        obj: UObjectHandle,
        prop: FPropertyHandle,
        elem_buf: *const u8,
        elem_size: u32,
    ) -> bool,
    pub set_add: unsafe extern "C" fn(
        obj: UObjectHandle,
        prop: FPropertyHandle,
        elem_buf: *const u8,
        elem_size: u32,
    ) -> RustealErrorCode,
    pub set_remove: unsafe extern "C" fn(
        obj: UObjectHandle,
        prop: FPropertyHandle,
        elem_buf: *const u8,
        elem_size: u32,
    ) -> RustealErrorCode,
    pub set_clear:
        unsafe extern "C" fn(obj: UObjectHandle, prop: FPropertyHandle) -> RustealErrorCode,
    pub set_get_element: unsafe extern "C" fn(
        obj: UObjectHandle,
        prop: FPropertyHandle,
        logical_index: i32,
        out_buf: *mut u8,
        buf_size: u32,
        out_written: *mut u32,
    ) -> RustealErrorCode,

    pub alloc_temp: unsafe extern "C" fn(prop: FPropertyHandle) -> *mut u8,

    pub free_temp: unsafe extern "C" fn(prop: FPropertyHandle, base: *mut u8),

    pub array_copy_all: unsafe extern "C" fn(
        obj: UObjectHandle,
        prop: FPropertyHandle,
        out_buf: *mut u8,
        buf_size: u32,
        out_total_written: *mut u32,
        out_count: *mut i32,
    ) -> RustealErrorCode,

    pub array_set_all: unsafe extern "C" fn(
        obj: UObjectHandle,
        prop: FPropertyHandle,
        in_buf: *const u8,
        buf_size: u32,
        count: i32,
    ) -> RustealErrorCode,

    pub map_copy_all: unsafe extern "C" fn(
        obj: UObjectHandle,
        prop: FPropertyHandle,
        out_buf: *mut u8,
        buf_size: u32,
        out_total_written: *mut u32,
        out_count: *mut i32,
    ) -> RustealErrorCode,

    pub set_copy_all: unsafe extern "C" fn(
        obj: UObjectHandle,
        prop: FPropertyHandle,
        out_buf: *mut u8,
        buf_size: u32,
        out_total_written: *mut u32,
        out_count: *mut i32,
    ) -> RustealErrorCode,
}

#[repr(C)]
pub struct RustealDelegateApi {
    pub bind_delegate: unsafe extern "C" fn(
        obj: UObjectHandle,
        prop: FPropertyHandle,
        callback_id: u64,
    ) -> RustealErrorCode,
    pub unbind_delegate:
        unsafe extern "C" fn(obj: UObjectHandle, prop: FPropertyHandle) -> RustealErrorCode,
    pub add_multicast: unsafe extern "C" fn(
        obj: UObjectHandle,
        prop: FPropertyHandle,
        callback_id: u64,
    ) -> RustealErrorCode,
    pub remove_multicast: unsafe extern "C" fn(
        obj: UObjectHandle,
        prop: FPropertyHandle,
        callback_id: u64,
    ) -> RustealErrorCode,
    pub broadcast_multicast: unsafe extern "C" fn(
        obj: UObjectHandle,
        prop: FPropertyHandle,
        params: *mut u8,
    ) -> RustealErrorCode,

    pub read_param: unsafe extern "C" fn(
        prop: FPropertyHandle,
        params_buf: *mut c_void,
        offset: u32,
        out_buf: *mut u8,
        out_buf_size: u32,
        out_written: *mut u32,
    ) -> RustealErrorCode,

    pub add_function: unsafe extern "C" fn(
        obj: UObjectHandle,
        prop: FPropertyHandle,
        target: UObjectHandle,
        name: *const u8,
        name_len: u32,
    ) -> RustealErrorCode,
}

#[repr(C)]
pub struct RustealReifyApi {
    pub create_class: unsafe extern "C" fn(
        name: *const u8,
        name_len: u32,
        parent: UClassHandle,
        rust_type_id: u64,
        shape: u64,
    ) -> UClassHandle,

    pub add_property: unsafe extern "C" fn(
        cls: UClassHandle,
        name: *const u8,
        name_len: u32,
        prop_type: u32,
        prop_flags: u64,
        extra: *const RustealReifyPropExtra,
    ) -> FPropertyHandle,

    pub add_function: unsafe extern "C" fn(
        cls: UClassHandle,
        name: *const u8,
        name_len: u32,
        callback_id: u64,
        func_flags: u32,
    ) -> UFunctionHandle,

    pub add_function_param: unsafe extern "C" fn(
        func: UFunctionHandle,
        name: *const u8,
        name_len: u32,
        prop_type: u32,
        param_flags: u64,
        extra: *const RustealReifyPropExtra,
    ) -> RustealErrorCode,

    pub finalize_class: unsafe extern "C" fn(cls: UClassHandle) -> RustealErrorCode,

    pub get_cdo: unsafe extern "C" fn(cls: UClassHandle) -> UObjectHandle,

    pub add_default_subobject: unsafe extern "C" fn(
        cls: UClassHandle,
        name: *const u8,
        name_len: u32,
        property: *const u8,
        property_len: u32,
        component_class: UClassHandle,
        flags: u32,
        attach_parent: *const u8,
        attach_len: u32,
        attach_socket: *const u8,
        socket_len: u32,
    ) -> RustealErrorCode,

    pub find_default_subobject:
        unsafe extern "C" fn(owner: UObjectHandle, name: *const u8, name_len: u32) -> UObjectHandle,

    pub set_property_metadata: unsafe extern "C" fn(
        prop: FPropertyHandle,
        key: *const u8,
        key_len: u32,
        value: *const u8,
        value_len: u32,
    ) -> RustealErrorCode,

    pub create_struct:
        unsafe extern "C" fn(name: *const u8, name_len: u32, shape: u64) -> UStructHandle,

    pub finalize_struct: unsafe extern "C" fn(strukt: UStructHandle) -> RustealErrorCode,

    pub add_delegate: unsafe extern "C" fn(
        cls: UClassHandle,
        name: *const u8,
        name_len: u32,
        prop_flags: u64,
    ) -> UFunctionHandle,

    pub add_interface:
        unsafe extern "C" fn(cls: UClassHandle, path: *const u8, path_len: u32) -> RustealErrorCode,

    pub set_class_config: unsafe extern "C" fn(
        cls: UClassHandle,
        config_name: *const u8,
        config_name_len: u32,
    ) -> RustealErrorCode,
}

pub const RUSTEAL_COMP_ROOT: u32 = 1;
pub const RUSTEAL_COMP_TRANSIENT: u32 = 2;

#[repr(C)]
pub struct RustealWidgetApi {
    pub create_widget: unsafe extern "C" fn(
        owning_object: UObjectHandle,
        widget_class: UClassHandle,
    ) -> UObjectHandle,

    pub set_root_widget: unsafe extern "C" fn(
        user_widget: UObjectHandle,
        root_widget: UObjectHandle,
    ) -> RustealErrorCode,

    pub get_widget_tree: unsafe extern "C" fn(user_widget: UObjectHandle) -> UObjectHandle,
}

#[repr(C)]
pub struct RustealInputApi {
    pub bind_action: unsafe extern "C" fn(
        actor: UObjectHandle,
        action: UObjectHandle,
        trigger_event: u8,
        function_name: *const u8,
        function_name_len: u32,
    ) -> RustealErrorCode,

    pub should_display_touch_interface: unsafe extern "C" fn() -> bool,
}

#[repr(C)]
pub struct RustealConsoleArgs {
    pub args: *const u8,
    pub args_len: u32,
    pub world: UObjectHandle,
}

#[repr(C)]
pub struct RustealConsoleApi {
    pub register_command: unsafe extern "C" fn(
        name: *const u8,
        name_len: u32,
        help: *const u8,
        help_len: u32,
        callback_id: u64,
    ) -> RustealErrorCode,

    pub register_variable: unsafe extern "C" fn(
        name: *const u8,
        name_len: u32,
        help: *const u8,
        help_len: u32,
        kind: u32,
        default_value: *const u8,
        default_len: u32,
    ) -> RustealErrorCode,

    pub unregister: unsafe extern "C" fn(name: *const u8, name_len: u32) -> RustealErrorCode,

    pub get_variable: unsafe extern "C" fn(
        name: *const u8,
        name_len: u32,
        buf: *mut u8,
        buf_len: u32,
        out_len: *mut u32,
    ) -> RustealErrorCode,

    pub set_variable: unsafe extern "C" fn(
        name: *const u8,
        name_len: u32,
        value: *const u8,
        value_len: u32,
    ) -> RustealErrorCode,
}

#[repr(C)]
pub struct RustealWorldApi {
    pub spawn_actor: unsafe extern "C" fn(
        world: UObjectHandle,
        class: UClassHandle,
        transform_buf: *const u8,
        transform_size: u32,
        owner: UObjectHandle,
    ) -> UObjectHandle,

    pub get_all_actors_of_class: unsafe extern "C" fn(
        world: UObjectHandle,
        class: UClassHandle,
        out_buf: *mut u8,
        buf_byte_size: u32,
        out_count: *mut u32,
    ) -> RustealErrorCode,

    pub find_object: unsafe extern "C" fn(
        class: UClassHandle,
        path_utf8: *const u8,
        path_len: u32,
    ) -> UObjectHandle,

    pub load_object: unsafe extern "C" fn(
        class: UClassHandle,
        path_utf8: *const u8,
        path_len: u32,
    ) -> UObjectHandle,

    pub get_world: unsafe extern "C" fn(object: UObjectHandle) -> UObjectHandle,

    pub new_object:
        unsafe extern "C" fn(outer: UObjectHandle, class: UClassHandle) -> UObjectHandle,

    pub spawn_actor_deferred: unsafe extern "C" fn(
        world: UObjectHandle,
        class: UClassHandle,
        transform_buf: *const u8,
        transform_size: u32,
        owner: UObjectHandle,
        instigator: UObjectHandle,
        collision_method: u8,
    ) -> UObjectHandle,

    pub finish_spawning: unsafe extern "C" fn(
        actor: UObjectHandle,
        transform_buf: *const u8,
        transform_size: u32,
    ) -> RustealErrorCode,

    pub channel_to_object_type: unsafe extern "C" fn(channel: u8) -> u8,

    pub find_data_table_row: unsafe extern "C" fn(
        table: UObjectHandle,
        row_name: FNameHandle,
        row_struct: UStructHandle,
    ) -> *mut u8,
}
