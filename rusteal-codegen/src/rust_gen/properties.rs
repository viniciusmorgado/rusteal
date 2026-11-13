use crate::context::CodegenContext;
use crate::naming::{strip_bool_prefix, to_snake_case};
use crate::schema::PropertyInfo;
use crate::type_map::{self, ConversionKind, MappedType};

pub struct PropertyContext {
    pub find_prop_fn: String,
    pub handle_expr: String,
    pub pre_access: String,
    pub container_expr: String,
}

pub fn rust_property_name(prop: &PropertyInfo) -> String {
    if prop.prop_type == "BoolProperty" {
        strip_bool_prefix(&prop.name)
    } else {
        to_snake_case(&prop.name)
    }
}

pub fn is_read_only_non_public(prop: &PropertyInfo) -> bool {
    use crate::schema::{
        CPF_BLUEPRINT_READ_ONLY, CPF_NATIVE_ACCESS_PRIVATE, CPF_NATIVE_ACCESS_PROTECTED,
    };

    prop.prop_flags & (CPF_NATIVE_ACCESS_PRIVATE | CPF_NATIVE_ACCESS_PROTECTED) != 0
        && prop.prop_flags & CPF_BLUEPRINT_READ_ONLY != 0
}

pub fn collect_deduped_properties<'a>(
    props: &'a [PropertyInfo],
    ctx: Option<&CodegenContext>,
) -> (std::collections::HashSet<String>, Vec<&'a PropertyInfo>) {
    let mut prop_names = std::collections::HashSet::new();
    let mut deduped = Vec::new();

    for prop in props {
        if prop.getter.is_some() {
            continue;
        }

        let mapped = type_map::map_property_type(
            &prop.prop_type,
            prop.class_name.as_deref(),
            prop.struct_name.as_deref(),
            prop.enum_name.as_deref(),
            prop.enum_underlying_type.as_deref(),
            prop.meta_class_name.as_deref(),
            prop.interface_name.as_deref(),
        );

        if !mapped.supported && prop.prop_type != "OptionalProperty" {
            continue;
        }

        if matches!(mapped.rust_to_ffi, ConversionKind::ObjectRef)
            && let Some(ctx) = ctx
        {
            let type_available = match prop.prop_type.as_str() {
                "ClassProperty" => {
                    let effective = prop
                        .meta_class_name
                        .as_deref()
                        .or(prop.class_name.as_deref());

                    effective.is_none_or(|c| ctx.classes.contains_key(c))
                }
                "InterfaceProperty" => prop
                    .interface_name
                    .as_deref()
                    .is_some_and(|i| ctx.classes.contains_key(i)),
                _ => prop
                    .class_name
                    .as_deref()
                    .is_none_or(|c| ctx.classes.contains_key(c)),
            };

            if !type_available {
                continue;
            }
        }

        if matches!(mapped.rust_to_ffi, ConversionKind::StructOpaque) {
            let valid = if let Some(ctx) = ctx {
                prop.struct_name
                    .as_deref()
                    .is_some_and(|sn| ctx.structs.contains_key(sn))
            } else {
                prop.struct_name.is_some()
            };

            if !valid {
                continue;
            }
        }

        if matches!(
            mapped.rust_to_ffi,
            ConversionKind::ContainerArray
                | ConversionKind::ContainerMap
                | ConversionKind::ContainerSet
        ) && type_map::resolve_container_rust_type(prop, ctx).is_none()
        {
            continue;
        }

        if let Some(ctx) = ctx {
            match mapped.rust_to_ffi {
                ConversionKind::EnumCast => {
                    if let Some(en) = &prop.enum_name
                        && !ctx.enums.contains_key(en.as_str())
                    {
                        continue;
                    }
                }
                ConversionKind::ObjectRef => {
                    if let Some(cn) = &prop.class_name
                        && !ctx.classes.contains_key(cn.as_str())
                    {
                        continue;
                    }
                }
                _ => {}
            }
        }

        let rust_name = if prop.prop_type == "BoolProperty" {
            strip_bool_prefix(&prop.name)
        } else {
            to_snake_case(&prop.name)
        };

        let is_container = matches!(
            mapped.rust_to_ffi,
            ConversionKind::ContainerArray
                | ConversionKind::ContainerMap
                | ConversionKind::ContainerSet
        );

        let is_delegate = matches!(
            mapped.rust_to_ffi,
            ConversionKind::Delegate | ConversionKind::MulticastDelegate
        );

        let getter_name = if is_container || is_delegate {
            rust_name.clone()
        } else {
            format!("get_{rust_name}")
        };

        if prop_names.contains(&getter_name) {
            continue;
        }

        prop_names.insert(getter_name);

        if !is_container && !is_delegate {
            prop_names.insert(format!("set_{rust_name}"));
        }

        deduped.push(prop);
    }

    (prop_names, deduped)
}

pub fn generate_property(
    out: &mut String,
    prop: &PropertyInfo,
    pctx: &PropertyContext,
    ctx: &CodegenContext,
    suppress_setters: &std::collections::HashSet<String>,
) {
    let mapped = type_map::map_property_type(
        &prop.prop_type,
        prop.class_name.as_deref(),
        prop.struct_name.as_deref(),
        prop.enum_name.as_deref(),
        prop.enum_underlying_type.as_deref(),
        prop.meta_class_name.as_deref(),
        prop.interface_name.as_deref(),
    );

    if prop.prop_type == "OptionalProperty" {
        generate_optional_property(out, prop, pctx, ctx, suppress_setters);
        return;
    }

    if !mapped.supported {
        return;
    }

    if matches!(
        mapped.rust_to_ffi,
        ConversionKind::Delegate | ConversionKind::MulticastDelegate
    ) {
        return;
    }

    let prop_name = &prop.name;

    let rust_name = if prop.prop_type == "BoolProperty" {
        strip_bool_prefix(prop_name)
    } else {
        to_snake_case(prop_name)
    };

    let prop_name_len = prop_name.len();
    let byte_lit = format!("b\"{prop_name}\\0\"");

    if prop.array_dim > 1 {
        generate_fixed_array_property(
            out,
            prop,
            &rust_name,
            &byte_lit,
            prop_name_len,
            pctx,
            ctx,
            &mapped,
        );

        return;
    }

    if matches!(
        mapped.rust_to_ffi,
        ConversionKind::ContainerArray
            | ConversionKind::ContainerMap
            | ConversionKind::ContainerSet
    ) {
        if let Some(container_type) = type_map::resolve_container_rust_type(prop, Some(ctx)) {
            generate_container_getter(
                out,
                &rust_name,
                &byte_lit,
                prop_name_len,
                pctx,
                &container_type,
            );
        }

        return;
    }

    match mapped.rust_to_ffi {
        ConversionKind::StringUtf8
            if matches!(
                prop.prop_type.as_str(),
                "SoftObjectProperty" | "SoftClassProperty"
            ) =>
        {
            generate_soft_getter(out, &rust_name, &byte_lit, prop_name_len, pctx, &mapped);
        }
        ConversionKind::StringUtf8 => {
            generate_string_getter(out, &rust_name, &byte_lit, prop_name_len, pctx);
        }
        ConversionKind::StructOpaque => {
            let struct_cpp = prop
                .struct_name
                .as_deref()
                .and_then(|sn| ctx.structs.get(sn))
                .map(|si| si.cpp_name.clone())
                .unwrap_or_else(|| {
                    format!("F{}", prop.struct_name.as_deref().unwrap_or("Unknown"))
                });

            generate_struct_getter(out, &rust_name, &byte_lit, prop_name_len, pctx, &struct_cpp);
            let setter_name = format!("set_{rust_name}");

            if !suppress_setters.contains(&setter_name) {
                generate_struct_setter(
                    out,
                    &rust_name,
                    &byte_lit,
                    prop_name_len,
                    pctx,
                    &struct_cpp,
                );
            }

            return;
        }
        ConversionKind::ObjectRef if prop.prop_type == "TextProperty" => {
            generate_text_getter(out, &rust_name, &byte_lit, prop_name_len, pctx);
        }
        ConversionKind::ObjectRef => {
            generate_object_getter(out, &rust_name, &byte_lit, prop_name_len, pctx, &mapped);
        }
        ConversionKind::EnumCast => {
            let actual_repr = prop
                .enum_name
                .as_deref()
                .and_then(|en| ctx.enum_actual_repr(en))
                .unwrap_or(&mapped.rust_ffi_type);

            generate_enum_getter(
                out,
                &rust_name,
                &byte_lit,
                prop_name_len,
                pctx,
                &mapped,
                actual_repr,
            );
        }
        ConversionKind::FName => {
            generate_fname_getter(out, &rust_name, &byte_lit, prop_name_len, pctx);
        }
        ConversionKind::IntCast => {
            generate_int_cast_getter(out, &rust_name, &byte_lit, prop_name_len, pctx, &mapped);
        }
        _ => {
            generate_primitive_getter(out, &rust_name, &byte_lit, prop_name_len, pctx, &mapped);
        }
    }

    let setter_name = format!("set_{rust_name}");

    if suppress_setters.contains(&setter_name) {
        return;
    }

    match mapped.rust_to_ffi {
        ConversionKind::StringUtf8
            if matches!(
                prop.prop_type.as_str(),
                "SoftObjectProperty" | "SoftClassProperty"
            ) =>
        {
            generate_soft_setter(out, &rust_name, &byte_lit, prop_name_len, pctx, &mapped);
        }
        ConversionKind::StringUtf8 => {
            generate_string_setter(out, &rust_name, &byte_lit, prop_name_len, pctx);
        }
        ConversionKind::StructOpaque => {}
        ConversionKind::ObjectRef if prop.prop_type == "TextProperty" => {
            generate_text_setter(out, &rust_name, &byte_lit, prop_name_len, pctx);
        }
        ConversionKind::ObjectRef => {
            generate_object_setter(out, &rust_name, &byte_lit, prop_name_len, pctx, &mapped);
        }
        ConversionKind::EnumCast => {
            generate_enum_setter(out, &rust_name, &byte_lit, prop_name_len, pctx, &mapped);
        }
        ConversionKind::FName => {
            generate_fname_setter(out, &rust_name, &byte_lit, prop_name_len, pctx);
        }
        ConversionKind::IntCast => {
            generate_int_cast_setter(out, &rust_name, &byte_lit, prop_name_len, pctx, &mapped);
        }
        _ => {
            generate_primitive_setter(out, &rust_name, &byte_lit, prop_name_len, pctx, &mapped);
        }
    }
}

pub fn default_value_for(rust_type: &str) -> &'static str {
    match rust_type {
        "bool" => "false",
        "i8" | "u8" | "i16" | "u16" | "i32" | "u32" | "i64" | "u64" => "0",
        "f32" => "0.0f32",
        "f64" => "0.0f64",
        _ => "Default::default()",
    }
}

fn emit_prop_lookup(
    out: &mut String,
    byte_lit: &str,
    prop_name_len: usize,
    pctx: &PropertyContext,
) {
    let find = &pctx.find_prop_fn;
    let handle = &pctx.handle_expr;

    out.push_str(&format!(
        "        static PROP: std::sync::OnceLock<rusteal_core::FPropertyHandle> = std::sync::OnceLock::new();\n\
         \x20       let prop = *PROP.get_or_init(|| unsafe {{\n\
         \x20           rusteal_core::ffi_dispatch::reflection_{find}(\n\
         \x20               {handle}, {byte_lit}.as_ptr(), {prop_name_len}\n\
         \x20           )\n\
         \x20       }});\n"
    ));
}

fn emit_pre_access(out: &mut String, pctx: &PropertyContext) {
    if !pctx.pre_access.is_empty() {
        out.push_str(&format!("        {}\n", pctx.pre_access));
    }
}

fn generate_primitive_getter(
    out: &mut String,
    rust_name: &str,
    byte_lit: &str,
    prop_name_len: usize,
    pctx: &PropertyContext,
    mapped: &MappedType,
) {
    let rust_type = &mapped.rust_type;
    let getter = &mapped.property_getter;
    let default = default_value_for(rust_type);
    let c = &pctx.container_expr;

    out.push_str(&format!(
        "    fn get_{rust_name}(&self) -> {rust_type} {{\n"
    ));

    emit_prop_lookup(out, byte_lit, prop_name_len, pctx);
    emit_pre_access(out, pctx);

    out.push_str(&format!(
        "        let mut out = {default};\n\
         \x20       rusteal_core::ffi_infallible_ctx(unsafe {{ rusteal_core::ffi_dispatch::property_{getter}({c}, prop, &mut out) }}, \"{rust_name}\");\n\
         \x20       out\n\
         \x20   }}\n\n"
    ));
}

fn generate_int_cast_getter(
    out: &mut String,
    rust_name: &str,
    byte_lit: &str,
    prop_name_len: usize,
    pctx: &PropertyContext,
    mapped: &MappedType,
) {
    let rust_type = &mapped.rust_type;
    let ffi_type = &mapped.rust_ffi_type;
    let getter = &mapped.property_getter;
    let default = default_value_for(ffi_type);
    let c = &pctx.container_expr;

    out.push_str(&format!(
        "    fn get_{rust_name}(&self) -> {rust_type} {{\n"
    ));

    emit_prop_lookup(out, byte_lit, prop_name_len, pctx);
    emit_pre_access(out, pctx);

    out.push_str(&format!(
        "        let mut out = {default};\n\
         \x20       rusteal_core::ffi_infallible_ctx(unsafe {{ rusteal_core::ffi_dispatch::property_{getter}({c}, prop, &mut out) }}, \"{rust_name}\");\n\
         \x20       out as {rust_type}\n\
         \x20   }}\n\n"
    ));
}

fn generate_string_getter(
    out: &mut String,
    rust_name: &str,
    byte_lit: &str,
    prop_name_len: usize,
    pctx: &PropertyContext,
) {
    let c = &pctx.container_expr;

    out.push_str(&format!("    fn get_{rust_name}(&self) -> String {{\n"));

    emit_prop_lookup(out, byte_lit, prop_name_len, pctx);
    emit_pre_access(out, pctx);

    out.push_str(&format!(
        "        let mut buf = vec![0u8; 512];\n\
         \x20       let mut out_len: u32 = 0;\n\
         \x20       rusteal_core::ffi_infallible_ctx(unsafe {{\n\
         \x20           rusteal_core::ffi_dispatch::property_get_string({c}, prop, buf.as_mut_ptr(), buf.len() as u32, &mut out_len)\n\
         \x20       }}, \"{rust_name}\");\n\
         \x20       rusteal_core::take_utf8(buf, out_len)\n\
         \x20   }}\n\n"
    ));
}

enum OptionalValue {
    Plain(&'static str),
    Bool,
    Enum {
        rust_type: String,
        repr: &'static str,
    },
    Struct {
        cpp_name: String,
        name: String,
    },
    Object(String),
}

fn optional_value(inner: &PropertyInfo, ctx: &CodegenContext) -> Option<OptionalValue> {
    let plain = |ty| Some(OptionalValue::Plain(ty));
    let enumeration = |name: &str| {
        let repr = ctx.enum_actual_repr(name)?;

        Some(OptionalValue::Enum {
            rust_type: name.to_string(),
            repr,
        })
    };

    match inner.prop_type.as_str() {
        "BoolProperty" => Some(OptionalValue::Bool),
        "Int8Property" => plain("i8"),
        "ByteProperty" => match inner.enum_name.as_deref() {
            Some(en) => enumeration(en),
            None => plain("u8"),
        },
        "EnumProperty" => enumeration(inner.enum_name.as_deref()?),
        "Int16Property" => plain("i16"),
        "UInt16Property" => plain("u16"),
        "IntProperty" => plain("i32"),
        "UInt32Property" => plain("u32"),
        "Int64Property" => plain("i64"),
        "UInt64Property" => plain("u64"),
        "FloatProperty" => plain("f32"),
        "DoubleProperty" => plain("f64"),
        "StructProperty" => {
            let name = inner.struct_name.as_deref()?;
            let si = ctx.structs.get(name)?;

            Some(OptionalValue::Struct {
                cpp_name: si.cpp_name.clone(),
                name: name.to_string(),
            })
        }
        "ObjectProperty" => {
            let class = inner.class_name.as_deref()?;

            ctx.classes
                .contains_key(class)
                .then(|| OptionalValue::Object(class.to_string()))
        }
        _ => None,
    }
}

fn generate_optional_property(
    out: &mut String,
    prop: &PropertyInfo,
    pctx: &PropertyContext,
    ctx: &CodegenContext,
    suppress_setters: &std::collections::HashSet<String>,
) {
    let Some(value) = prop
        .inner_prop
        .as_deref()
        .and_then(|inner| optional_value(inner, ctx))
    else {
        return;
    };

    let rust_name = to_snake_case(&prop.name);
    let byte_lit = format!("b\"{}\\0\"", prop.name);
    let prop_name_len = prop.name.len();
    let c = &pctx.container_expr;

    let (rust_type, size, decode) = match &value {
        OptionalValue::Plain(ty) => (
            ty.to_string(),
            format!("std::mem::size_of::<{ty}>()"),
            format!("Some(unsafe {{ (buf.as_ptr() as *const {ty}).read_unaligned() }})"),
        ),
        OptionalValue::Bool => ("bool".to_string(), "1".to_string(), "Some(buf[0] != 0)".to_string()),
        OptionalValue::Enum { rust_type, repr } => (
            rust_type.clone(),
            format!("std::mem::size_of::<{repr}>()"),
            format!(
                "{rust_type}::from_value(unsafe {{ (buf.as_ptr() as *const {repr}).read_unaligned() }})"
            ),
        ),
        OptionalValue::Struct { cpp_name, name } => (
            format!("rusteal_core::OwnedStruct<{cpp_name}>"),
            format!("rusteal_core::struct_size_by_name(\"{name}\")"),
            "Some(unsafe { rusteal_core::OwnedStruct::from_constructed(buf) })".to_string(),
        ),
        OptionalValue::Object(class) => (
            format!("rusteal_core::UObjectRef<{class}>"),
            "std::mem::size_of::<rusteal_core::UObjectHandle>()".to_string(),
            "Some(unsafe { rusteal_core::ObjectPointer::from_object_handle((buf.as_ptr() as *const rusteal_core::UObjectHandle).read_unaligned()) })".to_string(),
        ),
    };

    out.push_str(&format!(
        "    fn get_{rust_name}(&self) -> Option<{rust_type}> {{\n"
    ));

    emit_prop_lookup(out, &byte_lit, prop_name_len, pctx);
    emit_pre_access(out, pctx);

    out.push_str(&format!(
        "        let mut buf = vec![0u8; {size}];\n\
         \x20       let mut is_set = false;\n\
         \x20       rusteal_core::ffi_infallible_ctx(unsafe {{\n\
         \x20           rusteal_core::ffi_dispatch::property_get_optional({c}, prop, buf.as_mut_ptr(), buf.len() as u32, &mut is_set)\n\
         \x20       }}, \"{rust_name}\");\n\
         \x20       if !is_set {{\n\
         \x20           return None;\n\
         \x20       }}\n\
         \x20       {decode}\n\
         \x20   }}\n\n"
    ));

    if suppress_setters.contains(&format!("set_{rust_name}")) {
        return;
    }

    let (param_type, encode) = match &value {
        OptionalValue::Plain(_) => (rust_type.clone(), "let bytes = v.to_ne_bytes();".to_string()),
        OptionalValue::Bool => ("bool".to_string(), "let bytes = [v as u8];".to_string()),
        OptionalValue::Enum { repr, .. } => (
            rust_type.clone(),
            format!("let bytes = (v as {repr}).to_ne_bytes();"),
        ),
        OptionalValue::Struct { cpp_name, .. } => (
            format!("&rusteal_core::OwnedStruct<{cpp_name}>"),
            "let bytes = v.as_bytes();".to_string(),
        ),
        OptionalValue::Object(_) => (
            rust_type.clone(),
            "let bytes = (rusteal_core::ObjectPointer::object_handle(&v).0 as usize).to_ne_bytes();"
                .to_string(),
        ),
    };

    out.push_str(&format!(
        "    fn set_{rust_name}(&self, val: Option<{param_type}>) {{\n"
    ));

    emit_prop_lookup(out, &byte_lit, prop_name_len, pctx);
    emit_pre_access(out, pctx);

    out.push_str(&format!(
        "        let result = match val {{\n\
         \x20           None => unsafe {{ rusteal_core::ffi_dispatch::property_set_optional({c}, prop, std::ptr::null()) }},\n\
         \x20           Some(v) => {{\n\
         \x20               {encode}\n\
         \x20               unsafe {{ rusteal_core::ffi_dispatch::property_set_optional({c}, prop, bytes.as_ptr()) }}\n\
         \x20           }}\n\
         \x20       }};\n\
         \x20       rusteal_core::ffi_infallible_ctx(result, \"{rust_name}\");\n\
         \x20   }}\n\n"
    ));
}

fn generate_text_getter(
    out: &mut String,
    rust_name: &str,
    byte_lit: &str,
    prop_name_len: usize,
    pctx: &PropertyContext,
) {
    let c = &pctx.container_expr;

    out.push_str(&format!(
        "    fn get_{rust_name}(&self) -> rusteal_core::Text {{\n"
    ));

    emit_prop_lookup(out, byte_lit, prop_name_len, pctx);
    emit_pre_access(out, pctx);

    out.push_str(&format!(
        "        let mut raw: *mut u8 = std::ptr::null_mut();\n\
         \x20       rusteal_core::ffi_infallible_ctx(unsafe {{\n\
         \x20           rusteal_core::ffi_dispatch::property_get_text({c}, prop, &mut raw)\n\
         \x20       }}, \"{rust_name}\");\n\
         \x20       unsafe {{ rusteal_core::Text::from_raw(raw) }}\n\
         \x20   }}\n\n"
    ));
}

fn generate_text_setter(
    out: &mut String,
    rust_name: &str,
    byte_lit: &str,
    prop_name_len: usize,
    pctx: &PropertyContext,
) {
    let c = &pctx.container_expr;

    out.push_str(&format!(
        "    fn set_{rust_name}(&self, val: impl Into<rusteal_core::Text>) {{\n"
    ));

    emit_prop_lookup(out, byte_lit, prop_name_len, pctx);
    emit_pre_access(out, pctx);

    out.push_str(&format!(
        "        let val: rusteal_core::Text = val.into();\n\
         \x20       rusteal_core::ffi_infallible_ctx(unsafe {{\n\
         \x20           rusteal_core::ffi_dispatch::property_set_text({c}, prop, val.as_raw())\n\
         \x20       }}, \"{rust_name}\");\n\
         \x20   }}\n\n"
    ));
}

fn generate_soft_getter(
    out: &mut String,
    rust_name: &str,
    byte_lit: &str,
    prop_name_len: usize,
    pctx: &PropertyContext,
    mapped: &MappedType,
) {
    let rust_type = &mapped.rust_type;
    let c = &pctx.container_expr;

    out.push_str(&format!(
        "    fn get_{rust_name}(&self) -> {rust_type} {{\n"
    ));

    emit_prop_lookup(out, byte_lit, prop_name_len, pctx);
    emit_pre_access(out, pctx);

    out.push_str(&format!(
        "        let mut out_len: u32 = 0;\n\
         \x20       rusteal_core::ffi_infallible_ctx(unsafe {{\n\
         \x20           rusteal_core::ffi_dispatch::property_get_soft_object_path({c}, prop, std::ptr::null_mut(), 0, &mut out_len)\n\
         \x20       }}, \"{rust_name}\");\n\
         \x20       let mut buf = vec![0u8; out_len as usize];\n\
         \x20       rusteal_core::ffi_infallible_ctx(unsafe {{\n\
         \x20           rusteal_core::ffi_dispatch::property_get_soft_object_path({c}, prop, buf.as_mut_ptr(), buf.len() as u32, &mut out_len)\n\
         \x20       }}, \"{rust_name}\");\n\
         \x20       buf.truncate(out_len as usize);\n\
         \x20       <{rust_type}>::new(String::from_utf8(buf).unwrap_or_default())\n\
         \x20   }}\n\n"
    ));
}

fn generate_soft_setter(
    out: &mut String,
    rust_name: &str,
    byte_lit: &str,
    prop_name_len: usize,
    pctx: &PropertyContext,
    mapped: &MappedType,
) {
    let rust_type = &mapped.rust_type;
    let c = &pctx.container_expr;

    out.push_str(&format!(
        "    fn set_{rust_name}(&self, val: &{rust_type}) {{\n"
    ));

    emit_prop_lookup(out, byte_lit, prop_name_len, pctx);
    emit_pre_access(out, pctx);

    out.push_str(&format!(
        "        let path = val.path();\n\
         \x20       rusteal_core::ffi_infallible_ctx(unsafe {{\n\
         \x20           rusteal_core::ffi_dispatch::property_set_soft_object_path({c}, prop, path.as_ptr(), path.len() as u32)\n\
         \x20       }}, \"{rust_name}\");\n\
         \x20   }}\n\n"
    ));
}

fn generate_object_getter(
    out: &mut String,
    rust_name: &str,
    byte_lit: &str,
    prop_name_len: usize,
    pctx: &PropertyContext,
    mapped: &MappedType,
) {
    let rust_type = &mapped.rust_type;
    let c = &pctx.container_expr;

    out.push_str(&format!(
        "    fn get_{rust_name}(&self) -> {rust_type} {{\n"
    ));

    emit_prop_lookup(out, byte_lit, prop_name_len, pctx);
    emit_pre_access(out, pctx);

    out.push_str(&format!(
        "        let mut raw = rusteal_core::UObjectHandle::null();\n\
         \x20       rusteal_core::ffi_infallible_ctx(unsafe {{ rusteal_core::ffi_dispatch::property_get_object({c}, prop, &mut raw) }}, \"{rust_name}\");\n\
         \x20       unsafe {{ rusteal_core::ObjectPointer::from_object_handle(raw) }}\n\
         \x20   }}\n\n"
    ));
}

fn generate_enum_getter(
    out: &mut String,
    rust_name: &str,
    byte_lit: &str,
    prop_name_len: usize,
    pctx: &PropertyContext,
    mapped: &MappedType,
    actual_repr: &str,
) {
    let rust_type = &mapped.rust_type;
    let c = &pctx.container_expr;

    out.push_str(&format!(
        "    fn get_{rust_name}(&self) -> {rust_type} {{\n"
    ));

    emit_prop_lookup(out, byte_lit, prop_name_len, pctx);
    emit_pre_access(out, pctx);

    out.push_str(&format!(
        "        let mut raw: i64 = 0;\n\
         \x20       rusteal_core::ffi_infallible_ctx(unsafe {{ rusteal_core::ffi_dispatch::property_get_enum({c}, prop, &mut raw) }}, \"{rust_name}\");\n\
         \x20       {rust_type}::from_value(raw as {actual_repr}).expect(\"unknown enum value\")\n\
         \x20   }}\n\n"
    ));
}

fn generate_fname_getter(
    out: &mut String,
    rust_name: &str,
    byte_lit: &str,
    prop_name_len: usize,
    pctx: &PropertyContext,
) {
    let c = &pctx.container_expr;

    out.push_str(&format!(
        "    fn get_{rust_name}(&self) -> rusteal_core::FNameHandle {{\n"
    ));

    emit_prop_lookup(out, byte_lit, prop_name_len, pctx);
    emit_pre_access(out, pctx);

    out.push_str(&format!(
        "        let mut out = rusteal_core::FNameHandle(0);\n\
         \x20       rusteal_core::ffi_infallible_ctx(unsafe {{ rusteal_core::ffi_dispatch::property_get_fname({c}, prop, &mut out) }}, \"{rust_name}\");\n\
         \x20       out\n\
         \x20   }}\n\n"
    ));
}

fn generate_primitive_setter(
    out: &mut String,
    rust_name: &str,
    byte_lit: &str,
    prop_name_len: usize,
    pctx: &PropertyContext,
    mapped: &MappedType,
) {
    let rust_type = &mapped.rust_type;
    let setter = &mapped.property_setter;
    let c = &pctx.container_expr;

    out.push_str(&format!(
        "    fn set_{rust_name}(&self, val: {rust_type}) {{\n"
    ));

    emit_prop_lookup(out, byte_lit, prop_name_len, pctx);
    emit_pre_access(out, pctx);

    out.push_str(&format!(
        "        rusteal_core::ffi_infallible_ctx(unsafe {{ rusteal_core::ffi_dispatch::property_{setter}({c}, prop, val) }}, \"{rust_name}\");\n\
         \x20   }}\n\n"
    ));
}

fn generate_int_cast_setter(
    out: &mut String,
    rust_name: &str,
    byte_lit: &str,
    prop_name_len: usize,
    pctx: &PropertyContext,
    mapped: &MappedType,
) {
    let rust_type = &mapped.rust_type;
    let ffi_type = &mapped.rust_ffi_type;
    let setter = &mapped.property_setter;
    let c = &pctx.container_expr;

    out.push_str(&format!(
        "    fn set_{rust_name}(&self, val: {rust_type}) {{\n"
    ));

    emit_prop_lookup(out, byte_lit, prop_name_len, pctx);
    emit_pre_access(out, pctx);

    out.push_str(&format!(
        "        rusteal_core::ffi_infallible_ctx(unsafe {{ rusteal_core::ffi_dispatch::property_{setter}({c}, prop, val as {ffi_type}) }}, \"{rust_name}\");\n\
         \x20   }}\n\n"
    ));
}

fn generate_string_setter(
    out: &mut String,
    rust_name: &str,
    byte_lit: &str,
    prop_name_len: usize,
    pctx: &PropertyContext,
) {
    let c = &pctx.container_expr;

    out.push_str(&format!("    fn set_{rust_name}(&self, val: &str) {{\n"));

    emit_prop_lookup(out, byte_lit, prop_name_len, pctx);
    emit_pre_access(out, pctx);

    out.push_str(&format!(
        "        rusteal_core::ffi_infallible_ctx(unsafe {{\n\
         \x20           rusteal_core::ffi_dispatch::property_set_string({c}, prop, val.as_ptr(), val.len() as u32)\n\
         \x20       }}, \"{rust_name}\");\n\
         \x20   }}\n\n"
    ));
}

fn generate_object_setter(
    out: &mut String,
    rust_name: &str,
    byte_lit: &str,
    prop_name_len: usize,
    pctx: &PropertyContext,
    mapped: &MappedType,
) {
    let rust_type = &mapped.rust_type;
    let c = &pctx.container_expr;

    out.push_str(&format!(
        "    fn set_{rust_name}(&self, val: {rust_type}) {{\n"
    ));

    emit_prop_lookup(out, byte_lit, prop_name_len, pctx);
    emit_pre_access(out, pctx);

    out.push_str(&format!(
        "        rusteal_core::ffi_infallible_ctx(unsafe {{ rusteal_core::ffi_dispatch::property_set_object({c}, prop, rusteal_core::ObjectPointer::object_handle(&val)) }}, \"{rust_name}\");\n\
         \x20   }}\n\n"
    ));
}

fn generate_enum_setter(
    out: &mut String,
    rust_name: &str,
    byte_lit: &str,
    prop_name_len: usize,
    pctx: &PropertyContext,
    mapped: &MappedType,
) {
    let rust_type = &mapped.rust_type;
    let c = &pctx.container_expr;

    out.push_str(&format!(
        "    fn set_{rust_name}(&self, val: {rust_type}) {{\n"
    ));

    emit_prop_lookup(out, byte_lit, prop_name_len, pctx);
    emit_pre_access(out, pctx);

    out.push_str(&format!(
        "        rusteal_core::ffi_infallible_ctx(unsafe {{ rusteal_core::ffi_dispatch::property_set_enum({c}, prop, val as i64) }}, \"{rust_name}\");\n\
         \x20   }}\n\n"
    ));
}

fn generate_fname_setter(
    out: &mut String,
    rust_name: &str,
    byte_lit: &str,
    prop_name_len: usize,
    pctx: &PropertyContext,
) {
    let c = &pctx.container_expr;

    out.push_str(&format!(
        "    fn set_{rust_name}(&self, val: rusteal_core::FNameHandle) {{\n"
    ));

    emit_prop_lookup(out, byte_lit, prop_name_len, pctx);
    emit_pre_access(out, pctx);

    out.push_str(&format!(
        "        rusteal_core::ffi_infallible_ctx(unsafe {{ rusteal_core::ffi_dispatch::property_set_fname({c}, prop, val) }}, \"{rust_name}\");\n\
         \x20   }}\n\n"
    ));
}

fn generate_struct_getter(
    out: &mut String,
    rust_name: &str,
    byte_lit: &str,
    prop_name_len: usize,
    pctx: &PropertyContext,
    struct_cpp: &str,
) {
    let c = &pctx.container_expr;

    out.push_str(&format!(
        "    fn get_{rust_name}(&self) -> rusteal_core::OwnedStruct<{struct_cpp}> {{\n"
    ));

    emit_prop_lookup(out, byte_lit, prop_name_len, pctx);
    emit_pre_access(out, pctx);

    out.push_str(&format!(
        "        let size = unsafe {{ rusteal_core::ffi_dispatch::reflection_get_property_size(prop) }} as usize;\n\
         \x20       let mut buf = vec![0u8; size];\n\
         \x20       rusteal_core::ffi_infallible_ctx(unsafe {{\n\
         \x20           rusteal_core::ffi_dispatch::property_get_struct({c}, prop, buf.as_mut_ptr(), size as u32)\n\
         \x20       }}, \"{rust_name}\");\n\
         \x20       unsafe {{ rusteal_core::OwnedStruct::from_constructed(buf) }}\n\
         \x20   }}\n\n"
    ));
}

fn generate_struct_setter(
    out: &mut String,
    rust_name: &str,
    byte_lit: &str,
    prop_name_len: usize,
    pctx: &PropertyContext,
    struct_cpp: &str,
) {
    let c = &pctx.container_expr;

    out.push_str(&format!(
        "    fn set_{rust_name}(&self, val: &rusteal_core::OwnedStruct<{struct_cpp}>) {{\n"
    ));

    emit_prop_lookup(out, byte_lit, prop_name_len, pctx);
    emit_pre_access(out, pctx);

    out.push_str(&format!(
        "        let __bytes = val.to_bytes();\n\
         \x20       rusteal_core::ffi_infallible_ctx(unsafe {{\n\
         \x20           rusteal_core::ffi_dispatch::property_set_struct({c}, prop, __bytes.as_ptr(), __bytes.len() as u32)\n\
         \x20       }}, \"{rust_name}\");\n\
         \x20   }}\n\n"
    ));
}

#[allow(clippy::too_many_arguments)]
fn generate_fixed_array_property(
    out: &mut String,
    prop: &PropertyInfo,
    rust_name: &str,
    byte_lit: &str,
    prop_name_len: usize,
    pctx: &PropertyContext,
    ctx: &CodegenContext,
    mapped: &MappedType,
) {
    let array_dim = prop.array_dim;
    let c = &pctx.container_expr;

    let (getter_ret_type, getter_conversion, setter_param_type, setter_conversion) = match mapped
        .rust_to_ffi
    {
        ConversionKind::Identity if mapped.rust_type == "bool" => (
            "bool".to_string(),
            "Ok(buf[0] != 0)".to_string(),
            "bool".to_string(),
            "let mut buf = vec![0u8; elem_size];\n\
             \x20       if val { buf[0] = 1; }"
                .to_string(),
        ),
        ConversionKind::Identity | ConversionKind::IntCast => {
            let rust_type = &mapped.rust_type;
            let byte_count = rust_type_byte_size(rust_type);

            (
                rust_type.clone(),
                format!("Ok({rust_type}::from_ne_bytes(buf[..{byte_count}].try_into().unwrap()))"),
                rust_type.clone(),
                "let buf = val.to_ne_bytes().to_vec();".to_string(),
            )
        }
        ConversionKind::ObjectRef => {
            let rust_type = &mapped.rust_type;

            (
                rust_type.clone(),
                "let handle = rusteal_core::UObjectHandle::from_addr(u64::from_ne_bytes(buf[..8].try_into().unwrap()));\n\
                 \x20       Ok(unsafe { rusteal_core::ObjectPointer::from_object_handle(handle) })".to_string(),
                rust_type.clone(),
                "let buf = rusteal_core::ObjectPointer::object_handle(&val).to_addr().to_ne_bytes().to_vec();".to_string(),
            )
        }
        ConversionKind::EnumCast => {
            let rust_type = &mapped.rust_type;

            let actual_repr = prop
                .enum_name
                .as_deref()
                .and_then(|en| ctx.enum_actual_repr(en))
                .unwrap_or(&mapped.rust_ffi_type);

            let byte_count = rust_type_byte_size(actual_repr);

            (
                rust_type.clone(),
                format!(
                    "let raw = {actual_repr}::from_ne_bytes(buf[..{byte_count}].try_into().unwrap());\n\
                     \x20       {rust_type}::from_value(raw).ok_or_else(|| rusteal_core::RustealError::type_mismatch())"
                ),
                rust_type.clone(),
                format!("let buf = (val as {actual_repr}).to_ne_bytes().to_vec();"),
            )
        }
        ConversionKind::StructOpaque => {
            let struct_cpp = prop
                .struct_name
                .as_deref()
                .and_then(|sn| ctx.structs.get(sn))
                .map(|si| si.cpp_name.clone())
                .unwrap_or_else(|| {
                    format!("F{}", prop.struct_name.as_deref().unwrap_or("Unknown"))
                });

            (
                format!("rusteal_core::OwnedStruct<{struct_cpp}>"),
                "Ok(unsafe { rusteal_core::OwnedStruct::from_constructed(buf) })".to_string(),
                format!("&rusteal_core::OwnedStruct<{struct_cpp}>"),
                "let buf = val.to_bytes();".to_string(),
            )
        }
        _ => return,
    };

    out.push_str(&format!(
        "    #[track_caller]\n    fn get_{rust_name}(&self, index: u32) -> rusteal_core::RustealResult<{getter_ret_type}> {{\n"
    ));

    emit_prop_lookup(out, byte_lit, prop_name_len, pctx);
    emit_pre_access(out, pctx);

    out.push_str(&format!(
        "        if index >= {array_dim} {{ return Err(rusteal_core::RustealError::index_out_of_range()); }}\n\
         \x20       let elem_size = unsafe {{ rusteal_core::ffi_dispatch::reflection_get_element_size(prop) }} as usize;\n\
         \x20       let mut buf = vec![0u8; elem_size];\n\
         \x20       rusteal_core::check_ffi_ctx(unsafe {{\n\
         \x20           rusteal_core::ffi_dispatch::property_get_property_at({c}, prop, index, buf.as_mut_ptr(), elem_size as u32)\n\
         \x20       }}, \"{rust_name}\")?;\n\
         \x20       {getter_conversion}\n\
         \x20   }}\n\n"
    ));

    out.push_str(&format!(
        "    #[track_caller]\n    fn set_{rust_name}(&self, index: u32, val: {setter_param_type}) -> rusteal_core::RustealResult<()> {{\n"
    ));

    emit_prop_lookup(out, byte_lit, prop_name_len, pctx);
    emit_pre_access(out, pctx);

    out.push_str(&format!(
        "        if index >= {array_dim} {{ return Err(rusteal_core::RustealError::index_out_of_range()); }}\n\
         \x20       let elem_size = unsafe {{ rusteal_core::ffi_dispatch::reflection_get_element_size(prop) }} as usize;\n\
         \x20       {setter_conversion}\n\
         \x20       rusteal_core::check_ffi_ctx(unsafe {{\n\
         \x20           rusteal_core::ffi_dispatch::property_set_property_at({c}, prop, index, buf.as_ptr(), elem_size as u32)\n\
         \x20       }}, \"{rust_name}\")?;\n\
         \x20       Ok(())\n\
         \x20   }}\n\n"
    ));
}

fn rust_type_byte_size(rust_type: &str) -> usize {
    match rust_type {
        "bool" | "i8" | "u8" => 1,
        "i16" | "u16" => 2,
        "i32" | "u32" | "f32" => 4,
        "i64" | "u64" | "f64" => 8,
        _ => 8,
    }
}

fn generate_container_getter(
    out: &mut String,
    rust_name: &str,
    byte_lit: &str,
    prop_name_len: usize,
    pctx: &PropertyContext,
    container_type: &str,
) {
    out.push_str(&format!(
        "    fn {rust_name}(&self) -> {container_type} {{\n"
    ));

    emit_prop_lookup(out, byte_lit, prop_name_len, pctx);
    emit_pre_access(out, pctx);

    let turbofish_type = container_type.replace('<', "::<");
    let c = &pctx.container_expr;

    out.push_str(&format!(
        "        {turbofish_type}::new({c}, prop)\n\
         \x20   }}\n\n"
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusteal_ue_flags::{
        CPF_BLUEPRINT_READ_ONLY, CPF_BLUEPRINT_VISIBLE, CPF_NATIVE_ACCESS_SPECIFIER_PRIVATE,
        CPF_NATIVE_ACCESS_SPECIFIER_PUBLIC,
    };

    fn prop(name: &str, prop_type: &str, flags: u64) -> PropertyInfo {
        serde_json::from_value(serde_json::json!({
            "name": name,
            "type": prop_type,
            "prop_flags": flags,
        }))
        .unwrap()
    }

    #[test]
    fn a_blueprint_setter_alone_keeps_the_property_getter() {
        let mut setter_only = prop("MotionSource", "NameProperty", CPF_BLUEPRINT_VISIBLE);
        setter_only.setter = Some("SetTrackingMotionSource".to_string());

        let mut with_getter = prop("Hand", "NameProperty", CPF_BLUEPRINT_VISIBLE);
        with_getter.getter = Some("GetHand".to_string());

        let props = [setter_only, with_getter];
        let (names, kept) = collect_deduped_properties(&props, None);

        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].name, "MotionSource");
        assert!(names.contains("get_motion_source"));
    }

    #[test]
    fn only_non_public_read_only_properties_lose_their_setter() {
        let read_only = CPF_BLUEPRINT_VISIBLE | CPF_BLUEPRINT_READ_ONLY;

        assert!(is_read_only_non_public(&prop(
            "Mesh",
            "ObjectProperty",
            CPF_NATIVE_ACCESS_SPECIFIER_PRIVATE | read_only
        )));

        assert!(!is_read_only_non_public(&prop(
            "Mesh",
            "ObjectProperty",
            CPF_NATIVE_ACCESS_SPECIFIER_PRIVATE | CPF_BLUEPRINT_VISIBLE
        )));

        assert!(!is_read_only_non_public(&prop(
            "Speed",
            "FloatProperty",
            CPF_NATIVE_ACCESS_SPECIFIER_PUBLIC | read_only
        )));
    }

    #[test]
    fn accessor_stems() {
        assert_eq!(
            rust_property_name(&prop("CharacterMovement", "ObjectProperty", 0)),
            "character_movement"
        );

        assert_eq!(
            rust_property_name(&prop("bUseControllerRotationYaw", "BoolProperty", 0)),
            "use_controller_rotation_yaw"
        );
    }
}
