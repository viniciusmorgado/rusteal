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

const _: () = assert!(size_of::<RustealReifyPropExtra>() == 56);
const _: () = assert!(offset_of!(RustealReifyPropExtra, enum_underlying) == 32);
const _: () = assert!(offset_of!(RustealReifyPropExtra, inner_prop_type) == 36);
const _: () = assert!(offset_of!(RustealReifyPropExtra, value_prop_type) == 40);
const _: () = assert!(offset_of!(RustealReifyPropExtra, value) == 48);

const _: () = assert!(size_of::<crate::api_table::RustealConsoleArgs>() == 24);
const _: () = assert!(offset_of!(crate::api_table::RustealConsoleArgs, args_len) == 8);
const _: () = assert!(offset_of!(crate::api_table::RustealConsoleArgs, world) == 16);

const _: () = assert!(size_of::<crate::api_table::RustealErrorReport>() == 72);
const _: () = assert!(offset_of!(crate::api_table::RustealErrorReport, object) == 36);
const _: () = assert!(offset_of!(crate::api_table::RustealErrorReport, file) == 48);
const _: () = assert!(offset_of!(crate::api_table::RustealErrorReport, column) == 64);

const _: () = assert!(size_of::<crate::api_table::RustealApiTable>() == 136);
const _: () = assert!(offset_of!(crate::api_table::RustealApiTable, platform) == 128);
const _: () = assert!(offset_of!(crate::api_table::RustealApiTable, func_table) == 104);
const _: () = assert!(offset_of!(crate::api_table::RustealApiTable, func_count) == 112);
const _: () = assert!(offset_of!(crate::api_table::RustealApiTable, func_fingerprint) == 120);

const _: () = assert!(size_of::<crate::callbacks::RustealModuleExports>() == 32);
const _: () = assert!(offset_of!(crate::callbacks::RustealModuleExports, init) == 8);
const _: () = assert!(size_of::<crate::callbacks::RustealStaticModule>() == 24);
const _: () = assert!(offset_of!(crate::callbacks::RustealStaticModule, exports) == 16);
