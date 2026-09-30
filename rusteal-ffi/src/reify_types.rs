// Reify FFI types: property type enum, extra metadata struct, and flag constants.

use crate::handles::*;

/// Property type discriminator for reify API.
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
    /// `TArray` of the type in `RustealReifyPropExtra::inner_prop_type`, whose
    /// class/struct/enum fields describe the element.
    Array = 18,
}

/// Extra metadata for Object/Class/Struct/Enum/Array properties.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct RustealReifyPropExtra {
    /// Object/Class property class handle.
    pub class_handle: UClassHandle,
    /// Class property metaclass handle.
    pub meta_class_handle: UClassHandle,
    /// Struct property UScriptStruct handle.
    pub struct_handle: UStructHandle,
    /// Enum type handle (UEnum* cast to UClassHandle).
    pub enum_handle: UClassHandle,
    /// Enum backing type size.
    pub enum_underlying: u32,
    /// Array property element type (a `RustealReifyPropType`, not `Array`).
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
