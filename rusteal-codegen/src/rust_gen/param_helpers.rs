use crate::context::CodegenContext;
use crate::naming::{escape_reserved, to_snake_case};
use crate::schema::*;
use crate::type_map::{ConversionKind, MappedType};

use super::classes::{is_soft, is_struct_owned};
use super::properties;

/// A zeroed buffer as large as the engine's struct `struct_name`, its size
/// looked up once per call site. The C++ wrapper copies the whole struct into
/// it (`sizeof`), so a guessed size overruns it for larger structs.
pub fn struct_buffer_expr(struct_name: Option<&str>) -> String {
    let name = struct_name.expect("a struct parameter has a struct name");

    format!(
        "{{ static SIZE: std::sync::OnceLock<usize> = std::sync::OnceLock::new(); \
         vec![0u8; *SIZE.get_or_init(|| rusteal_core::struct_size_by_name(\"{name}\"))] }}"
    )
}

pub fn emit_out_param_var_decl(out: &mut String, param: &ParamInfo, mapped: &MappedType) {
    let pname = escape_reserved(&to_snake_case(&param.name));

    match mapped.ffi_to_rust {
        ConversionKind::StructOpaque => {
            out.push_str(&format!(
                "        let mut {pname}_buf = {};\n",
                struct_buffer_expr(param.struct_name.as_deref())
            ));
        }
        ConversionKind::StringUtf8 => {
            out.push_str(&format!("        let mut {pname}_buf = vec![0u8; 512];\n"));
            out.push_str(&format!("        let mut {pname}_len: u32 = 0;\n"));
        }
        ConversionKind::ObjectRef => {
            out.push_str(&format!(
                "        let mut {pname} = rusteal_core::UObjectHandle::null();\n"
            ));
        }
        ConversionKind::EnumCast => {
            out.push_str(&format!(
                "        let mut {pname}: {} = 0;\n",
                mapped.rust_ffi_type
            ));
        }
        _ => {
            let default = properties::default_value_for(&mapped.rust_ffi_type);
            out.push_str(&format!("        let mut {pname} = {default};\n"));
        }
    }
}

pub fn emit_inout_string_buf_decl(out: &mut String, param: &ParamInfo, mapped: &MappedType) {
    if mapped.ffi_to_rust == ConversionKind::StringUtf8 {
        let pname = escape_reserved(&to_snake_case(&param.name));
        out.push_str(&format!("        let mut {pname}_buf = vec![0u8; 512];\n"));
        out.push_str(&format!("        let mut {pname}_len: u32 = 0;\n"));
    }
}

pub fn emit_out_param_conversion(
    out: &mut String,
    param: &ParamInfo,
    mapped: &MappedType,
    ctx: &CodegenContext,
) -> String {
    let pname = escape_reserved(&to_snake_case(&param.name));

    match mapped.ffi_to_rust {
        ConversionKind::ObjectRef => {
            format!("unsafe {{ rusteal_core::ObjectPointer::from_object_handle({pname}) }}")
        }
        ConversionKind::StringUtf8 => {
            out.push_str(&format!(
                "        let {pname}_str = rusteal_core::take_utf8({pname}_buf, {pname}_len);\n"
            ));

            if is_soft(param) {
                format!("rusteal_core::SoftObjectRef::new({pname}_str)")
            } else {
                format!("{pname}_str")
            }
        }
        ConversionKind::EnumCast => {
            let rt = &mapped.rust_type;

            let actual_repr = param
                .enum_name
                .as_deref()
                .and_then(|en| ctx.enum_actual_repr(en))
                .unwrap_or(&mapped.rust_ffi_type);

            out.push_str(&format!(
                "        let {pname}_enum = {rt}::from_value({pname} as {actual_repr}).expect(\"unknown enum value\");\n"
            ));

            format!("{pname}_enum")
        }
        ConversionKind::StructOpaque => {
            if is_struct_owned(param.struct_name.as_deref(), ctx) {
                out.push_str(&format!(
                    "        let {pname}_owned = rusteal_core::OwnedStruct::from_bytes({pname}_buf);\n"
                ));

                format!("{pname}_owned")
            } else {
                out.push_str(&format!(
                    "        let {pname}_ptr = {pname}_buf.as_ptr();\n"
                ));

                out.push_str(&format!("        std::mem::forget({pname}_buf);\n"));

                format!("{pname}_ptr")
            }
        }
        ConversionKind::IntCast => {
            let rt = &mapped.rust_type;

            format!("{pname} as {rt}")
        }
        ConversionKind::FName => pname.to_string(),
        _ => pname.to_string(),
    }
}

pub fn emit_return_expr(out: &mut String, return_parts: &[String]) {
    match return_parts.len() {
        0 => {}
        1 => out.push_str(&format!("        {}\n", return_parts[0])),
        _ => out.push_str(&format!("        ({})\n", return_parts.join(", "))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn struct_out_buffers_take_the_engine_size() {
        let param: ParamInfo = serde_json::from_value(serde_json::json!({
            "name": "SweepHitResult",
            "type": "StructProperty",
            "prop_flags": 0,
            "struct_name": "HitResult",
        }))
        .unwrap();

        let mut out = String::new();
        emit_out_param_var_decl(&mut out, &param, &super::super::classes::map_param(&param));

        assert!(
            out.contains("rusteal_core::struct_size_by_name(\"HitResult\")"),
            "{out}"
        );
        assert!(!out.contains("vec![0u8; 256]"), "{out}");
    }
}
