use rusteal_ffi::{UClassHandle, UObjectHandle, UStructHandle};

use crate::error::RustealResult;

pub trait UeClass: 'static {
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
