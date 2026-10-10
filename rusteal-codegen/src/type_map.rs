use crate::schema::ParamInfo;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamDirection {
    In,
    Out,
    InOut,
    Return,
}

#[derive(Debug, Clone)]
pub struct MappedType {
    pub rust_type: String,
    pub rust_ffi_type: String,
    pub cpp_type: String,
    pub property_getter: String,
    pub property_setter: String,
    pub rust_to_ffi: ConversionKind,
    pub ffi_to_rust: ConversionKind,
    pub supported: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConversionKind {
    Identity,
    IntCast,
    ObjectRef,
    StringUtf8,
    EnumCast,
    StructOpaque,
    FName,
    ContainerArray,
    ContainerMap,
    ContainerSet,
    Delegate,
    MulticastDelegate,
}

const SUPPORTED_TYPES: &[&str] = &[
    "BoolProperty",
    "Int8Property",
    "ByteProperty",
    "Int16Property",
    "UInt16Property",
    "IntProperty",
    "UInt32Property",
    "Int64Property",
    "UInt64Property",
    "FloatProperty",
    "DoubleProperty",
    "StrProperty",
    "NameProperty",
    "TextProperty",
    "EnumProperty",
    "ObjectProperty",
    "ClassProperty",
    "StructProperty",
    "ArrayProperty",
    "MapProperty",
    "SetProperty",
    "SoftObjectProperty",
    "WeakObjectProperty",
    "InterfaceProperty",
    "DelegateProperty",
    "MulticastInlineDelegateProperty",
    "MulticastSparseDelegateProperty",
];

pub fn is_supported_type(prop_type: &str) -> bool {
    SUPPORTED_TYPES.contains(&prop_type)
}

pub fn map_property_type(
    prop_type: &str,
    class_name: Option<&str>,
    struct_name: Option<&str>,
    enum_name: Option<&str>,
    enum_underlying_type: Option<&str>,
    meta_class_name: Option<&str>,
    interface_name: Option<&str>,
) -> MappedType {
    match prop_type {
        "BoolProperty" => MappedType {
            rust_type: "bool".into(),
            rust_ffi_type: "bool".into(),
            cpp_type: "bool".into(),
            property_getter: "get_bool".into(),
            property_setter: "set_bool".into(),
            rust_to_ffi: ConversionKind::Identity,
            ffi_to_rust: ConversionKind::Identity,
            supported: true,
        },
        "Int8Property" => int_type("i8", "int8"),
        "ByteProperty" => {
            if let Some(en) = enum_name {
                enum_type(en, enum_underlying_type.unwrap_or("uint8"))
            } else {
                int_type("u8", "uint8")
            }
        }
        "Int16Property" => int_type("i16", "int16"),
        "UInt16Property" => int_type("u16", "uint16"),
        "IntProperty" => int_type("i32", "int32"),
        "UInt32Property" => int_type("u32", "uint32"),
        "Int64Property" => int_type("i64", "int64"),
        "UInt64Property" => int_type("u64", "uint64"),
        "FloatProperty" => MappedType {
            rust_type: "f32".into(),
            rust_ffi_type: "f32".into(),
            cpp_type: "float".into(),
            property_getter: "get_f32".into(),
            property_setter: "set_f32".into(),
            rust_to_ffi: ConversionKind::Identity,
            ffi_to_rust: ConversionKind::Identity,
            supported: true,
        },
        "DoubleProperty" => MappedType {
            rust_type: "f64".into(),
            rust_ffi_type: "f64".into(),
            cpp_type: "double".into(),
            property_getter: "get_f64".into(),
            property_setter: "set_f64".into(),
            rust_to_ffi: ConversionKind::Identity,
            ffi_to_rust: ConversionKind::Identity,
            supported: true,
        },
        "StrProperty" => MappedType {
            rust_type: "String".into(),
            rust_ffi_type: "*const u8".into(),
            cpp_type: "FString".into(),
            property_getter: "get_string".into(),
            property_setter: "set_string".into(),
            rust_to_ffi: ConversionKind::StringUtf8,
            ffi_to_rust: ConversionKind::StringUtf8,
            supported: true,
        },
        "TextProperty" => MappedType {
            rust_type: "String".into(),
            rust_ffi_type: "*const u8".into(),
            cpp_type: "FText".into(),
            property_getter: "get_string".into(),
            property_setter: "set_string".into(),
            rust_to_ffi: ConversionKind::StringUtf8,
            ffi_to_rust: ConversionKind::StringUtf8,
            supported: true,
        },
        "NameProperty" => MappedType {
            rust_type: "rusteal_core::FNameHandle".into(),
            rust_ffi_type: "rusteal_core::FNameHandle".into(),
            cpp_type: "FName".into(),
            property_getter: "get_fname".into(),
            property_setter: "set_fname".into(),
            rust_to_ffi: ConversionKind::FName,
            ffi_to_rust: ConversionKind::FName,
            supported: true,
        },
        "EnumProperty" => {
            if let Some(en) = enum_name {
                enum_type(en, enum_underlying_type.unwrap_or("uint8"))
            } else {
                unsupported("EnumProperty without enum_name")
            }
        }
        "ObjectProperty" => {
            if let Some(cls) = class_name {
                MappedType {
                    rust_type: format!("rusteal_core::UObjectRef<{cls}>"),
                    rust_ffi_type: "rusteal_core::UObjectHandle".into(),
                    cpp_type: format!("{cls}*"),
                    property_getter: "get_object".into(),
                    property_setter: "set_object".into(),
                    rust_to_ffi: ConversionKind::ObjectRef,
                    ffi_to_rust: ConversionKind::ObjectRef,
                    supported: true,
                }
            } else {
                MappedType {
                    rust_type: "rusteal_core::UObjectHandle".into(),
                    rust_ffi_type: "rusteal_core::UObjectHandle".into(),
                    cpp_type: "UObject*".into(),
                    property_getter: "get_object".into(),
                    property_setter: "set_object".into(),
                    rust_to_ffi: ConversionKind::Identity,
                    ffi_to_rust: ConversionKind::Identity,
                    supported: true,
                }
            }
        }
        "SoftObjectProperty" | "WeakObjectProperty" => {
            if let Some(cls) = class_name {
                MappedType {
                    rust_type: format!("rusteal_core::UObjectRef<{cls}>"),
                    rust_ffi_type: "rusteal_core::UObjectHandle".into(),
                    cpp_type: format!("{cls}*"),
                    property_getter: "get_object".into(),
                    property_setter: "set_object".into(),
                    rust_to_ffi: ConversionKind::ObjectRef,
                    ffi_to_rust: ConversionKind::ObjectRef,
                    supported: true,
                }
            } else {
                MappedType {
                    rust_type: "rusteal_core::UObjectHandle".into(),
                    rust_ffi_type: "rusteal_core::UObjectHandle".into(),
                    cpp_type: "UObject*".into(),
                    property_getter: "get_object".into(),
                    property_setter: "set_object".into(),
                    rust_to_ffi: ConversionKind::Identity,
                    ffi_to_rust: ConversionKind::Identity,
                    supported: true,
                }
            }
        }
        "ClassProperty" => {
            let effective_class = meta_class_name.or(class_name);

            if let Some(cls) = effective_class {
                MappedType {
                    rust_type: format!("rusteal_core::SubclassOf<{cls}>"),
                    rust_ffi_type: "rusteal_core::UObjectHandle".into(),
                    cpp_type: format!("{cls}*"),
                    property_getter: "get_object".into(),
                    property_setter: "set_object".into(),
                    rust_to_ffi: ConversionKind::ObjectRef,
                    ffi_to_rust: ConversionKind::ObjectRef,
                    supported: true,
                }
            } else {
                MappedType {
                    rust_type: "rusteal_core::UObjectHandle".into(),
                    rust_ffi_type: "rusteal_core::UObjectHandle".into(),
                    cpp_type: "UObject*".into(),
                    property_getter: "get_object".into(),
                    property_setter: "set_object".into(),
                    rust_to_ffi: ConversionKind::Identity,
                    ffi_to_rust: ConversionKind::Identity,
                    supported: true,
                }
            }
        }
        "InterfaceProperty" => {
            if let Some(iface) = interface_name {
                MappedType {
                    rust_type: format!("rusteal_core::UObjectRef<{iface}>"),
                    rust_ffi_type: "rusteal_core::UObjectHandle".into(),
                    cpp_type: format!("{iface}*"),
                    property_getter: "get_object".into(),
                    property_setter: "set_object".into(),
                    rust_to_ffi: ConversionKind::ObjectRef,
                    ffi_to_rust: ConversionKind::ObjectRef,
                    supported: true,
                }
            } else {
                unsupported("InterfaceProperty without interface_name")
            }
        }
        "StructProperty" => {
            if let Some(sn) = struct_name {
                MappedType {
                    rust_type: format!("*const u8 /* {sn} */"),
                    rust_ffi_type: "*const u8".into(),
                    cpp_type: format!("F{sn}"),
                    property_getter: "get_struct".into(),
                    property_setter: "set_struct".into(),
                    rust_to_ffi: ConversionKind::StructOpaque,
                    ffi_to_rust: ConversionKind::StructOpaque,
                    supported: true,
                }
            } else {
                unsupported("StructProperty without struct_name")
            }
        }
        "ArrayProperty" => MappedType {
            rust_type: "rusteal_core::UeArray<_>".into(),
            rust_ffi_type: String::new(),
            cpp_type: String::new(),
            property_getter: String::new(),
            property_setter: String::new(),
            rust_to_ffi: ConversionKind::ContainerArray,
            ffi_to_rust: ConversionKind::ContainerArray,
            supported: true,
        },
        "MapProperty" => MappedType {
            rust_type: "rusteal_core::UeMap<_, _>".into(),
            rust_ffi_type: String::new(),
            cpp_type: String::new(),
            property_getter: String::new(),
            property_setter: String::new(),
            rust_to_ffi: ConversionKind::ContainerMap,
            ffi_to_rust: ConversionKind::ContainerMap,
            supported: true,
        },
        "SetProperty" => MappedType {
            rust_type: "rusteal_core::UeSet<_>".into(),
            rust_ffi_type: String::new(),
            cpp_type: String::new(),
            property_getter: String::new(),
            property_setter: String::new(),
            rust_to_ffi: ConversionKind::ContainerSet,
            ffi_to_rust: ConversionKind::ContainerSet,
            supported: true,
        },
        "DelegateProperty" => MappedType {
            rust_type: "/* delegate */".into(),
            rust_ffi_type: String::new(),
            cpp_type: String::new(),
            property_getter: String::new(),
            property_setter: String::new(),
            rust_to_ffi: ConversionKind::Delegate,
            ffi_to_rust: ConversionKind::Delegate,
            supported: true,
        },
        "MulticastInlineDelegateProperty" | "MulticastSparseDelegateProperty" => MappedType {
            rust_type: "/* multicast delegate */".into(),
            rust_ffi_type: String::new(),
            cpp_type: String::new(),
            property_getter: String::new(),
            property_setter: String::new(),
            rust_to_ffi: ConversionKind::MulticastDelegate,
            ffi_to_rust: ConversionKind::MulticastDelegate,
            supported: true,
        },
        _ => unsupported(prop_type),
    }
}

pub fn param_direction(param: &ParamInfo) -> ParamDirection {
    use crate::schema::*;

    if param.prop_flags & CPF_RETURN_PARM != 0 {
        return ParamDirection::Return;
    }

    let is_out = param.prop_flags & CPF_OUT_PARM != 0;
    let is_const = param.prop_flags & CPF_CONST_PARM != 0;
    let is_ref = param.prop_flags & CPF_REFERENCE_PARM != 0;

    if is_out && is_const {
        ParamDirection::In
    } else if is_out && is_ref {
        ParamDirection::InOut
    } else if is_out {
        ParamDirection::Out
    } else {
        ParamDirection::In
    }
}

fn int_type(rust: &str, _cpp: &str) -> MappedType {
    let (getter, setter, ffi_type) = match rust {
        "i8" => ("get_u8", "set_u8", "u8"),
        "u8" => ("get_u8", "set_u8", "u8"),
        "i16" => ("get_i32", "set_i32", "i32"),
        "u16" => ("get_i32", "set_i32", "i32"),
        "i32" => ("get_i32", "set_i32", "i32"),
        "u32" => ("get_i32", "set_i32", "i32"),
        "i64" => ("get_i64", "set_i64", "i64"),
        "u64" => ("get_i64", "set_i64", "i64"),
        _ => ("get_i32", "set_i32", "i32"),
    };

    let cpp = match rust {
        "i8" => "int8",
        "u8" => "uint8",
        "i16" => "int16",
        "u16" => "uint16",
        "i32" => "int32",
        "u32" => "uint32",
        "i64" => "int64",
        "u64" => "uint64",
        _ => "int32",
    };

    let needs_cast = rust != ffi_type;

    MappedType {
        rust_type: rust.into(),
        rust_ffi_type: ffi_type.into(),
        cpp_type: cpp.into(),
        property_getter: getter.into(),
        property_setter: setter.into(),
        rust_to_ffi: if needs_cast {
            ConversionKind::IntCast
        } else {
            ConversionKind::Identity
        },
        ffi_to_rust: if needs_cast {
            ConversionKind::IntCast
        } else {
            ConversionKind::Identity
        },
        supported: true,
    }
}

fn enum_type(enum_name: &str, underlying: &str) -> MappedType {
    let repr = match underlying {
        "uint8" => "u8",
        "int8" => "i8",
        "uint16" => "u16",
        "int16" => "i16",
        "uint32" => "u32",
        "int32" => "i32",
        "uint64" => "u64",
        "int64" => "i64",
        _ => "u8",
    };

    MappedType {
        rust_type: enum_name.to_string(),
        rust_ffi_type: repr.into(),
        cpp_type: enum_name.to_string(),
        property_getter: "get_enum".into(),
        property_setter: "set_enum".into(),
        rust_to_ffi: ConversionKind::EnumCast,
        ffi_to_rust: ConversionKind::EnumCast,
        supported: true,
    }
}

fn unsupported(reason: &str) -> MappedType {
    MappedType {
        rust_type: format!("/* unsupported: {reason} */"),
        rust_ffi_type: String::new(),
        cpp_type: String::new(),
        property_getter: String::new(),
        property_setter: String::new(),
        rust_to_ffi: ConversionKind::Identity,
        ffi_to_rust: ConversionKind::Identity,
        supported: false,
    }
}

use crate::context::CodegenContext;
use crate::schema::PropertyInfo;

pub fn container_element_rust_type(
    inner: &PropertyInfo,
    ctx: Option<&CodegenContext>,
) -> Option<String> {
    match inner.prop_type.as_str() {
        "BoolProperty" => Some("bool".into()),
        "Int8Property" => Some("i8".into()),
        "ByteProperty" => {
            if let Some(en) = &inner.enum_name {
                if let Some(ctx) = ctx
                    && !ctx.enums.contains_key(en.as_str())
                {
                    return None;
                }

                Some(en.clone())
            } else {
                Some("u8".into())
            }
        }
        "Int16Property" => Some("i16".into()),
        "UInt16Property" => Some("u16".into()),
        "IntProperty" => Some("i32".into()),
        "UInt32Property" => Some("u32".into()),
        "Int64Property" => Some("i64".into()),
        "UInt64Property" => Some("u64".into()),
        "FloatProperty" => Some("f32".into()),
        "DoubleProperty" => Some("f64".into()),
        "StrProperty" | "TextProperty" => Some("String".into()),
        "NameProperty" => Some("rusteal_core::FNameHandle".into()),
        "ObjectProperty" | "SoftObjectProperty" | "WeakObjectProperty" => {
            if let Some(cls) = &inner.class_name {
                if let Some(ctx) = ctx
                    && !ctx.classes.contains_key(cls.as_str())
                {
                    return None;
                }

                Some(format!("rusteal_core::UObjectRef<{cls}>"))
            } else {
                Some("rusteal_core::UObjectHandle".into())
            }
        }
        "ClassProperty" => {
            let effective_class = inner
                .meta_class_name
                .as_deref()
                .or(inner.class_name.as_deref());

            if let Some(cls) = effective_class {
                if let Some(ctx) = ctx
                    && !ctx.classes.contains_key(cls)
                {
                    return None;
                }

                Some(format!("rusteal_core::SubclassOf<{cls}>"))
            } else {
                Some("rusteal_core::UObjectHandle".into())
            }
        }
        "InterfaceProperty" => {
            if let Some(ref iface) = inner.interface_name {
                if let Some(ctx) = ctx
                    && !ctx.classes.contains_key(iface.as_str())
                {
                    return None;
                }

                Some(format!("rusteal_core::UObjectRef<{iface}>"))
            } else {
                None
            }
        }
        "EnumProperty" => {
            if let Some(en) = &inner.enum_name {
                if let Some(ctx) = ctx
                    && !ctx.enums.contains_key(en.as_str())
                {
                    return None;
                }

                Some(en.clone())
            } else {
                None
            }
        }
        "StructProperty" => {
            if let Some(sn) = &inner.struct_name {
                if let Some(ctx) = ctx {
                    if let Some(si) = ctx.structs.get(sn.as_str()) {
                        if si.has_static_struct {
                            Some(format!("rusteal_core::OwnedStruct<{}>", si.cpp_name))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    Some(format!("rusteal_core::OwnedStruct<F{sn}>"))
                }
            } else {
                None
            }
        }
        _ => None,
    }
}

pub fn resolve_container_rust_type(
    prop: &PropertyInfo,
    ctx: Option<&CodegenContext>,
) -> Option<String> {
    match prop.prop_type.as_str() {
        "ArrayProperty" => {
            let inner = prop.inner_prop.as_ref()?;
            let elem_type = container_element_rust_type(inner, ctx)?;

            Some(format!("rusteal_core::UeArray<{elem_type}>"))
        }
        "MapProperty" => {
            let key = prop.key_prop.as_ref()?;
            let val = prop.value_prop.as_ref()?;
            let key_type = container_element_rust_type(key, ctx)?;
            let val_type = container_element_rust_type(val, ctx)?;

            Some(format!("rusteal_core::UeMap<{key_type}, {val_type}>"))
        }
        "SetProperty" => {
            let elem = prop.element_prop.as_ref()?;
            let elem_type = container_element_rust_type(elem, ctx)?;

            Some(format!("rusteal_core::UeSet<{elem_type}>"))
        }
        _ => None,
    }
}

pub fn is_container_param(param: &ParamInfo) -> bool {
    matches!(
        param.prop_type.as_str(),
        "ArrayProperty" | "MapProperty" | "SetProperty"
    )
}

pub fn container_param_input_type(param: &ParamInfo, ctx: &CodegenContext) -> Option<String> {
    match param.prop_type.as_str() {
        "ArrayProperty" => {
            let inner = param.inner_prop.as_ref()?;
            let elem = container_element_rust_type(inner, Some(ctx))?;

            Some(format!("&[{elem}]"))
        }
        "SetProperty" => {
            let elem = param.element_prop.as_ref()?;
            let etype = container_element_rust_type(elem, Some(ctx))?;

            Some(format!("&[{etype}]"))
        }
        "MapProperty" => {
            let key = param.key_prop.as_ref()?;
            let val = param.value_prop.as_ref()?;
            let kt = container_element_rust_type(key, Some(ctx))?;
            let vt = container_element_rust_type(val, Some(ctx))?;

            Some(format!("&[({kt}, {vt})]"))
        }
        _ => None,
    }
}

pub fn container_param_output_type(param: &ParamInfo, ctx: &CodegenContext) -> Option<String> {
    match param.prop_type.as_str() {
        "ArrayProperty" => {
            let inner = param.inner_prop.as_ref()?;
            let elem = container_element_rust_type(inner, Some(ctx))?;

            Some(format!("Vec<{elem}>"))
        }
        "SetProperty" => {
            let elem = param.element_prop.as_ref()?;
            let etype = container_element_rust_type(elem, Some(ctx))?;

            Some(format!("Vec<{etype}>"))
        }
        "MapProperty" => {
            let key = param.key_prop.as_ref()?;
            let val = param.value_prop.as_ref()?;
            let kt = container_element_rust_type(key, Some(ctx))?;
            let vt = container_element_rust_type(val, Some(ctx))?;

            Some(format!("Vec<({kt}, {vt})>"))
        }
        _ => None,
    }
}

pub fn container_elem_type_str(param: &ParamInfo, ctx: &CodegenContext) -> Option<String> {
    match param.prop_type.as_str() {
        "ArrayProperty" => {
            let inner = param.inner_prop.as_ref()?;

            container_element_rust_type(inner, Some(ctx))
        }
        "SetProperty" => {
            let elem = param.element_prop.as_ref()?;

            container_element_rust_type(elem, Some(ctx))
        }
        "MapProperty" => {
            let key = param.key_prop.as_ref()?;
            let val = param.value_prop.as_ref()?;
            let kt = container_element_rust_type(key, Some(ctx))?;
            let vt = container_element_rust_type(val, Some(ctx))?;

            Some(format!("{kt}, {vt}"))
        }
        _ => None,
    }
}
