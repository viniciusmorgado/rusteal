#pragma once

#include "CoreTypes.h"

struct RustealUObjectHandle {
  void *ptr;
};
struct RustealUClassHandle {
  void *ptr;
};
struct RustealFPropertyHandle {
  void *ptr;
};
struct RustealUFunctionHandle {
  void *ptr;
};
struct RustealUStructHandle {
  void *ptr;
};
struct RustealFNameHandle {
  uint64 value;
};
struct RustealFWeakObjectHandle {
  int32 object_index;
  int32 object_serial_number;
};

enum class ERustealErrorCode : uint32 {
  Ok = 0,
  ObjectDestroyed = 1,
  InvalidCast = 2,
  PropertyNotFound = 3,
  FunctionNotFound = 4,
  TypeMismatch = 5,
  NullArgument = 6,
  IndexOutOfRange = 7,
  InvalidOperation = 8,
  InternalError = 9,
  BufferTooSmall = 10,
};

struct FRustealCoreApi {
  bool (*is_valid)(RustealUObjectHandle obj);
  ERustealErrorCode (*get_name)(RustealUObjectHandle obj, uint8 *buf,
                                uint32 buf_len, uint32 *out_len);
  RustealUClassHandle (*get_class)(RustealUObjectHandle obj);
  bool (*is_a)(RustealUObjectHandle obj, RustealUClassHandle target_class);
  RustealUObjectHandle (*get_outer)(RustealUObjectHandle obj);

  RustealFNameHandle (*make_fname)(const uint8 *name_utf8, uint32 name_len);
  ERustealErrorCode (*fname_to_string)(RustealFNameHandle handle, uint8 *buf,
                                       uint32 buf_len, uint32 *out_len);

  RustealFWeakObjectHandle (*make_weak)(RustealUObjectHandle obj);
  RustealUObjectHandle (*resolve_weak)(RustealFWeakObjectHandle weak);
  bool (*is_weak_valid)(RustealFWeakObjectHandle weak);

  bool (*take_error_message)(ERustealErrorCode code, uint8 *buf, uint32 buf_len,
                             uint32 *out_len);

  bool (*is_child_of)(RustealUClassHandle class_handle,
                      RustealUClassHandle parent);

  uint32 (*take_pending_utf8)(const uint8 *key, uint8 *buf, uint32 buf_len);

  void *(*text_from_string)(const uint8 *utf8, uint32 len);
  void *(*text_localized)(const uint8 *ns, uint32 ns_len, const uint8 *key,
                          uint32 key_len, const uint8 *source,
                          uint32 source_len);
  void *(*text_clone)(const void *text);
  void (*text_destroy)(void *text);
  void (*text_to_string)(const void *text, uint8 *buf, uint32 buf_len,
                         uint32 *out_len);
  bool (*text_is_empty)(const void *text);
  bool (*text_equal)(const void *a, const void *b);
};

struct FRustealErrorReport {
  uint8 level;
  bool script_error;
  const uint8 *summary;
  uint32 summary_len;
  const uint8 *details;
  uint32 details_len;
  RustealFWeakObjectHandle object;
  const uint8 *file;
  uint32 file_len;
  uint32 line;
  uint32 column;
};

struct FRustealLoggingApi {
  void (*log)(uint8 level, const uint8 *msg, uint32 msg_len);

  void (*report)(const FRustealErrorReport *report);
};

struct FRustealLifecycleApi {
  void (*add_gc_root)(RustealUObjectHandle obj);
  void (*remove_gc_root)(RustealUObjectHandle obj);
  void (*register_pinned)(RustealUObjectHandle obj);
  void (*unregister_pinned)(RustealUObjectHandle obj);
};

struct FRustealPropertyApi {
  ERustealErrorCode (*get_bool)(RustealUObjectHandle obj,
                                RustealFPropertyHandle prop, bool *out);
  ERustealErrorCode (*set_bool)(RustealUObjectHandle obj,
                                RustealFPropertyHandle prop, bool val);
  ERustealErrorCode (*get_i32)(RustealUObjectHandle obj,
                               RustealFPropertyHandle prop, int32 *out);
  ERustealErrorCode (*set_i32)(RustealUObjectHandle obj,
                               RustealFPropertyHandle prop, int32 val);
  ERustealErrorCode (*get_i64)(RustealUObjectHandle obj,
                               RustealFPropertyHandle prop, int64 *out);
  ERustealErrorCode (*set_i64)(RustealUObjectHandle obj,
                               RustealFPropertyHandle prop, int64 val);
  ERustealErrorCode (*get_u8)(RustealUObjectHandle obj,
                              RustealFPropertyHandle prop, uint8 *out);
  ERustealErrorCode (*set_u8)(RustealUObjectHandle obj,
                              RustealFPropertyHandle prop, uint8 val);
  ERustealErrorCode (*get_f32)(RustealUObjectHandle obj,
                               RustealFPropertyHandle prop, float *out);
  ERustealErrorCode (*set_f32)(RustealUObjectHandle obj,
                               RustealFPropertyHandle prop, float val);
  ERustealErrorCode (*get_f64)(RustealUObjectHandle obj,
                               RustealFPropertyHandle prop, double *out);
  ERustealErrorCode (*set_f64)(RustealUObjectHandle obj,
                               RustealFPropertyHandle prop, double val);
  ERustealErrorCode (*get_string)(RustealUObjectHandle obj,
                                  RustealFPropertyHandle prop, uint8 *buf,
                                  uint32 buf_len, uint32 *out_len);
  ERustealErrorCode (*set_string)(RustealUObjectHandle obj,
                                  RustealFPropertyHandle prop, const uint8 *buf,
                                  uint32 len);
  ERustealErrorCode (*get_fname)(RustealUObjectHandle obj,
                                 RustealFPropertyHandle prop,
                                 RustealFNameHandle *out);
  ERustealErrorCode (*set_fname)(RustealUObjectHandle obj,
                                 RustealFPropertyHandle prop,
                                 RustealFNameHandle val);
  ERustealErrorCode (*get_object)(RustealUObjectHandle obj,
                                  RustealFPropertyHandle prop,
                                  RustealUObjectHandle *out);
  ERustealErrorCode (*set_object)(RustealUObjectHandle obj,
                                  RustealFPropertyHandle prop,
                                  RustealUObjectHandle val);
  ERustealErrorCode (*get_enum)(RustealUObjectHandle obj,
                                RustealFPropertyHandle prop, int64 *out);
  ERustealErrorCode (*set_enum)(RustealUObjectHandle obj,
                                RustealFPropertyHandle prop, int64 val);
  ERustealErrorCode (*get_struct)(RustealUObjectHandle obj,
                                  RustealFPropertyHandle prop, uint8 *out_buf,
                                  uint32 buf_size);
  ERustealErrorCode (*set_struct)(RustealUObjectHandle obj,
                                  RustealFPropertyHandle prop,
                                  const uint8 *in_buf, uint32 buf_size);

  ERustealErrorCode (*get_property_at)(RustealUObjectHandle obj,
                                       RustealFPropertyHandle prop,
                                       uint32 index, uint8 *out_buf,
                                       uint32 buf_size);
  ERustealErrorCode (*set_property_at)(RustealUObjectHandle obj,
                                       RustealFPropertyHandle prop,
                                       uint32 index, const uint8 *in_buf,
                                       uint32 buf_size);

  ERustealErrorCode (*get_soft_object_path)(RustealUObjectHandle obj,
                                            RustealFPropertyHandle prop,
                                            uint8 *buf, uint32 buf_len,
                                            uint32 *out_len);
  ERustealErrorCode (*set_soft_object_path)(RustealUObjectHandle obj,
                                            RustealFPropertyHandle prop,
                                            const uint8 *buf, uint32 len);
  ERustealErrorCode (*get_text)(RustealUObjectHandle obj,
                                RustealFPropertyHandle prop, void **out);
  ERustealErrorCode (*set_text)(RustealUObjectHandle obj,
                                RustealFPropertyHandle prop, const void *text);
  ERustealErrorCode (*get_optional)(RustealUObjectHandle obj,
                                    RustealFPropertyHandle prop, uint8 *buf,
                                    uint32 buf_size, bool *is_set);
  ERustealErrorCode (*set_optional)(RustealUObjectHandle obj,
                                    RustealFPropertyHandle prop,
                                    const uint8 *buf);
};

struct FRustealReflectionApi {
  RustealUClassHandle (*find_class)(const uint8 *name, uint32 name_len);
  RustealFPropertyHandle (*find_property)(RustealUClassHandle cls,
                                          const uint8 *name, uint32 name_len);
  RustealUClassHandle (*get_static_class)(const uint8 *name, uint32 name_len);
  uint32 (*get_property_size)(RustealFPropertyHandle prop);

  RustealUStructHandle (*find_struct)(const uint8 *name, uint32 name_len);
  RustealFPropertyHandle (*find_struct_property)(RustealUStructHandle ustruct,
                                                 const uint8 *name,
                                                 uint32 name_len);

  RustealUFunctionHandle (*find_function)(RustealUObjectHandle obj,
                                          const uint8 *name, uint32 name_len);
  uint8 *(*alloc_params)(RustealUFunctionHandle func);
  void (*free_params)(RustealUFunctionHandle func, uint8 *params);
  ERustealErrorCode (*call_function)(RustealUObjectHandle obj,
                                     RustealUFunctionHandle func,
                                     uint8 *params);
  RustealFPropertyHandle (*get_function_param)(RustealUFunctionHandle func,
                                               const uint8 *name,
                                               uint32 name_len);
  uint32 (*get_property_offset)(RustealFPropertyHandle prop);

  RustealUFunctionHandle (*find_function_by_class)(RustealUClassHandle cls,
                                                   const uint8 *name,
                                                   uint32 name_len);

  uint32 (*get_element_size)(RustealFPropertyHandle prop);

  uint32 (*get_struct_size)(RustealUStructHandle ustruct);

  ERustealErrorCode (*initialize_struct)(RustealUStructHandle ustruct,
                                         uint8 *data);

  ERustealErrorCode (*destroy_struct)(RustealUStructHandle ustruct,
                                      uint8 *data);

  ERustealErrorCode (*copy_struct)(RustealUStructHandle ustruct, uint8 *dest,
                                   const uint8 *src);

  RustealUClassHandle (*find_enum)(const uint8 *name, uint32 name_len);

  RustealUFunctionHandle (*get_delegate_signature)(RustealFPropertyHandle prop);

  RustealUStructHandle (*instanced_struct_type)(const uint8 *instanced);
  bool (*instanced_struct_get)(const uint8 *instanced,
                               RustealUStructHandle ustruct, uint8 *out);
  void (*instanced_struct_set)(uint8 *instanced, RustealUStructHandle ustruct,
                               const uint8 *value);
  void (*instanced_struct_reset)(uint8 *instanced);
};

struct FRustealContainerApi {
  int32 (*array_len)(RustealUObjectHandle obj, RustealFPropertyHandle prop);
  ERustealErrorCode (*array_get)(RustealUObjectHandle obj,
                                 RustealFPropertyHandle prop, int32 index,
                                 uint8 *out_buf, uint32 buf_size,
                                 uint32 *out_written);
  ERustealErrorCode (*array_set)(RustealUObjectHandle obj,
                                 RustealFPropertyHandle prop, int32 index,
                                 const uint8 *in_buf, uint32 buf_size);
  ERustealErrorCode (*array_add)(RustealUObjectHandle obj,
                                 RustealFPropertyHandle prop,
                                 const uint8 *in_buf, uint32 buf_size);
  ERustealErrorCode (*array_remove)(RustealUObjectHandle obj,
                                    RustealFPropertyHandle prop, int32 index);
  ERustealErrorCode (*array_clear)(RustealUObjectHandle obj,
                                   RustealFPropertyHandle prop);
  uint32 (*array_element_size)(RustealFPropertyHandle prop);

  int32 (*map_len)(RustealUObjectHandle obj, RustealFPropertyHandle prop);
  ERustealErrorCode (*map_find)(RustealUObjectHandle obj,
                                RustealFPropertyHandle prop,
                                const uint8 *key_buf, uint32 key_size,
                                uint8 *out_val_buf, uint32 val_size,
                                uint32 *out_written);
  ERustealErrorCode (*map_add)(RustealUObjectHandle obj,
                               RustealFPropertyHandle prop,
                               const uint8 *key_buf, uint32 key_size,
                               const uint8 *val_buf, uint32 val_size);
  ERustealErrorCode (*map_remove)(RustealUObjectHandle obj,
                                  RustealFPropertyHandle prop,
                                  const uint8 *key_buf, uint32 key_size);
  ERustealErrorCode (*map_clear)(RustealUObjectHandle obj,
                                 RustealFPropertyHandle prop);
  ERustealErrorCode (*map_get_pair)(RustealUObjectHandle obj,
                                    RustealFPropertyHandle prop,
                                    int32 logical_index, uint8 *out_key_buf,
                                    uint32 key_buf_size,
                                    uint32 *out_key_written, uint8 *out_val_buf,
                                    uint32 val_buf_size,
                                    uint32 *out_val_written);

  int32 (*set_len)(RustealUObjectHandle obj, RustealFPropertyHandle prop);
  bool (*set_contains)(RustealUObjectHandle obj, RustealFPropertyHandle prop,
                       const uint8 *elem_buf, uint32 elem_size);
  ERustealErrorCode (*set_add)(RustealUObjectHandle obj,
                               RustealFPropertyHandle prop,
                               const uint8 *elem_buf, uint32 elem_size);
  ERustealErrorCode (*set_remove)(RustealUObjectHandle obj,
                                  RustealFPropertyHandle prop,
                                  const uint8 *elem_buf, uint32 elem_size);
  ERustealErrorCode (*set_clear)(RustealUObjectHandle obj,
                                 RustealFPropertyHandle prop);
  ERustealErrorCode (*set_get_element)(RustealUObjectHandle obj,
                                       RustealFPropertyHandle prop,
                                       int32 logical_index, uint8 *out_buf,
                                       uint32 buf_size, uint32 *out_written);

  void *(*alloc_temp)(RustealFPropertyHandle prop);
  void (*free_temp)(RustealFPropertyHandle prop, void *base);

  ERustealErrorCode (*array_copy_all)(RustealUObjectHandle obj,
                                      RustealFPropertyHandle prop,
                                      uint8 *out_buf, uint32 buf_size,
                                      uint32 *out_total_written,
                                      int32 *out_count);
  ERustealErrorCode (*array_set_all)(RustealUObjectHandle obj,
                                     RustealFPropertyHandle prop,
                                     const uint8 *in_buf, uint32 buf_size,
                                     int32 count);
  ERustealErrorCode (*map_copy_all)(RustealUObjectHandle obj,
                                    RustealFPropertyHandle prop, uint8 *out_buf,
                                    uint32 buf_size, uint32 *out_total_written,
                                    int32 *out_count);
  ERustealErrorCode (*set_copy_all)(RustealUObjectHandle obj,
                                    RustealFPropertyHandle prop, uint8 *out_buf,
                                    uint32 buf_size, uint32 *out_total_written,
                                    int32 *out_count);
};
struct FRustealDelegateApi {
  ERustealErrorCode (*bind_delegate)(RustealUObjectHandle obj,
                                     RustealFPropertyHandle prop,
                                     uint64 callback_id);
  ERustealErrorCode (*unbind_delegate)(RustealUObjectHandle obj,
                                       RustealFPropertyHandle prop,
                                       uint64 callback_id);
  ERustealErrorCode (*add_multicast)(RustealUObjectHandle obj,
                                     RustealFPropertyHandle prop,
                                     uint64 callback_id);
  ERustealErrorCode (*remove_multicast)(RustealUObjectHandle obj,
                                        RustealFPropertyHandle prop,
                                        uint64 callback_id);
  ERustealErrorCode (*broadcast_multicast)(RustealUObjectHandle obj,
                                           RustealFPropertyHandle prop,
                                           uint8 *params);

  ERustealErrorCode (*read_param)(RustealFPropertyHandle prop, void *params_buf,
                                  uint32 offset, uint8 *out_buf,
                                  uint32 out_buf_size, uint32 *out_written);

  ERustealErrorCode (*add_function)(RustealUObjectHandle obj,
                                    RustealFPropertyHandle prop,
                                    RustealUObjectHandle target,
                                    const uint8 *name, uint32 name_len);

  void *(*make_delegate)(RustealUFunctionHandle signature,
                         RustealUObjectHandle owner, uint64 callback_id);
  void (*free_delegate)(void *delegate);

  ERustealErrorCode (*execute_delegate)(RustealUObjectHandle obj,
                                        RustealFPropertyHandle prop,
                                        uint8 *params);

  ERustealErrorCode (*latent_cancel)(uint64 callback_id);
  uint32 (*live_latent_proxies)();

  uint64 (*latent_function_begin)(RustealUObjectHandle owner,
                                  const uint8 *info,
                                  uint64 abort_callback_id);
  void (*latent_function_finish)(uint64 action, bool completed);
  uint32 (*live_latent_functions)();
};

enum class ERustealReifyDelegateKind : uint32 {
  Multicast = 0,
  Single = 1,
  Sparse = 2,
};

enum class ERustealReifyPropType : uint32 {
  Bool = 0,
  Int8 = 1,
  Int16 = 2,
  Int32 = 3,
  Int64 = 4,
  UInt8 = 5,
  UInt16 = 6,
  UInt32 = 7,
  UInt64 = 8,
  Float = 9,
  Double = 10,
  String = 11,
  Name = 12,
  Text = 13,
  Object = 14,
  Class = 15,
  Struct = 16,
  Enum = 17,
  Array = 18,
  SoftObject = 19,
  WeakObject = 20,
  SoftClass = 21,
  Interface = 22,
  Map = 23,
  Set = 24,
};

struct FRustealReifyPropExtra {
  RustealUClassHandle class_handle;
  RustealUClassHandle meta_class_handle;
  RustealUStructHandle struct_handle;
  RustealUClassHandle enum_handle;
  uint32 enum_underlying;
  uint32 inner_prop_type;
  uint32 value_prop_type;
  const FRustealReifyPropExtra *value;
};

struct FRustealReifyApi {
  RustealUClassHandle (*create_class)(const uint8 *name, uint32 name_len,
                                      RustealUClassHandle parent,
                                      uint64 rust_type_id, uint64 shape);

  RustealFPropertyHandle (*add_property)(RustealUClassHandle cls,
                                         const uint8 *name, uint32 name_len,
                                         uint32 prop_type, uint64 prop_flags,
                                         const FRustealReifyPropExtra *extra);

  RustealUFunctionHandle (*add_function)(RustealUClassHandle cls,
                                         const uint8 *name, uint32 name_len,
                                         uint64 callback_id, uint32 func_flags);

  ERustealErrorCode (*add_function_param)(RustealUFunctionHandle func,
                                          const uint8 *name, uint32 name_len,
                                          uint32 prop_type, uint64 param_flags,
                                          const FRustealReifyPropExtra *extra);

  ERustealErrorCode (*finalize_class)(RustealUClassHandle cls);

  RustealUObjectHandle (*get_cdo)(RustealUClassHandle cls);

  ERustealErrorCode (*add_default_subobject)(
      RustealUClassHandle cls, const uint8 *name, uint32 name_len,
      const uint8 *property, uint32 property_len,
      RustealUClassHandle component_class, uint32 flags,
      const uint8 *attach_parent, uint32 attach_len, const uint8 *attach_socket,
      uint32 socket_len);

  RustealUObjectHandle (*find_default_subobject)(RustealUObjectHandle owner,
                                                 const uint8 *name,
                                                 uint32 name_len);

  ERustealErrorCode (*set_property_metadata)(RustealFPropertyHandle prop,
                                             const uint8 *key, uint32 key_len,
                                             const uint8 *value,
                                             uint32 value_len);

  RustealUStructHandle (*create_struct)(const uint8 *name, uint32 name_len,
                                        uint64 shape);

  ERustealErrorCode (*finalize_struct)(RustealUStructHandle strukt);

  RustealUFunctionHandle (*add_delegate)(RustealUClassHandle cls,
                                         const uint8 *name, uint32 name_len,
                                         uint64 prop_flags, uint32 kind);

  ERustealErrorCode (*add_interface)(RustealUClassHandle cls, const uint8 *path,
                                     uint32 path_len);

  ERustealErrorCode (*set_class_config)(RustealUClassHandle cls,
                                        const uint8 *config_name,
                                        uint32 config_name_len);

  RustealUClassHandle (*create_enum)(const uint8 *name, uint32 name_len,
                                     const uint8 *entries, uint32 entries_len,
                                     const int64 *values, uint32 count);

  RustealUClassHandle (*create_interface)(const uint8 *name, uint32 name_len,
                                          uint64 shape);

  ERustealErrorCode (*finalize_interface)(RustealUClassHandle interface);

  ERustealErrorCode (*add_interface_class)(RustealUClassHandle cls,
                                           RustealUClassHandle interface);

  ERustealErrorCode (*set_function_metadata)(RustealUFunctionHandle func,
                                             const uint8 *key, uint32 key_len,
                                             const uint8 *value,
                                             uint32 value_len);
};
struct FRustealWidgetApi {
  RustealUObjectHandle (*create_widget)(RustealUObjectHandle owning_object,
                                        RustealUClassHandle widget_class);

  ERustealErrorCode (*set_root_widget)(RustealUObjectHandle user_widget,
                                       RustealUObjectHandle root_widget);

  RustealUObjectHandle (*get_widget_tree)(RustealUObjectHandle user_widget);
};

struct FRustealInputApi {
  ERustealErrorCode (*bind_action)(RustealUObjectHandle actor,
                                   RustealUObjectHandle action,
                                   uint8 trigger_event,
                                   const uint8 *function_name,
                                   uint32 function_name_len);

  bool (*should_display_touch_interface)();
};

struct FRustealConsoleArgs {
  const uint8 *args;
  uint32 args_len;
  RustealUObjectHandle world;
};

struct FRustealConsoleApi {
  ERustealErrorCode (*register_command)(const uint8 *name, uint32 name_len,
                                        const uint8 *help, uint32 help_len,
                                        uint64 callback_id);

  ERustealErrorCode (*register_variable)(const uint8 *name, uint32 name_len,
                                         const uint8 *help, uint32 help_len,
                                         uint32 kind,
                                         const uint8 *default_value,
                                         uint32 default_len);

  ERustealErrorCode (*unregister)(const uint8 *name, uint32 name_len);

  ERustealErrorCode (*get_variable)(const uint8 *name, uint32 name_len,
                                    uint8 *buf, uint32 buf_len,
                                    uint32 *out_len);

  ERustealErrorCode (*set_variable)(const uint8 *name, uint32 name_len,
                                    const uint8 *value, uint32 value_len);
};

struct FRustealWorldApi {
  RustealUObjectHandle (*spawn_actor)(RustealUObjectHandle world,
                                      RustealUClassHandle cls,
                                      const uint8 *transform_buf,
                                      uint32 transform_size,
                                      RustealUObjectHandle owner);
  ERustealErrorCode (*get_all_actors_of_class)(RustealUObjectHandle world,
                                               RustealUClassHandle cls,
                                               uint8 *out_buf,
                                               uint32 buf_byte_size,
                                               uint32 *out_count);
  RustealUObjectHandle (*find_object)(RustealUClassHandle cls,
                                      const uint8 *path_utf8, uint32 path_len);
  RustealUObjectHandle (*load_object)(RustealUClassHandle cls,
                                      const uint8 *path_utf8, uint32 path_len);
  RustealUObjectHandle (*get_world)(RustealUObjectHandle object);

  RustealUObjectHandle (*new_object)(RustealUObjectHandle outer,
                                     RustealUClassHandle cls);

  RustealUObjectHandle (*spawn_actor_deferred)(RustealUObjectHandle world,
                                               RustealUClassHandle cls,
                                               const uint8 *transform_buf,
                                               uint32 transform_size,
                                               RustealUObjectHandle owner,
                                               RustealUObjectHandle instigator,
                                               uint8 collision_method);

  ERustealErrorCode (*finish_spawning)(RustealUObjectHandle actor,
                                       const uint8 *transform_buf,
                                       uint32 transform_size);

  uint8 (*channel_to_object_type)(uint8 channel);

  uint8 *(*find_data_table_row)(RustealUObjectHandle table,
                                RustealFNameHandle row_name,
                                RustealUStructHandle row_struct);

  ERustealErrorCode (*set_timer)(RustealUObjectHandle context, float rate,
                                 bool looping, bool max_once_per_frame,
                                 float first_delay, uint64 callback_id,
                                 uint64 *out_handle);
  ERustealErrorCode (*set_timer_for_next_tick)(RustealUObjectHandle context,
                                               uint64 callback_id,
                                               uint64 *out_handle);
  ERustealErrorCode (*clear_timer)(RustealUObjectHandle context,
                                   uint64 handle);
  ERustealErrorCode (*pause_timer)(RustealUObjectHandle context, uint64 handle,
                                   bool paused);
  float (*timer_remaining)(RustealUObjectHandle context, uint64 handle);
  uint32 (*live_timer_callbacks)();

  ERustealErrorCode (*load_async)(const uint8 *paths, uint32 paths_len,
                                  uint64 callback_id);
  ERustealErrorCode (*load_primary_assets)(const uint8 *ids, uint32 ids_len,
                                           const uint8 *bundles,
                                           uint32 bundles_len,
                                           uint64 callback_id);
  ERustealErrorCode (*load_finish)(uint64 callback_id,
                                   RustealUObjectHandle *out_objects,
                                   uint32 capacity, uint32 *out_count);
  ERustealErrorCode (*load_cancel)(uint64 callback_id);
  uint32 (*loads_in_progress)();
};

struct FRustealPlatformApi {
  bool (*launch_task)(const void *run, uint8 *data);
  double (*seconds)();
  void (*sleep)(float seconds);
  bool (*is_game_thread)();
  uint8 *(*create_thread)(const void *run, uint8 *data, const uint8 *name,
                          uint32 name_len);
  void (*join_thread)(uint8 *thread);
  uint32 (*worker_count)();
};

struct FRustealApiTable {
  uint32 version;

  const FRustealCoreApi *core;
  const FRustealPropertyApi *property;
  const FRustealReflectionApi *reflection;
  const FRustealContainerApi *container;
  const FRustealDelegateApi *delegate;
  const FRustealLifecycleApi *lifecycle;
  const FRustealReifyApi *reify;
  const FRustealWorldApi *world;
  const FRustealLoggingApi *logging;
  const FRustealWidgetApi *widget;
  const FRustealInputApi *input;
  const FRustealConsoleApi *console;

  const void *const *func_table;
  uint32 func_count;
  uint64 func_fingerprint;

  const FRustealPlatformApi *platform;
};

struct FRustealRustCallbacks {
  void (*drop_rust_instance)(RustealUObjectHandle handle, uint64 type_id,
                             uint8 *rust_data);
  void (*invoke_rust_function)(uint64 callback_id, RustealUObjectHandle obj,
                               uint8 *params);
  void (*invoke_delegate_callback)(uint64 callback_id, uint8 *params);
  void (*on_shutdown)();
  void (*construct_rust_instance)(RustealUObjectHandle obj, uint64 type_id,
                                  bool is_cdo);
  void (*notify_pinned_destroyed)(RustealUObjectHandle handle);
  void (*on_tick)(float delta_seconds);
  void (*release_delegate_callback)(uint64 callback_id);
};

struct FRustealModuleExports {
  uint32 version;
  const FRustealRustCallbacks *(*init)(const FRustealApiTable *api_table);
  void (*shutdown)();
  const FRustealRustCallbacks *(*callbacks)();
};

struct FRustealStaticModule {
  const uint8 *name;
  uint32 name_len;
  const FRustealModuleExports *exports;
};

using FRustealInitFn =
    const FRustealRustCallbacks *(*)(const FRustealApiTable *api_table);
using FRustealShutdownFn = void (*)();
using FRustealVersionFn = uint32 (*)();
using FRustealCallbacksFn = const FRustealRustCallbacks *(*)();
