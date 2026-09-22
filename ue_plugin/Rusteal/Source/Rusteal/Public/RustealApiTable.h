#pragma once

// RustealApiTable.h — C++ half of the Rust ↔ C++ FFI contract.
// Every struct here must be layout-identical to the corresponding
// #[repr(C)] definition in rusteal-ffi/src/.

#include "CoreMinimal.h"

// ---------------------------------------------------------------------------
// Handle types (opaque, never dereferenced on the Rust side)
// ---------------------------------------------------------------------------

struct RustealUObjectHandle  { void* ptr; };
struct RustealUClassHandle   { void* ptr; };
struct RustealFPropertyHandle { void* ptr; };
struct RustealUFunctionHandle { void* ptr; };
struct RustealUStructHandle  { void* ptr; };
struct RustealFNameHandle    { uint64 value; };
struct RustealFWeakObjectHandle { int32 object_index; int32 object_serial_number; };

// ---------------------------------------------------------------------------
// Error codes
// ---------------------------------------------------------------------------

enum class ERustealErrorCode : uint32
{
    Ok              = 0,
    ObjectDestroyed = 1,
    InvalidCast     = 2,
    PropertyNotFound = 3,
    FunctionNotFound = 4,
    TypeMismatch    = 5,
    NullArgument    = 6,
    IndexOutOfRange = 7,
    InvalidOperation = 8,
    InternalError   = 9,
    BufferTooSmall  = 10,
};

// ---------------------------------------------------------------------------
// RustealCoreApi
// ---------------------------------------------------------------------------

struct FRustealCoreApi
{
    bool  (*is_valid)(RustealUObjectHandle obj);
    ERustealErrorCode (*get_name)(RustealUObjectHandle obj, uint8* buf, uint32 buf_len, uint32* out_len);
    RustealUClassHandle (*get_class)(RustealUObjectHandle obj);
    bool  (*is_a)(RustealUObjectHandle obj, RustealUClassHandle target_class);
    RustealUObjectHandle (*get_outer)(RustealUObjectHandle obj);

    // FName construction / conversion
    RustealFNameHandle (*make_fname)(const uint8* name_utf8, uint32 name_len);
    ERustealErrorCode  (*fname_to_string)(RustealFNameHandle handle, uint8* buf, uint32 buf_len, uint32* out_len);

    // Weak object pointers
    RustealFWeakObjectHandle (*make_weak)(RustealUObjectHandle obj);
    RustealUObjectHandle     (*resolve_weak)(RustealFWeakObjectHandle weak);
    bool                  (*is_weak_valid)(RustealFWeakObjectHandle weak);
};

// ---------------------------------------------------------------------------
// RustealLoggingApi
// ---------------------------------------------------------------------------

struct FRustealLoggingApi
{
    // level: 0=Display, 1=Warning, 2=Error.  msg is UTF-8 (not null-terminated).
    void (*log)(uint8 level, const uint8* msg, uint32 msg_len);
};

// ---------------------------------------------------------------------------
// RustealLifecycleApi
// ---------------------------------------------------------------------------

struct FRustealLifecycleApi
{
    void (*add_gc_root)(RustealUObjectHandle obj);
    void (*remove_gc_root)(RustealUObjectHandle obj);
    void (*register_pinned)(RustealUObjectHandle obj);
    void (*unregister_pinned)(RustealUObjectHandle obj);
};

// ---------------------------------------------------------------------------
// RustealPropertyApi
// ---------------------------------------------------------------------------

struct FRustealPropertyApi
{
    // Bool
    ERustealErrorCode (*get_bool)(RustealUObjectHandle obj, RustealFPropertyHandle prop, bool* out);
    ERustealErrorCode (*set_bool)(RustealUObjectHandle obj, RustealFPropertyHandle prop, bool val);
    // int32
    ERustealErrorCode (*get_i32)(RustealUObjectHandle obj, RustealFPropertyHandle prop, int32* out);
    ERustealErrorCode (*set_i32)(RustealUObjectHandle obj, RustealFPropertyHandle prop, int32 val);
    // int64
    ERustealErrorCode (*get_i64)(RustealUObjectHandle obj, RustealFPropertyHandle prop, int64* out);
    ERustealErrorCode (*set_i64)(RustealUObjectHandle obj, RustealFPropertyHandle prop, int64 val);
    // uint8
    ERustealErrorCode (*get_u8)(RustealUObjectHandle obj, RustealFPropertyHandle prop, uint8* out);
    ERustealErrorCode (*set_u8)(RustealUObjectHandle obj, RustealFPropertyHandle prop, uint8 val);
    // float
    ERustealErrorCode (*get_f32)(RustealUObjectHandle obj, RustealFPropertyHandle prop, float* out);
    ERustealErrorCode (*set_f32)(RustealUObjectHandle obj, RustealFPropertyHandle prop, float val);
    // double
    ERustealErrorCode (*get_f64)(RustealUObjectHandle obj, RustealFPropertyHandle prop, double* out);
    ERustealErrorCode (*set_f64)(RustealUObjectHandle obj, RustealFPropertyHandle prop, double val);
    // String (UTF-8)
    ERustealErrorCode (*get_string)(RustealUObjectHandle obj, RustealFPropertyHandle prop, uint8* buf, uint32 buf_len, uint32* out_len);
    ERustealErrorCode (*set_string)(RustealUObjectHandle obj, RustealFPropertyHandle prop, const uint8* buf, uint32 len);
    // FName
    ERustealErrorCode (*get_fname)(RustealUObjectHandle obj, RustealFPropertyHandle prop, RustealFNameHandle* out);
    ERustealErrorCode (*set_fname)(RustealUObjectHandle obj, RustealFPropertyHandle prop, RustealFNameHandle val);
    // Object reference
    ERustealErrorCode (*get_object)(RustealUObjectHandle obj, RustealFPropertyHandle prop, RustealUObjectHandle* out);
    ERustealErrorCode (*set_object)(RustealUObjectHandle obj, RustealFPropertyHandle prop, RustealUObjectHandle val);
    // Enum (as int64)
    ERustealErrorCode (*get_enum)(RustealUObjectHandle obj, RustealFPropertyHandle prop, int64* out);
    ERustealErrorCode (*set_enum)(RustealUObjectHandle obj, RustealFPropertyHandle prop, int64 val);
    // Struct (raw memory copy)
    ERustealErrorCode (*get_struct)(RustealUObjectHandle obj, RustealFPropertyHandle prop, uint8* out_buf, uint32 buf_size);
    ERustealErrorCode (*set_struct)(RustealUObjectHandle obj, RustealFPropertyHandle prop, const uint8* in_buf, uint32 buf_size);

    // Indexed access for fixed arrays (array_dim > 1).
    // Uses CopySingleValue internally — works for bool, numeric, enum, struct, object.
    // NOT safe for string/name/text types (requires constructed FString at dest).
    ERustealErrorCode (*get_property_at)(RustealUObjectHandle obj, RustealFPropertyHandle prop,
        uint32 index, uint8* out_buf, uint32 buf_size);
    ERustealErrorCode (*set_property_at)(RustealUObjectHandle obj, RustealFPropertyHandle prop,
        uint32 index, const uint8* in_buf, uint32 buf_size);
};

// ---------------------------------------------------------------------------
// RustealReflectionApi
// ---------------------------------------------------------------------------

struct FRustealReflectionApi
{
    RustealUClassHandle   (*find_class)(const uint8* name, uint32 name_len);
    RustealFPropertyHandle (*find_property)(RustealUClassHandle cls, const uint8* name, uint32 name_len);
    RustealUClassHandle   (*get_static_class)(const uint8* name, uint32 name_len);
    uint32             (*get_property_size)(RustealFPropertyHandle prop);

    // Struct reflection
    RustealUStructHandle  (*find_struct)(const uint8* name, uint32 name_len);
    RustealFPropertyHandle (*find_struct_property)(RustealUStructHandle ustruct, const uint8* name, uint32 name_len);

    // Reflection call support (for functions not in func_table)
    RustealUFunctionHandle (*find_function)(RustealUObjectHandle obj, const uint8* name, uint32 name_len);
    uint8*             (*alloc_params)(RustealUFunctionHandle func);
    void               (*free_params)(RustealUFunctionHandle func, uint8* params);
    ERustealErrorCode     (*call_function)(RustealUObjectHandle obj, RustealUFunctionHandle func, uint8* params);
    RustealFPropertyHandle (*get_function_param)(RustealUFunctionHandle func, const uint8* name, uint32 name_len);
    uint32             (*get_property_offset)(RustealFPropertyHandle prop);

    // Find a UFunction by class (no instance needed — for OnceLock caching)
    RustealUFunctionHandle (*find_function_by_class)(RustealUClassHandle cls, const uint8* name, uint32 name_len);

    // Get the element size of a property (FProperty::ElementSize).
    // For scalar properties, equals get_property_size().
    // For fixed arrays (array_dim > 1), equals total_size / array_dim.
    uint32 (*get_element_size)(RustealFPropertyHandle prop);

    // Get the structure size of a UScriptStruct.
    uint32 (*get_struct_size)(RustealUStructHandle ustruct);

    // Initialize struct memory using UScriptStruct default constructor.
    ERustealErrorCode (*initialize_struct)(RustealUStructHandle ustruct, uint8* data);

    // Destroy struct memory (calls C++ destructors for non-trivial members).
    ERustealErrorCode (*destroy_struct)(RustealUStructHandle ustruct, uint8* data);
};

// ---------------------------------------------------------------------------
// Placeholder sub-tables (filled in later phases)
// ---------------------------------------------------------------------------

struct FRustealContainerApi
{
    // -- TArray --
    int32 (*array_len)(RustealUObjectHandle obj, RustealFPropertyHandle prop);
    ERustealErrorCode (*array_get)(RustealUObjectHandle obj, RustealFPropertyHandle prop,
        int32 index, uint8* out_buf, uint32 buf_size, uint32* out_written);
    ERustealErrorCode (*array_set)(RustealUObjectHandle obj, RustealFPropertyHandle prop,
        int32 index, const uint8* in_buf, uint32 buf_size);
    ERustealErrorCode (*array_add)(RustealUObjectHandle obj, RustealFPropertyHandle prop,
        const uint8* in_buf, uint32 buf_size);
    ERustealErrorCode (*array_remove)(RustealUObjectHandle obj, RustealFPropertyHandle prop, int32 index);
    ERustealErrorCode (*array_clear)(RustealUObjectHandle obj, RustealFPropertyHandle prop);
    uint32 (*array_element_size)(RustealFPropertyHandle prop);

    // -- TMap --
    int32 (*map_len)(RustealUObjectHandle obj, RustealFPropertyHandle prop);
    ERustealErrorCode (*map_find)(RustealUObjectHandle obj, RustealFPropertyHandle prop,
        const uint8* key_buf, uint32 key_size,
        uint8* out_val_buf, uint32 val_size, uint32* out_written);
    ERustealErrorCode (*map_add)(RustealUObjectHandle obj, RustealFPropertyHandle prop,
        const uint8* key_buf, uint32 key_size, const uint8* val_buf, uint32 val_size);
    ERustealErrorCode (*map_remove)(RustealUObjectHandle obj, RustealFPropertyHandle prop,
        const uint8* key_buf, uint32 key_size);
    ERustealErrorCode (*map_clear)(RustealUObjectHandle obj, RustealFPropertyHandle prop);
    ERustealErrorCode (*map_get_pair)(RustealUObjectHandle obj, RustealFPropertyHandle prop,
        int32 logical_index,
        uint8* out_key_buf, uint32 key_buf_size, uint32* out_key_written,
        uint8* out_val_buf, uint32 val_buf_size, uint32* out_val_written);

    // -- TSet --
    int32 (*set_len)(RustealUObjectHandle obj, RustealFPropertyHandle prop);
    bool  (*set_contains)(RustealUObjectHandle obj, RustealFPropertyHandle prop,
        const uint8* elem_buf, uint32 elem_size);
    ERustealErrorCode (*set_add)(RustealUObjectHandle obj, RustealFPropertyHandle prop,
        const uint8* elem_buf, uint32 elem_size);
    ERustealErrorCode (*set_remove)(RustealUObjectHandle obj, RustealFPropertyHandle prop,
        const uint8* elem_buf, uint32 elem_size);
    ERustealErrorCode (*set_clear)(RustealUObjectHandle obj, RustealFPropertyHandle prop);
    ERustealErrorCode (*set_get_element)(RustealUObjectHandle obj, RustealFPropertyHandle prop,
        int32 logical_index, uint8* out_buf, uint32 buf_size, uint32* out_written);

    // -- Temp container allocation (for function params) --
    void* (*alloc_temp)(RustealFPropertyHandle prop);
    void  (*free_temp)(RustealFPropertyHandle prop, void* base);

    // -- Bulk copy/set (single FFI call for entire container) --
    // Format: [u32 written_1][data_1][u32 written_2][data_2]...
    ERustealErrorCode (*array_copy_all)(RustealUObjectHandle obj, RustealFPropertyHandle prop,
        uint8* out_buf, uint32 buf_size, uint32* out_total_written, int32* out_count);
    ERustealErrorCode (*array_set_all)(RustealUObjectHandle obj, RustealFPropertyHandle prop,
        const uint8* in_buf, uint32 buf_size, int32 count);
    ERustealErrorCode (*map_copy_all)(RustealUObjectHandle obj, RustealFPropertyHandle prop,
        uint8* out_buf, uint32 buf_size, uint32* out_total_written, int32* out_count);
    ERustealErrorCode (*set_copy_all)(RustealUObjectHandle obj, RustealFPropertyHandle prop,
        uint8* out_buf, uint32 buf_size, uint32* out_total_written, int32* out_count);
};
struct FRustealDelegateApi
{
    ERustealErrorCode (*bind_delegate)(RustealUObjectHandle obj, RustealFPropertyHandle prop, uint64 callback_id);
    ERustealErrorCode (*unbind_delegate)(RustealUObjectHandle obj, RustealFPropertyHandle prop);
    ERustealErrorCode (*add_multicast)(RustealUObjectHandle obj, RustealFPropertyHandle prop, uint64 callback_id);
    ERustealErrorCode (*remove_multicast)(RustealUObjectHandle obj, RustealFPropertyHandle prop, uint64 callback_id);
    ERustealErrorCode (*broadcast_multicast)(RustealUObjectHandle obj, RustealFPropertyHandle prop, uint8* params);

    // Read a typed parameter from a raw ProcessEvent params buffer.
    // String/Text: writes [u32 utf8_len][utf8 bytes]. FName: writes packed uint64.
    // Struct: CopyScriptStruct. Primitives/enum/object: raw memcpy.
    ERustealErrorCode (*read_param)(
        RustealFPropertyHandle prop,
        void* params_buf,
        uint32 offset,
        uint8* out_buf,
        uint32 out_buf_size,
        uint32* out_written);
};
// ---------------------------------------------------------------------------
// Reify API types
// ---------------------------------------------------------------------------

enum class ERustealReifyPropType : uint32
{
    Bool = 0, Int8 = 1, Int16 = 2, Int32 = 3, Int64 = 4,
    UInt8 = 5, UInt16 = 6, UInt32 = 7, UInt64 = 8,
    Float = 9, Double = 10,
    String = 11, Name = 12, Text = 13,
    Object = 14, Class = 15, Struct = 16, Enum = 17,
};

struct FRustealReifyPropExtra
{
    RustealUClassHandle class_handle;      // Object/Class property class
    RustealUClassHandle meta_class_handle; // Class property metaclass
    RustealUStructHandle struct_handle;    // Struct property struct
    RustealUClassHandle enum_handle;       // Enum type (UEnum* cast)
    uint32 enum_underlying;             // Enum backing type
};

// ---------------------------------------------------------------------------
// FRustealReifyApi — runtime class creation, property/function registration
// ---------------------------------------------------------------------------

struct FRustealReifyApi
{
    RustealUClassHandle (*create_class)(
        const uint8* name, uint32 name_len,
        RustealUClassHandle parent,
        uint64 rust_type_id);

    RustealFPropertyHandle (*add_property)(
        RustealUClassHandle cls,
        const uint8* name, uint32 name_len,
        uint32 prop_type, uint64 prop_flags,
        const FRustealReifyPropExtra* extra);

    RustealUFunctionHandle (*add_function)(
        RustealUClassHandle cls,
        const uint8* name, uint32 name_len,
        uint64 callback_id, uint32 func_flags);

    ERustealErrorCode (*add_function_param)(
        RustealUFunctionHandle func,
        const uint8* name, uint32 name_len,
        uint32 prop_type, uint64 param_flags,
        const FRustealReifyPropExtra* extra);

    ERustealErrorCode (*finalize_class)(RustealUClassHandle cls);

    RustealUObjectHandle (*get_cdo)(RustealUClassHandle cls);

    ERustealErrorCode (*add_default_subobject)(
        RustealUClassHandle cls,
        const uint8* name, uint32 name_len,
        RustealUClassHandle component_class,
        uint32 flags,
        const uint8* attach_parent, uint32 attach_len);

    RustealUObjectHandle (*find_default_subobject)(
        RustealUObjectHandle owner,
        const uint8* name, uint32 name_len);
};
struct FRustealWidgetApi
{
    // Create a UMG widget. owning_object should be a PlayerController, World, or GameInstance.
    RustealUObjectHandle (*create_widget)(RustealUObjectHandle owning_object, RustealUClassHandle widget_class);

    // Set the root widget of a UUserWidget's WidgetTree.
    ERustealErrorCode (*set_root_widget)(RustealUObjectHandle user_widget, RustealUObjectHandle root_widget);

    // Get the WidgetTree UObject from a UUserWidget.
    RustealUObjectHandle (*get_widget_tree)(RustealUObjectHandle user_widget);
};

struct FRustealWorldApi
{
    RustealUObjectHandle (*spawn_actor)(RustealUObjectHandle world, RustealUClassHandle cls,
        const uint8* transform_buf, uint32 transform_size, RustealUObjectHandle owner);
    ERustealErrorCode (*get_all_actors_of_class)(RustealUObjectHandle world, RustealUClassHandle cls,
        uint8* out_buf, uint32 buf_byte_size, uint32* out_count);
    RustealUObjectHandle (*find_object)(RustealUClassHandle cls, const uint8* path_utf8, uint32 path_len);
    RustealUObjectHandle (*load_object)(RustealUClassHandle cls, const uint8* path_utf8, uint32 path_len);
    RustealUObjectHandle (*get_world)(RustealUObjectHandle actor);

    // Create a new UObject. outer can be null (falls back to transient package).
    RustealUObjectHandle (*new_object)(RustealUObjectHandle outer, RustealUClassHandle cls);

    // Spawn an actor with deferred construction (BeginPlay not yet called).
    // collision_method maps to ESpawnActorCollisionHandlingMethod.
    RustealUObjectHandle (*spawn_actor_deferred)(RustealUObjectHandle world, RustealUClassHandle cls,
        const uint8* transform_buf, uint32 transform_size,
        RustealUObjectHandle owner, RustealUObjectHandle instigator, uint8 collision_method);

    // Finish spawning a deferred actor (triggers BeginPlay).
    ERustealErrorCode (*finish_spawning)(RustealUObjectHandle actor,
        const uint8* transform_buf, uint32 transform_size);
};

// ---------------------------------------------------------------------------
// Main API table
// ---------------------------------------------------------------------------

struct FRustealApiTable
{
    uint32 version;

    // Fixed sub-tables
    const FRustealCoreApi*         core;
    const FRustealPropertyApi*     property;
    const FRustealReflectionApi*   reflection;
    const FRustealContainerApi*    container;
    const FRustealDelegateApi*     delegate;
    const FRustealLifecycleApi*    lifecycle;
    const FRustealReifyApi*        reify;
    const FRustealWorldApi*        world;
    const FRustealLoggingApi*      logging;
    const FRustealWidgetApi*       widget;

    // Generated function-pointer array
    const void* const*          func_table;
    uint32                      func_count;
};

// ---------------------------------------------------------------------------
// Rust callback table (returned by rusteal_init)
// ---------------------------------------------------------------------------

struct FRustealRustCallbacks
{
    void (*drop_rust_instance)(RustealUObjectHandle handle, uint64 type_id, uint8* rust_data);
    void (*invoke_rust_function)(uint64 callback_id, RustealUObjectHandle obj, uint8* params);
    void (*invoke_delegate_callback)(uint64 callback_id, uint8* params);
    void (*on_shutdown)();
    void (*construct_rust_instance)(RustealUObjectHandle obj, uint64 type_id, bool is_cdo);
    void (*notify_pinned_destroyed)(RustealUObjectHandle handle);
};

// ---------------------------------------------------------------------------
// DLL function signatures
// ---------------------------------------------------------------------------

using FRustealInitFn     = const FRustealRustCallbacks* (*)(const FRustealApiTable* api_table);
using FRustealShutdownFn = void (*)();
