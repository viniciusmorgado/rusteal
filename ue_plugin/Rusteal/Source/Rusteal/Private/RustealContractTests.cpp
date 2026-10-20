#include "RustealApiTable.h"

static_assert(sizeof(RustealUObjectHandle) == 8,
              "RustealUObjectHandle must be 8 bytes");
static_assert(sizeof(RustealUClassHandle) == 8,
              "RustealUClassHandle must be 8 bytes");
static_assert(sizeof(RustealFPropertyHandle) == 8,
              "RustealFPropertyHandle must be 8 bytes");
static_assert(sizeof(RustealUFunctionHandle) == 8,
              "RustealUFunctionHandle must be 8 bytes");
static_assert(sizeof(RustealUStructHandle) == 8,
              "RustealUStructHandle must be 8 bytes");
static_assert(sizeof(RustealFNameHandle) == 8,
              "RustealFNameHandle must be 8 bytes");
static_assert(sizeof(RustealFWeakObjectHandle) == 8,
              "RustealFWeakObjectHandle must be 8 bytes");

static_assert(sizeof(ERustealErrorCode) == 4,
              "ERustealErrorCode must be 4 bytes (uint32)");

static_assert(alignof(RustealUObjectHandle) == alignof(void *),
              "RustealUObjectHandle alignment");
static_assert(alignof(RustealUClassHandle) == alignof(void *),
              "RustealUClassHandle alignment");
static_assert(alignof(RustealFPropertyHandle) == alignof(void *),
              "RustealFPropertyHandle alignment");
static_assert(alignof(RustealUFunctionHandle) == alignof(void *),
              "RustealUFunctionHandle alignment");
static_assert(alignof(RustealUStructHandle) == alignof(void *),
              "RustealUStructHandle alignment");
static_assert(alignof(RustealFNameHandle) == alignof(uint64),
              "RustealFNameHandle alignment");

static_assert(offsetof(RustealFWeakObjectHandle, object_index) == 0,
              "FWeakObjectHandle::object_index at offset 0");
static_assert(offsetof(RustealFWeakObjectHandle, object_serial_number) == 4,
              "FWeakObjectHandle::object_serial_number at offset 4");

static_assert(sizeof(FRustealReifyPropExtra) == 56,
              "FRustealReifyPropExtra must be 56 bytes");
static_assert(offsetof(FRustealReifyPropExtra, enum_underlying) == 32,
              "FRustealReifyPropExtra::enum_underlying at offset 32");
static_assert(offsetof(FRustealReifyPropExtra, inner_prop_type) == 36,
              "FRustealReifyPropExtra::inner_prop_type at offset 36");
static_assert(offsetof(FRustealReifyPropExtra, value_prop_type) == 40,
              "FRustealReifyPropExtra::value_prop_type at offset 40");
static_assert(offsetof(FRustealReifyPropExtra, value) == 48,
              "FRustealReifyPropExtra::value at offset 48");

static_assert(sizeof(FRustealInputArgs) == 24,
              "FRustealInputArgs must be 24 bytes");
static_assert(offsetof(FRustealInputArgs, action) == 16,
              "FRustealInputArgs::action at offset 16");

static_assert(sizeof(FRustealNativeEventArgs) == 24,
              "FRustealNativeEventArgs must be 24 bytes");
static_assert(offsetof(FRustealNativeEventArgs, a) == 16,
              "FRustealNativeEventArgs::a at offset 16");

static_assert(sizeof(FRustealConsoleArgs) == 24,
              "FRustealConsoleArgs must be 24 bytes");
static_assert(offsetof(FRustealConsoleArgs, args_len) == 8,
              "FRustealConsoleArgs::args_len at offset 8");
static_assert(offsetof(FRustealConsoleArgs, world) == 16,
              "FRustealConsoleArgs::world at offset 16");

static_assert(sizeof(FRustealErrorReport) == 72,
              "FRustealErrorReport must be 72 bytes");
static_assert(offsetof(FRustealErrorReport, object) == 36,
              "FRustealErrorReport::object at offset 36");
static_assert(offsetof(FRustealErrorReport, file) == 48,
              "FRustealErrorReport::file at offset 48");
static_assert(offsetof(FRustealErrorReport, column) == 64,
              "FRustealErrorReport::column at offset 64");

static_assert(sizeof(FRustealApiTable) == 136,
              "FRustealApiTable must be 136 bytes");
static_assert(offsetof(FRustealApiTable, func_table) == 104,
              "FRustealApiTable::func_table at offset 104");
static_assert(offsetof(FRustealApiTable, func_count) == 112,
              "FRustealApiTable::func_count at offset 112");
static_assert(offsetof(FRustealApiTable, func_fingerprint) == 120,
              "FRustealApiTable::func_fingerprint at offset 120");

static_assert(sizeof(FRustealModuleExports) == 32,
              "FRustealModuleExports must be 32 bytes");
static_assert(offsetof(FRustealModuleExports, init) == 8,
              "FRustealModuleExports::init at offset 8");
static_assert(sizeof(FRustealStaticModule) == 24,
              "FRustealStaticModule must be 24 bytes");
static_assert(offsetof(FRustealStaticModule, exports) == 16,
              "FRustealStaticModule::exports at offset 16");
static_assert(offsetof(FRustealApiTable, platform) == 128,
              "FRustealApiTable::platform at offset 128");
