// Compile-time contract tests: ensure handle sizes match C++ expectations.
// These const assertions fail at compile time if sizes drift.

use std::mem::size_of;

use crate::handles::*;
use crate::error::RustealErrorCode;

const _: () = assert!(size_of::<UObjectHandle>() == 8);
const _: () = assert!(size_of::<UClassHandle>() == 8);
const _: () = assert!(size_of::<FPropertyHandle>() == 8);
const _: () = assert!(size_of::<UFunctionHandle>() == 8);
const _: () = assert!(size_of::<UStructHandle>() == 8);
const _: () = assert!(size_of::<FNameHandle>() == 8);
const _: () = assert!(size_of::<FWeakObjectHandle>() == 8);
const _: () = assert!(size_of::<RustealErrorCode>() == 4);
