use crate::handles::*;

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RustealReifyPropType {
    Bool = 0,
    Int8 = 1,
    Int16 = 2,
    Int32 = 3,
    Int64 = 4,
    UInt8 = 5,
    UInt16 = 6,
    UInt32 = 7,
    UInt64 = 8,
    Float = 9,
    Double = 10,
    String = 11,
    Name = 12,
    Text = 13,
    Object = 14,
    Class = 15,
    Struct = 16,
    Enum = 17,
    Array = 18,
    SoftObject = 19,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct RustealReifyPropExtra {
    pub class_handle: UClassHandle,
    pub meta_class_handle: UClassHandle,
    pub struct_handle: UStructHandle,
    pub enum_handle: UClassHandle,
    pub enum_underlying: u32,
    pub inner_prop_type: u32,
}

impl Default for RustealReifyPropExtra {
    fn default() -> Self {
        Self {
            class_handle: UClassHandle::null(),
            meta_class_handle: UClassHandle::null(),
            struct_handle: UStructHandle::null(),
            enum_handle: UClassHandle::null(),
            enum_underlying: 0,
            inner_prop_type: 0,
        }
    }
}
