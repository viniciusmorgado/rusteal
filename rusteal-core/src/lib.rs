#![allow(clippy::missing_safety_doc)]

pub mod api;
pub mod error;
pub mod traits;

#[allow(unused_imports, unused_unsafe, unsafe_op_in_unsafe_fn, clippy::all)]
pub mod ffi_dispatch {
    include!(concat!(env!("OUT_DIR"), "/ffi_dispatch.rs"));
}
pub mod console;
pub mod containers;
pub mod delegate_registry;
pub mod dynamic_call;
pub mod ffi_guard;
pub mod fname;
pub mod input;
pub mod logging;
pub mod object_ref;
pub mod pinned;
pub mod reify_registry;
pub mod soft_ref;
pub mod struct_ref;
pub mod subclass_of;
pub mod task;
pub mod ue_math;
pub mod weak_ptr;
pub mod widget;
pub mod world;

pub use api::{api, init_api};
pub use console::ConsoleVariable;
pub use containers::{ContainerElement, OwnedStruct, UeArray, UeMap, UeSet};
pub use delegate_registry::DelegateBinding;
pub use dynamic_call::{DynamicCall, DynamicCallResult};
pub use error::{
    RustealError, RustealResult, check_ffi, check_ffi_ctx, ffi_infallible, ffi_infallible_ctx,
};
pub use ffi_guard::ffi_boundary;
pub use logging::{LOG_DISPLAY, LOG_ERROR, LOG_WARNING};
pub use object_ref::{Checked, ObjectPointer, UObjectRef};
pub use pinned::Pinned;
pub use soft_ref::SoftObjectRef;
pub use struct_ref::UStructRef;
pub use subclass_of::SubclassOf;
pub use traits::{HasParent, Inherits, UeClass, UeEnum, UeHandle, UeStruct, ValidHandle};

pub use fname::FName;
pub use struct_ref::{OutRef, out_ref_from_param, struct_ref_from_param};
pub use ue_math::{
    BoxSphereBounds, Color, LinearColor, Plane, Ray, Rotator, Sphere, Transform, UeBox, UeBox2d,
};
pub use weak_ptr::TWeakObjectPtr;

pub use rusteal_ffi::{
    FNameHandle, FPropertyHandle, FWeakObjectHandle, RustealErrorCode, UClassHandle, UObjectHandle,
    UStructHandle,
};

use std::sync::{Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};

pub(crate) fn lock_or_recover<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

pub(crate) fn read_or_recover<T>(m: &RwLock<T>) -> RwLockReadGuard<'_, T> {
    m.read().unwrap_or_else(|e| e.into_inner())
}

pub(crate) fn write_or_recover<T>(m: &RwLock<T>) -> RwLockWriteGuard<'_, T> {
    m.write().unwrap_or_else(|e| e.into_inner())
}
