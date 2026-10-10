use crate::context::CodegenContext;
use crate::schema::ParamInfo;
use crate::type_map::{ConversionKind, MappedType};

pub fn parse_default_literal(
    param: &ParamInfo,
    mapped: &MappedType,
    ctx: &CodegenContext,
) -> Option<String> {
    let default_str = param.default.as_deref()?;

    if mapped.rust_to_ffi == ConversionKind::StructOpaque {
        return None;
    }

    match param.prop_type.as_str() {
        "BoolProperty" => parse_bool_default(default_str),
        "FloatProperty" => parse_float_default(default_str, "f32"),
        "DoubleProperty" => parse_float_default(default_str, "f64"),
        "IntProperty" => parse_int_default(default_str, "i32"),
        "Int8Property" => parse_int_default(default_str, "i8"),
        "Int16Property" => parse_int_default(default_str, "i16"),
        "Int64Property" => parse_int_default(default_str, "i64"),
        "ByteProperty" => {
            if param.enum_name.is_some() {
                parse_enum_default(default_str, param, ctx)
            } else {
                parse_int_default(default_str, "u8")
            }
        }
        "UInt16Property" => parse_int_default(default_str, "u16"),
        "UInt32Property" => parse_int_default(default_str, "u32"),
        "UInt64Property" => parse_int_default(default_str, "u64"),
        "EnumProperty" => parse_enum_default(default_str, param, ctx),
        "ObjectProperty" | "ClassProperty" | "SoftObjectProperty" | "WeakObjectProperty"
        | "InterfaceProperty" => parse_object_default(default_str, mapped),
        "StrProperty" | "TextProperty" => parse_string_default(default_str),
        "NameProperty" => parse_fname_default(default_str),
        _ => None,
    }
}

fn parse_bool_default(s: &str) -> Option<String> {
    match s {
        "true" | "True" => Some("true".into()),
        "false" | "False" => Some("false".into()),
        _ => None,
    }
}

fn parse_float_default(s: &str, suffix: &str) -> Option<String> {
    let _: f64 = s.parse().ok()?;

    if s.contains('.') {
        Some(format!("{s}{suffix}"))
    } else {
        Some(format!("{s}.0{suffix}"))
    }
}

fn parse_int_default(s: &str, suffix: &str) -> Option<String> {
    let _: i128 = s.parse().ok()?;
    Some(format!("{s}{suffix}"))
}

fn parse_object_default(s: &str, mapped: &MappedType) -> Option<String> {
    if s == "None" {
        if mapped.rust_to_ffi == ConversionKind::ObjectRef {
            Some("unsafe { rusteal_core::ObjectPointer::from_object_handle(rusteal_core::UObjectHandle::null()) }".into())
        } else {
            Some("rusteal_core::UObjectHandle::null()".into())
        }
    } else {
        None
    }
}

fn parse_string_default(s: &str) -> Option<String> {
    let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
    Some(format!("\"{escaped}\""))
}

fn parse_fname_default(s: &str) -> Option<String> {
    if s.is_empty() || s == "None" {
        Some("rusteal_core::FNameHandle(0)".into())
    } else {
        None
    }
}

fn parse_enum_default(s: &str, param: &ParamInfo, ctx: &CodegenContext) -> Option<String> {
    let enum_name = param.enum_name.as_deref()?;
    let enum_info = ctx.enums.get(enum_name)?;

    let actual_repr = ctx.enum_actual_repr(enum_name).unwrap_or("u8");

    for (variant_name, value) in &enum_info.pairs {
        if variant_name == s || variant_name.ends_with(&format!("::{s}")) {
            return Some(format!(
                "{enum_name}::from_value({value} as {actual_repr}).expect(\"invalid enum default for {enum_name}\")"
            ));
        }
    }

    None
}
