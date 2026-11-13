use rusteal_ffi::{UClassHandle, UObjectHandle, UStructHandle};

use crate::error::RustealResult;

pub trait UeClass: 'static {
    const RUST_TYPE_ID: u64 = 0;

    fn static_class() -> UClassHandle;
}

/// A class defined in Rust with `#[uclass]`: the object it is a handle to, and the way back
/// from that object to the Rust class.
pub trait RustObject: UeClass + Sized {
    fn object_handle(&self) -> UObjectHandle;

    fn from_object_handle(handle: UObjectHandle) -> RustealResult<Self>;
}

pub trait UeInterface: 'static {
    fn static_class() -> UClassHandle;
}

pub trait UeStruct: 'static {
    fn static_struct() -> UStructHandle;
}

pub trait UeEnum: Copy + 'static {
    type Repr: Copy;

    fn static_enum() -> UClassHandle;

    fn to_i64(self) -> i64;

    fn from_i64(value: i64) -> Option<Self>;
}

pub trait HasParent: UeClass {
    type Parent: UeClass;
}

pub trait Inherits<U: UeClass>: UeClass {}

pub trait UeHandle {
    fn checked_handle(&self) -> RustealResult<UObjectHandle>;

    fn raw_handle(&self) -> UObjectHandle;
}

pub trait ValidHandle {
    fn handle(&self) -> UObjectHandle;
}
