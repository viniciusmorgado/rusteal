use std::mem::{offset_of, size_of};

use crate::error::RustealErrorCode;
use crate::handles::*;
use crate::reify_types::RustealReifyPropExtra;

const _: () = assert!(size_of::<UObjectHandle>() == 8);
const _: () = assert!(size_of::<UClassHandle>() == 8);
const _: () = assert!(size_of::<FPropertyHandle>() == 8);
const _: () = assert!(size_of::<UFunctionHandle>() == 8);
const _: () = assert!(size_of::<UStructHandle>() == 8);
const _: () = assert!(size_of::<FNameHandle>() == 8);
const _: () = assert!(size_of::<FWeakObjectHandle>() == 8);
const _: () = assert!(size_of::<RustealErrorCode>() == 4);

const _: () = assert!(size_of::<RustealReifyPropExtra>() == 40);
const _: () = assert!(offset_of!(RustealReifyPropExtra, enum_underlying) == 32);
const _: () = assert!(offset_of!(RustealReifyPropExtra, inner_prop_type) == 36);

const _: () = assert!(size_of::<crate::api_table::RustealConsoleArgs>() == 24);
const _: () = assert!(offset_of!(crate::api_table::RustealConsoleArgs, args_len) == 8);
const _: () = assert!(offset_of!(crate::api_table::RustealConsoleArgs, world) == 16);
