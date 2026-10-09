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

static_assert(sizeof(FRustealReifyPropExtra) == 40,
              "FRustealReifyPropExtra must be 40 bytes");
static_assert(offsetof(FRustealReifyPropExtra, enum_underlying) == 32,
              "FRustealReifyPropExtra::enum_underlying at offset 32");
static_assert(offsetof(FRustealReifyPropExtra, inner_prop_type) == 36,
              "FRustealReifyPropExtra::inner_prop_type at offset 36");

static_assert(sizeof(FRustealConsoleArgs) == 24,
              "FRustealConsoleArgs must be 24 bytes");
static_assert(offsetof(FRustealConsoleArgs, args_len) == 8,
              "FRustealConsoleArgs::args_len at offset 8");
static_assert(offsetof(FRustealConsoleArgs, world) == 16,
              "FRustealConsoleArgs::world at offset 16");
