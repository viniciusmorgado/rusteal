// RustealContractTests.cpp — Compile-time FFI contract tests.
// These static_asserts ensure C++ handle types and API structs
// are layout-compatible with their Rust #[repr(C)] counterparts.

#include "RustealApiTable.h"

// ---------------------------------------------------------------------------
// Handle sizes (must match Rust side exactly)
// ---------------------------------------------------------------------------

static_assert(sizeof(RustealUObjectHandle)      == 8,  "RustealUObjectHandle must be 8 bytes");
static_assert(sizeof(RustealUClassHandle)       == 8,  "RustealUClassHandle must be 8 bytes");
static_assert(sizeof(RustealFPropertyHandle)    == 8,  "RustealFPropertyHandle must be 8 bytes");
static_assert(sizeof(RustealUFunctionHandle)    == 8,  "RustealUFunctionHandle must be 8 bytes");
static_assert(sizeof(RustealUStructHandle)      == 8,  "RustealUStructHandle must be 8 bytes");
static_assert(sizeof(RustealFNameHandle)        == 8,  "RustealFNameHandle must be 8 bytes");
static_assert(sizeof(RustealFWeakObjectHandle)  == 8,  "RustealFWeakObjectHandle must be 8 bytes");

// ---------------------------------------------------------------------------
// Error code size
// ---------------------------------------------------------------------------

static_assert(sizeof(ERustealErrorCode) == 4, "ERustealErrorCode must be 4 bytes (uint32)");

// ---------------------------------------------------------------------------
// Handle alignment
// ---------------------------------------------------------------------------

static_assert(alignof(RustealUObjectHandle)     == alignof(void*), "RustealUObjectHandle alignment");
static_assert(alignof(RustealUClassHandle)      == alignof(void*), "RustealUClassHandle alignment");
static_assert(alignof(RustealFPropertyHandle)   == alignof(void*), "RustealFPropertyHandle alignment");
static_assert(alignof(RustealUFunctionHandle)   == alignof(void*), "RustealUFunctionHandle alignment");
static_assert(alignof(RustealUStructHandle)     == alignof(void*), "RustealUStructHandle alignment");
static_assert(alignof(RustealFNameHandle)       == alignof(uint64), "RustealFNameHandle alignment");

// ---------------------------------------------------------------------------
// Weak object handle layout
// ---------------------------------------------------------------------------

static_assert(offsetof(RustealFWeakObjectHandle, object_index)         == 0, "FWeakObjectHandle::object_index at offset 0");
static_assert(offsetof(RustealFWeakObjectHandle, object_serial_number) == 4, "FWeakObjectHandle::object_serial_number at offset 4");
