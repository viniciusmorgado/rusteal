use crate::context::CodegenContext;
use crate::naming::{strip_bool_prefix, to_snake_case};
use crate::schema::PropertyInfo;
use crate::type_map::{self, ConversionKind, MappedType};

pub struct PropertyContext {
    pub find_prop_fn: String,
    pub handle_expr: String,
    pub pre_access: String,
    pub container_expr: String,
    pub is_class: bool,
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
        if prop.getter.is_some() || prop.setter.is_some() {
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

        if !mapped.supported {
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
                    .is_some_and(|sn| ctx.structs.get(sn).is_some_and(|si| si.has_static_struct))
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
        if pctx.is_class
            && let Some(container_type) = type_map::resolve_container_rust_type(prop, Some(ctx))
        {
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
        ConversionKind::StringUtf8 => {
            generate_string_setter(out, &rust_name, &byte_lit, prop_name_len, pctx);
        }
        ConversionKind::StructOpaque => {}
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
         \x20       buf.truncate(out_len as usize);\n\
         \x20       String::from_utf8_lossy(&buf).into_owned()\n\
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
         \x20       rusteal_core::OwnedStruct::from_bytes(buf)\n\
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
                     \x20       {rust_type}::from_value(raw).ok_or(rusteal_core::RustealError::TypeMismatch)"
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
                "Ok(rusteal_core::OwnedStruct::from_bytes(buf))".to_string(),
                format!("&rusteal_core::OwnedStruct<{struct_cpp}>"),
                "let buf = val.to_bytes();".to_string(),
            )
        }
        _ => return,
    };

    out.push_str(&format!(
        "    fn get_{rust_name}(&self, index: u32) -> rusteal_core::RustealResult<{getter_ret_type}> {{\n"
    ));

    emit_prop_lookup(out, byte_lit, prop_name_len, pctx);
    emit_pre_access(out, pctx);

    out.push_str(&format!(
        "        if index >= {array_dim} {{ return Err(rusteal_core::RustealError::IndexOutOfRange); }}\n\
         \x20       let elem_size = unsafe {{ rusteal_core::ffi_dispatch::reflection_get_element_size(prop) }} as usize;\n\
         \x20       let mut buf = vec![0u8; elem_size];\n\
         \x20       rusteal_core::check_ffi_ctx(unsafe {{\n\
         \x20           rusteal_core::ffi_dispatch::property_get_property_at({c}, prop, index, buf.as_mut_ptr(), elem_size as u32)\n\
         \x20       }}, \"{rust_name}\")?;\n\
         \x20       {getter_conversion}\n\
         \x20   }}\n\n"
    ));

    out.push_str(&format!(
        "    fn set_{rust_name}(&self, index: u32, val: {setter_param_type}) -> rusteal_core::RustealResult<()> {{\n"
    ));

    emit_prop_lookup(out, byte_lit, prop_name_len, pctx);
    emit_pre_access(out, pctx);

    out.push_str(&format!(
        "        if index >= {array_dim} {{ return Err(rusteal_core::RustealError::IndexOutOfRange); }}\n\
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

    out.push_str(&format!(
        "        {turbofish_type}::new(h, prop)\n\
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
