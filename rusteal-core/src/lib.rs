#![allow(clippy::missing_safety_doc)]

pub mod api;
pub mod error;
pub mod event;
pub mod events;
pub mod traits;

#[allow(unused_imports, unused_unsafe, unsafe_op_in_unsafe_fn, clippy::all)]
pub mod ffi_dispatch {
    include!(concat!(env!("OUT_DIR"), "/ffi_dispatch.rs"));
}
pub mod console;
pub mod containers;
pub mod delegate_registry;
pub mod dynamic_call;
pub mod dynamic_value;
mod executor;
pub mod ffi_guard;
pub mod fname;
pub mod http;
pub mod input;
pub mod latent;
pub mod loading;
pub mod logging;
pub mod method;
pub mod module;
pub mod object_ref;
pub mod pinned;
pub mod reflected;
pub mod reify_registry;
pub mod script_interface;
pub mod soft_class_ref;
pub mod soft_ref;
mod std_fallback;
#[doc(hidden)]
pub mod strings;
pub mod struct_ref;
pub mod subclass_of;
mod sync;
pub mod task;
pub mod text;
pub mod tick;
pub mod time;
pub mod timer;
pub mod ue_math;
pub mod weak_ptr;
pub mod widget;
pub mod world;

pub use api::{api, init_api};
pub use console::{CommandContext, ConsoleVariable};
pub use containers::{ContainerElement, OwnedStruct, UeArray, UeMap, UeSet, struct_size_by_name};
pub use delegate_registry::DelegateBinding;
pub use dynamic_call::{DynamicCall, DynamicCallResult};
pub use dynamic_value::DynamicValue;
pub use error::{
    ErrorContext, ErrorKind, RustealError, RustealResult, UeSubject, check_ffi, check_ffi_ctx,
    ffi_infallible, ffi_infallible_ctx,
};
pub use ffi_guard::ffi_boundary;
pub use logging::{LOG_DISPLAY, LOG_ERROR, LOG_WARNING};
pub use method::method;
pub use module::{FuncTable, Module, ModuleId, type_id};
pub use object_ref::{Checked, ObjectPointer, UObjectRef};
pub use pinned::Pinned;
pub use script_interface::ScriptInterface;
pub use soft_class_ref::SoftClassRef;
pub use soft_ref::SoftObjectRef;
#[doc(hidden)]
pub use strings::take_utf8;
pub use struct_ref::UStructRef;
pub use subclass_of::SubclassOf;
pub use text::Text;
pub use traits::{
    HasParent, Inherits, RustObject, UeClass, UeEnum, UeHandle, UeInterface, UeStruct, ValidHandle,
};

pub use fname::FName;
pub use struct_ref::{OutRef, out_ref_from_param, struct_ref_from_param};
pub use ue_math::{
    BoxSphereBounds, Color, LinearColor, Plane, Ray, Rotator, Sphere, Transform, UeBox, UeBox2d,
};
pub use weak_ptr::TWeakObjectPtr;

pub use rusteal_ffi::{
    FNameHandle, FPropertyHandle, FWeakObjectHandle, RustealErrorCode, UClassHandle,
    UFunctionHandle, UObjectHandle, UStructHandle,
};

pub(crate) use sync::{lock_or_recover, read_or_recover, write_or_recover};
