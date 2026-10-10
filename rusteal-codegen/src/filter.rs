use std::collections::{HashMap, HashSet};

use crate::config::Blocklist;
use crate::context::CodegenContext;
use crate::schema::*;
use crate::type_map;

pub fn apply_filters(ctx: &mut CodegenContext, blocklist: &Blocklist, editor: bool) {
    let blocked_classes: HashSet<&str> = blocklist.classes.iter().map(|s| s.as_str()).collect();
    let blocked_structs: HashSet<&str> = blocklist.structs.iter().map(|s| s.as_str()).collect();
    let blocked_functions: Vec<(String, String)> = blocklist.function_tuples();

    for cls in &blocklist.classes {
        ctx.classes.remove(cls);
    }

    let available_types: HashSet<String> = ctx
        .module_classes
        .iter()
        .filter(|(module, _)| ctx.enabled_modules.contains(*module))
        .flat_map(|(_, classes)| classes.iter().map(|c| c.name.clone()))
        .filter(|name| !blocked_classes.contains(name.as_str()))
        .chain(
            ctx.module_structs
                .iter()
                .filter(|(module, _)| ctx.enabled_modules.contains(*module))
                .flat_map(|(_, structs)| structs.iter().map(|s| s.name.clone()))
                .filter(|name| !blocked_structs.contains(name.as_str())),
        )
        .chain(
            ctx.module_enums
                .iter()
                .filter(|(module, _)| ctx.enabled_modules.contains(*module))
                .flat_map(|(_, enums)| enums.iter().map(|e| e.name.clone())),
        )
        .collect();

    let unreachable_headers: HashSet<String> = ctx
        .module_classes
        .values()
        .flat_map(|classes| {
            classes
                .iter()
                .filter(|c| !c.header_public)
                .map(|c| c.name.clone())
        })
        .chain(ctx.module_structs.values().flat_map(|structs| {
            structs
                .iter()
                .filter(|s| !s.header_public)
                .map(|s| s.name.clone())
        }))
        .collect();

    for classes in ctx.module_classes.values_mut() {
        classes.retain(|c| !blocked_classes.contains(c.name.as_str()));

        for class in classes.iter_mut() {
            class
                .props
                .retain(|p| is_property_exportable(p, &available_types));

            if !class.header_public {
                class.funcs.clear();
            }

            class
                .funcs
                .retain(|f| !references_any(f, &unreachable_headers));

            filter_functions(
                &class.name,
                &mut class.funcs,
                &available_types,
                &blocked_structs,
                &blocked_functions,
                editor,
            );
        }
    }
}

fn references_any(func: &FunctionInfo, types: &HashSet<String>) -> bool {
    let names = |p: &PropertyInfo| {
        [
            p.class_name.clone(),
            p.meta_class_name.clone(),
            p.struct_name.clone(),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
    };

    func.params.iter().any(|param| {
        let direct = [
            param.class_name.as_ref(),
            param.meta_class_name.as_ref(),
            param.struct_name.as_ref(),
        ];

        let inner = [
            param.inner_prop.as_deref(),
            param.key_prop.as_deref(),
            param.value_prop.as_deref(),
            param.element_prop.as_deref(),
        ];

        direct
            .into_iter()
            .flatten()
            .any(|name| types.contains(name))
            || inner
                .into_iter()
                .flatten()
                .flat_map(names)
                .any(|name| types.contains(&name))
    })
}

fn is_property_exportable(prop: &PropertyInfo, available: &HashSet<String>) -> bool {
    if !type_map::is_supported_type(&prop.prop_type) {
        return false;
    }

    if prop.array_dim > 1 {
        match prop.prop_type.as_str() {
            "StrProperty" | "NameProperty" | "TextProperty" => return false,
            _ => {}
        }
    }

    if prop.prop_flags & (CPF_NATIVE_ACCESS_PRIVATE | CPF_NATIVE_ACCESS_PROTECTED) != 0
        && prop.prop_flags & (CPF_BLUEPRINT_VISIBLE | CPF_EDIT | CPF_BLUEPRINT_ASSIGNABLE) == 0
    {
        return false;
    }

    if is_delegate_type(&prop.prop_type) {
        return is_delegate_exportable(prop, available);
    }

    if let Some(ref cls) = prop.class_name
        && !available.contains(cls)
    {
        return false;
    }

    if let Some(ref sn) = prop.struct_name
        && !available.contains(sn)
    {
        return false;
    }

    if let Some(ref en) = prop.enum_name
        && !available.contains(en)
    {
        return false;
    }

    if let Some(ref iface) = prop.interface_name
        && !available.contains(iface)
    {
        return false;
    }

    if prop.prop_type == "ClassProperty"
        && let Some(ref meta_cls) = prop.meta_class_name
        && !available.contains(meta_cls)
    {
        return false;
    }

    true
}

fn is_delegate_type(prop_type: &str) -> bool {
    matches!(
        prop_type,
        "DelegateProperty" | "MulticastInlineDelegateProperty" | "MulticastSparseDelegateProperty"
    )
}

fn is_delegate_exportable(prop: &PropertyInfo, available: &HashSet<String>) -> bool {
    let func_info = match &prop.func_info {
        Some(fi) => fi,
        None => return false,
    };

    let params = match func_info.get("params").and_then(|p| p.as_array()) {
        Some(params) => params,
        None => return true,
    };

    for param_value in params {
        let param_type = match param_value.get("type").and_then(|t| t.as_str()) {
            Some(t) => t,
            None => return false,
        };

        if !type_map::is_supported_type(param_type) {
            return false;
        }

        if is_delegate_type(param_type) {
            return false;
        }

        if matches!(param_type, "ArrayProperty" | "MapProperty" | "SetProperty") {
            return false;
        }

        if let Some(cls) = param_value.get("class_name").and_then(|v| v.as_str())
            && !available.contains(cls)
        {
            return false;
        }

        if let Some(sn) = param_value.get("struct_name").and_then(|v| v.as_str())
            && !available.contains(sn)
        {
            return false;
        }

        if let Some(en) = param_value.get("enum_name").and_then(|v| v.as_str())
            && !available.contains(en)
        {
            return false;
        }
    }

    true
}

fn is_container_param_exportable(param: &ParamInfo, available: &HashSet<String>) -> bool {
    match param.prop_type.as_str() {
        "ArrayProperty" => {
            if let Some(ref inner) = param.inner_prop {
                is_inner_type_exportable(inner, available)
            } else {
                false
            }
        }
        "MapProperty" => {
            let key_ok = param
                .key_prop
                .as_ref()
                .map(|k| is_inner_type_exportable(k, available))
                .unwrap_or(false);

            let val_ok = param
                .value_prop
                .as_ref()
                .map(|v| is_inner_type_exportable(v, available))
                .unwrap_or(false);

            key_ok && val_ok
        }
        "SetProperty" => {
            if let Some(ref elem) = param.element_prop {
                is_inner_type_exportable(elem, available)
            } else {
                false
            }
        }
        _ => false,
    }
}

fn is_inner_type_exportable(inner: &PropertyInfo, available: &HashSet<String>) -> bool {
    if !type_map::is_supported_type(&inner.prop_type) {
        return false;
    }

    if matches!(
        inner.prop_type.as_str(),
        "ArrayProperty" | "MapProperty" | "SetProperty"
    ) {
        return false;
    }

    if let Some(ref cls) = inner.class_name
        && !available.contains(cls)
    {
        return false;
    }

    if let Some(ref sn) = inner.struct_name
        && !available.contains(sn)
    {
        return false;
    }

    if let Some(ref en) = inner.enum_name
        && !available.contains(en)
    {
        return false;
    }

    if let Some(ref iface) = inner.interface_name
        && !available.contains(iface)
    {
        return false;
    }

    true
}

fn filter_functions(
    class_name: &str,
    funcs: &mut Vec<FunctionInfo>,
    available: &HashSet<String>,
    blocked_structs: &HashSet<&str>,
    blocked_functions: &[(String, String)],
    editor: bool,
) {
    let all_names: HashSet<String> = funcs.iter().map(|f| f.name.clone()).collect();

    funcs.retain(|f| {
        if blocked_functions
            .iter()
            .any(|(c, func)| c == class_name && func == &f.name)
        {
            return false;
        }

        if f.func_flags & FUNC_EDITOR_ONLY != 0 && !editor {
            return false;
        }

        if f.func_flags & FUNC_NATIVE == 0 {
            return false;
        }

        if f.name.starts_with("K2_") {
            let base_name = &f.name[3..];

            if all_names.contains(base_name) {
                return false;
            }
        }

        for param in &f.params {
            if !type_map::is_supported_type(&param.prop_type) {
                return false;
            }

            if is_delegate_type(&param.prop_type) {
                return false;
            }

            if matches!(
                param.prop_type.as_str(),
                "ArrayProperty" | "MapProperty" | "SetProperty"
            ) && !is_container_param_exportable(param, available)
            {
                return false;
            }

            if let Some(ref cls) = param.class_name
                && !available.contains(cls)
            {
                return false;
            }

            if let Some(ref sn) = param.struct_name
                && (!available.contains(sn) || blocked_structs.contains(sn.as_str()))
            {
                return false;
            }

            if let Some(ref en) = param.enum_name
                && !available.contains(en)
            {
                return false;
            }

            if let Some(ref iface) = param.interface_name
                && !available.contains(iface)
            {
                return false;
            }

            if param.prop_type == "ClassProperty"
                && let Some(ref meta_cls) = param.meta_class_name
                && !available.contains(meta_cls)
            {
                return false;
            }
        }

        true
    });

    for f in funcs.iter_mut() {
        f.ue_name = f.name.clone();
    }

    let mut name_counts: HashMap<String, usize> = HashMap::new();

    for f in funcs.iter() {
        *name_counts.entry(f.name.clone()).or_default() += 1;
    }

    let mut name_indices: HashMap<String, usize> = HashMap::new();

    for f in funcs.iter_mut() {
        if let Some(&count) = name_counts.get(&f.name)
            && count > 1
        {
            let idx = name_indices.entry(f.name.clone()).or_insert(0);
            *idx += 1;
            f.name = format!("{}_{}", f.name, idx);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusteal_ue_flags::{CPF_BLUEPRINT_READ_ONLY, CPF_BLUEPRINT_VISIBLE};

    fn object_prop(name: &str, flags: u64) -> PropertyInfo {
        serde_json::from_value(serde_json::json!({
            "name": name,
            "type": "ObjectProperty",
            "prop_flags": flags,
            "class_name": "CharacterMovementComponent",
        }))
        .unwrap()
    }

    #[test]
    fn private_properties_only_when_blueprint_can_reach_them() {
        let available: HashSet<String> = ["CharacterMovementComponent".to_string()].into();
        let read_only = CPF_BLUEPRINT_VISIBLE | CPF_BLUEPRINT_READ_ONLY;

        assert!(is_property_exportable(
            &object_prop("Public", 0),
            &available
        ));

        assert!(!is_property_exportable(
            &object_prop("Hidden", CPF_NATIVE_ACCESS_PRIVATE),
            &available
        ));

        assert!(!is_property_exportable(
            &object_prop("Guarded", CPF_NATIVE_ACCESS_PROTECTED),
            &available
        ));

        assert!(is_property_exportable(
            &object_prop("CharacterMovement", CPF_NATIVE_ACCESS_PRIVATE | read_only),
            &available
        ));

        assert!(is_property_exportable(
            &object_prop(
                "Guarded",
                CPF_NATIVE_ACCESS_PROTECTED | CPF_BLUEPRINT_VISIBLE
            ),
            &available
        ));

        assert!(is_property_exportable(
            &object_prop("Editable", CPF_NATIVE_ACCESS_PROTECTED | CPF_EDIT),
            &available
        ));

        assert!(is_property_exportable(
            &object_prop(
                "Assignable",
                CPF_NATIVE_ACCESS_PROTECTED | CPF_BLUEPRINT_ASSIGNABLE
            ),
            &available
        ));
    }

    fn function(params: serde_json::Value) -> FunctionInfo {
        serde_json::from_value(serde_json::json!({
            "name": "Probe",
            "func_flags": FUNC_NATIVE,
            "params": params,
        }))
        .unwrap()
    }

    fn param(fields: serde_json::Value) -> serde_json::Value {
        let mut param = serde_json::json!({
            "name": "P", "type": "ObjectProperty", "prop_flags": 0,
            "enum_name": null, "enum_cpp_name": null, "enum_cpp_form": null,
            "enum_underlying_type": null, "class_name": null, "meta_class_name": null,
            "struct_name": null, "interface_name": null, "func_info": null,
            "inner_prop": null, "key_prop": null, "value_prop": null,
            "element_prop": null, "default": null,
        });

        for (key, value) in fields.as_object().unwrap() {
            param[key] = value.clone();
        }

        param
    }

    #[test]
    fn functions_naming_unreachable_headers_drop_out() {
        let internal: HashSet<String> = ["NiagaraDataInterfaceArrayMesh".to_string()].into();
        let takes = |fields| function(serde_json::json!([param(fields)]));

        assert!(references_any(
            &takes(serde_json::json!({"class_name": "NiagaraDataInterfaceArrayMesh"})),
            &internal
        ));

        assert!(references_any(
            &takes(serde_json::json!({
                "type": "ArrayProperty",
                "inner_prop": {"name": "P", "type": "ObjectProperty", "prop_flags": 0,
                               "class_name": "NiagaraDataInterfaceArrayMesh"},
            })),
            &internal
        ));

        assert!(!references_any(
            &takes(serde_json::json!({"class_name": "NiagaraComponent"})),
            &internal
        ));

        assert!(!references_any(&function(serde_json::json!([])), &internal));
    }
}
