use std::fmt::Write;
use std::path::Path;

use serde::Deserialize;

#[derive(Deserialize, Default)]
pub struct ShimFile {
    #[serde(default)]
    pub set: Vec<SetSpec>,
    #[serde(default)]
    pub shim: Vec<ShimSpec>,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct SetSpec {
    pub name: String,
    pub virtuals: Vec<VirtualSpec>,
}

impl ShimFile {
    pub fn expanded(&self) -> Result<Vec<ShimSpec>, String> {
        self.shim
            .iter()
            .map(|shim| {
                let mut shim = shim.clone();
                let mut virtuals = Vec::new();

                for name in std::mem::take(&mut shim.sets) {
                    let set = self
                        .set
                        .iter()
                        .find(|set| set.name == name)
                        .ok_or_else(|| format!("{}: no set named {name}", shim.class))?;

                    virtuals.extend(set.virtuals.iter().cloned());
                }

                virtuals.append(&mut shim.virtuals);
                shim.virtuals = virtuals;
                Ok(shim)
            })
            .collect()
    }
}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct ShimSpec {
    pub class: String,
    pub parent: String,
    #[serde(default)]
    pub include: Vec<String>,
    #[serde(default)]
    pub modules: Vec<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub sets: Vec<String>,
    #[serde(default)]
    pub virtuals: Vec<VirtualSpec>,
    #[serde(default)]
    pub interfaces: Vec<InterfaceSpec>,
    #[serde(default)]
    pub constructor: Vec<String>,
    #[serde(default)]
    pub post_init_properties: Vec<String>,
    #[serde(default)]
    pub functions: Vec<FunctionSpec>,
}

/// A static function of the base, for what C++ reaches and Blueprint or Rust cannot: a
/// `UFUNCTION` with `specifiers` (`BlueprintCallable` when there are none), whose body is `body`.
#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct FunctionSpec {
    pub signature: String,
    #[serde(default)]
    pub specifiers: Vec<String>,
    #[serde(default)]
    pub meta: Vec<String>,
    pub body: Vec<String>,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct InterfaceSpec {
    pub name: String,
    #[serde(default)]
    pub include: Vec<String>,
    #[serde(default)]
    pub virtuals: Vec<VirtualSpec>,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum VirtualSpec {
    Signature(String),
    Detailed(VirtualDetails),
}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct VirtualDetails {
    pub signature: String,
    #[serde(default)]
    pub event: Option<String>,
    #[serde(default)]
    pub order: Order,
    #[serde(default)]
    pub drop: Vec<String>,
    #[serde(default)]
    pub meta: Vec<String>,
    #[serde(default)]
    pub event_params: Option<Vec<String>>,
    #[serde(default)]
    pub event_args: Vec<String>,
    #[serde(default)]
    pub default: Option<String>,
}

#[derive(Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Order {
    #[default]
    After,
    Before,
}

impl VirtualSpec {
    fn details(&self) -> VirtualDetails {
        match self {
            VirtualSpec::Signature(signature) => VirtualDetails {
                signature: signature.clone(),
                event: None,
                order: Order::After,
                drop: Vec::new(),
                meta: Vec::new(),
                event_params: None,
                event_args: Vec::new(),
                default: None,
            },
            VirtualSpec::Detailed(details) => details.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Param {
    pub ty: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signature {
    pub ret: String,
    pub name: String,
    pub params: Vec<Param>,
    pub is_const: bool,
}

pub fn parse_signature(text: &str) -> Result<Signature, String> {
    let text = text.trim();
    let open = text
        .find('(')
        .ok_or_else(|| format!("`{text}`: no parameter list"))?;
    let close = text
        .rfind(')')
        .ok_or_else(|| format!("`{text}`: no closing parenthesis"))?;

    if close < open {
        return Err(format!("`{text}`: unbalanced parentheses"));
    }

    let head = text[..open].trim();
    let tail = text[close + 1..].trim();

    let mut is_const = false;

    for word in tail.split_whitespace() {
        match word {
            "const" => is_const = true,
            "override" | "final" => {}
            other => {
                return Err(format!(
                    "`{text}`: unexpected `{other}` after the parameters"
                ));
            }
        }
    }

    let head = head.strip_prefix("virtual ").unwrap_or(head).trim();
    let (ret, name) = split_type_and_name(head)
        .ok_or_else(|| format!("`{text}`: expected a return type and a name"))?;

    let params = split_params(&text[open + 1..close])
        .into_iter()
        .map(|param| parse_param(&param).map_err(|e| format!("`{text}`: {e}")))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Signature {
        ret,
        name,
        params,
        is_const,
    })
}

fn split_params(list: &str) -> Vec<String> {
    let mut params = Vec::new();
    let mut depth = 0i32;
    let mut current = String::new();

    for c in list.chars() {
        match c {
            '<' | '(' => depth += 1,
            '>' | ')' => depth -= 1,
            ',' if depth == 0 => {
                params.push(std::mem::take(&mut current));
                continue;
            }
            _ => {}
        }

        current.push(c);
    }

    params.push(current);
    params
        .into_iter()
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty() && p != "void")
        .collect()
}

fn parse_param(text: &str) -> Result<Param, String> {
    let without_default = match text.find('=') {
        Some(i) => text[..i].trim(),
        None => text.trim(),
    };

    let (ty, name) = split_type_and_name(without_default)
        .ok_or_else(|| format!("parameter `{text}` needs a type and a name"))?;

    Ok(Param { ty, name })
}

fn split_type_and_name(text: &str) -> Option<(String, String)> {
    let end = text.trim_end();
    let start = end
        .rfind(|c: char| !(c.is_alphanumeric() || c == '_'))
        .map(|i| i + 1)
        .unwrap_or(0);

    let name = &end[start..];
    let ty = end[..start].trim();

    if name.is_empty() || ty.is_empty() || name.chars().next()?.is_ascii_digit() {
        return None;
    }

    Some((normalize_type(ty), name.to_string()))
}

fn normalize_type(ty: &str) -> String {
    ty.split_whitespace()
        .filter(|word| !matches!(*word, "class" | "struct" | "enum"))
        .collect::<Vec<_>>()
        .join(" ")
        .replace(" *", "*")
        .replace(" &", "&")
        .replace("< ", "<")
        .replace(" >", ">")
}

fn bare_type(ty: &str) -> &str {
    let ty = ty.strip_prefix("const ").unwrap_or(ty).trim();
    ty.trim_end_matches(['*', '&']).trim()
}

pub fn check_param_type(param: &Param) -> Result<(), String> {
    let ty = param.ty.as_str();
    let bare = bare_type(ty);

    if ty.ends_with('*') {
        let object = bare.starts_with('U') || bare.starts_with('A');

        if ty.ends_with("**") || !object {
            return Err(format!(
                "`{ty} {}` is not an object pointer, which an event cannot take: list it in `drop`",
                param.name
            ));
        }

        return Ok(());
    }

    const SCALARS: &[&str] = &[
        "bool", "int8", "uint8", "int16", "uint16", "int32", "uint32", "int64", "uint64", "float",
        "double",
    ];

    let reflected = SCALARS.contains(&bare)
        || bare.starts_with('F')
        || bare.starts_with('E')
        || bare.starts_with("TEnumAsByte<")
        || bare.starts_with("TSubclassOf<")
        || bare.starts_with("TObjectPtr<")
        || bare.starts_with("TArray<");

    if reflected {
        Ok(())
    } else {
        Err(format!(
            "`{ty} {}` is not a type a Blueprint event takes: list it in `drop`",
            param.name
        ))
    }
}

struct Virtual {
    sig: Signature,
    event: String,
    meta: Vec<String>,
    order: Order,
    kept: Vec<Param>,
    args: Vec<String>,
    default: Option<String>,
    pure_interface: bool,
}

struct Function<'a> {
    sig: Signature,
    spec: &'a FunctionSpec,
}

struct Shim<'a> {
    spec: &'a ShimSpec,
    cpp_name: String,
    category: String,
    virtuals: Vec<Virtual>,
    functions: Vec<Function<'a>>,
}

fn prefix_of(parent: &str) -> Result<char, String> {
    match parent.chars().next() {
        Some(c @ ('A' | 'U')) => Ok(c),
        _ => Err(format!(
            "parent `{parent}` must be a C++ class name with its prefix (AActor, UActorComponent)"
        )),
    }
}

fn resolve(spec: &ShimSpec) -> Result<Shim<'_>, String> {
    let prefix = prefix_of(&spec.parent)?;

    if !spec.sets.is_empty() {
        return Err(format!(
            "{}: `sets` names virtuals of the curated list only",
            spec.class
        ));
    }

    if spec.class.is_empty() || !spec.class.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err(format!("`{}` is not a class name", spec.class));
    }

    let cpp_name = format!("{prefix}{}", spec.class);
    let category = spec.category.clone().unwrap_or_else(|| {
        format!(
            "Rusteal|{}",
            spec.class.strip_prefix("Rusteal").unwrap_or(&spec.class)
        )
    });

    let mut virtuals = Vec::new();

    let own = spec.virtuals.iter().map(|v| (v, false));
    let from_interfaces = spec
        .interfaces
        .iter()
        .flat_map(|i| i.virtuals.iter().map(|v| (v, true)));

    for (virtual_spec, pure_interface) in own.chain(from_interfaces) {
        let details = virtual_spec.details();
        let sig =
            parse_signature(&details.signature).map_err(|e| format!("{}: {e}", spec.class))?;

        for dropped in &details.drop {
            if !sig.params.iter().any(|p| &p.name == dropped) {
                return Err(format!(
                    "{}::{}: `drop` names `{dropped}`, which is not a parameter",
                    spec.class, sig.name
                ));
            }
        }

        let (kept, args) = match &details.event_params {
            Some(declared) => {
                if !details.drop.is_empty() {
                    return Err(format!(
                        "{}::{}: `event_params` declares the event's parameters, so `drop` is \
                         not needed",
                        spec.class, sig.name
                    ));
                }

                if declared.len() != details.event_args.len() {
                    return Err(format!(
                        "{}::{}: `event_params` has {} parameters and `event_args` {} \
                         expressions; each parameter needs the expression that fills it",
                        spec.class,
                        sig.name,
                        declared.len(),
                        details.event_args.len()
                    ));
                }

                let params = declared
                    .iter()
                    .map(|text| parse_param(text.trim()))
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|e| format!("{}::{}: {e}", spec.class, sig.name))?;

                (params, details.event_args.clone())
            }
            None => {
                if !details.event_args.is_empty() {
                    return Err(format!(
                        "{}::{}: `event_args` goes with `event_params`",
                        spec.class, sig.name
                    ));
                }

                let params: Vec<Param> = sig
                    .params
                    .iter()
                    .filter(|p| !details.drop.contains(&p.name))
                    .cloned()
                    .collect();
                let args = params.iter().map(|p| p.name.clone()).collect();

                (params, args)
            }
        };

        for param in &kept {
            check_param_type(param).map_err(|e| format!("{}::{}: {e}", spec.class, sig.name))?;
        }

        if sig.ret != "void" {
            check_param_type(&Param {
                ty: sig.ret.clone(),
                name: "return value".to_string(),
            })
            .map_err(|e| format!("{}::{}: {e}", spec.class, sig.name))?;
        }

        let event = details.event.clone().unwrap_or_else(|| {
            format!(
                "Receive{}",
                sig.name.strip_prefix("Native").unwrap_or(&sig.name)
            )
        });

        if virtuals.iter().any(|v: &Virtual| v.event == event) {
            return Err(format!(
                "{}: two virtuals raise the event {event}",
                spec.class
            ));
        }

        let has_default = details.default.is_some();

        virtuals.push(Virtual {
            sig,
            event,
            meta: details.meta.clone(),
            order: details.order,
            kept,
            args,
            default: details.default.clone(),
            pure_interface,
        });

        let added = virtuals.last().unwrap();

        if has_default && !native_event(added) {
            return Err(format!(
                "{}::{}: `default` is the body of a BlueprintNativeEvent's implementation; a \
                 void virtual without out parameters raises a BlueprintImplementableEvent, \
                 which has none",
                spec.class, added.sig.name
            ));
        }
    }

    let mut functions = Vec::new();

    for function in &spec.functions {
        let sig =
            parse_signature(&function.signature).map_err(|e| format!("{}: {e}", spec.class))?;

        if sig.is_const {
            return Err(format!(
                "{}::{}: a base's functions are static, so not const",
                spec.class, sig.name
            ));
        }

        for param in &sig.params {
            check_param_type(param).map_err(|e| format!("{}::{}: {e}", spec.class, sig.name))?;
        }

        if sig.ret != "void" {
            check_param_type(&Param {
                ty: sig.ret.clone(),
                name: "return value".to_string(),
            })
            .map_err(|e| format!("{}::{}: {e}", spec.class, sig.name))?;
        }

        if functions.iter().any(|f: &Function| f.sig.name == sig.name)
            || virtuals
                .iter()
                .any(|v| v.event == sig.name || v.sig.name == sig.name)
        {
            return Err(format!(
                "{}: two functions are named {}",
                spec.class, sig.name
            ));
        }

        functions.push(Function {
            sig,
            spec: function,
        });
    }

    Ok(Shim {
        spec,
        cpp_name,
        category,
        virtuals,
        functions,
    })
}

fn param_list(params: &[Param]) -> String {
    params
        .iter()
        .map(|p| {
            let base = p.ty.trim_end_matches(['*', '&']);
            let marks = &p.ty[base.len()..];
            format!("{base} {marks}{}", p.name)
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn arg_list(params: &[Param]) -> String {
    params
        .iter()
        .map(|p| p.name.clone())
        .collect::<Vec<_>>()
        .join(", ")
}

fn has_out_params(v: &Virtual) -> bool {
    v.kept
        .iter()
        .any(|p| p.ty.ends_with('&') && !p.ty.starts_with("const "))
}

fn native_event(v: &Virtual) -> bool {
    v.sig.ret != "void" || v.pure_interface || has_out_params(v)
}

pub fn validate(specs: &[ShimSpec]) -> Result<(), String> {
    let mut names = std::collections::HashSet::new();

    for spec in specs {
        let shim = resolve(spec)?;

        if !names.insert(shim.cpp_name.clone()) {
            return Err(format!("two shims are named {}", shim.cpp_name));
        }
    }

    Ok(())
}

pub fn generate_header(specs: &[ShimSpec], file_stem: &str, api: &str) -> Result<String, String> {
    validate(specs)?;
    let shims = specs.iter().map(resolve).collect::<Result<Vec<_>, _>>()?;

    let mut out = String::from("#pragma once\n\n#include \"CoreMinimal.h\"\n");
    let mut includes: Vec<&str> = Vec::new();

    for shim in &shims {
        for include in shim
            .spec
            .include
            .iter()
            .chain(shim.spec.interfaces.iter().flat_map(|i| i.include.iter()))
        {
            if !includes.contains(&include.as_str()) {
                includes.push(include);
            }
        }
    }

    for include in includes {
        writeln!(out, "#include \"{include}\"").unwrap();
    }

    writeln!(out, "#include \"{file_stem}.generated.h\"").unwrap();

    for shim in &shims {
        let bases = std::iter::once(format!("public {}", shim.spec.parent))
            .chain(
                shim.spec
                    .interfaces
                    .iter()
                    .map(|i| format!("public {}", i.name)),
            )
            .collect::<Vec<_>>()
            .join(", ");

        write!(
            out,
            "\nUCLASS(Abstract, Blueprintable)\nclass {api} {} : {bases} {{\n  GENERATED_BODY()\n\npublic:\n",
            shim.cpp_name
        )
        .unwrap();

        if !shim.spec.constructor.is_empty() {
            writeln!(
                out,
                "  {}(const FObjectInitializer &ObjectInitializer);\n",
                shim.cpp_name
            )
            .unwrap();
        }

        if !shim.spec.post_init_properties.is_empty() {
            out.push_str("  virtual void PostInitProperties() override;\n");
        }

        for v in &shim.virtuals {
            writeln!(
                out,
                "  virtual {} {}({}){} override;",
                v.sig.ret,
                v.sig.name,
                param_list(&v.sig.params),
                if v.sig.is_const { " const" } else { "" }
            )
            .unwrap();
        }

        for v in &shim.virtuals {
            let kind = if native_event(v) {
                "BlueprintNativeEvent"
            } else {
                "BlueprintImplementableEvent"
            };

            let ret = &v.sig.ret;
            let constness = if v.sig.is_const { " const" } else { "" };

            let meta = if v.meta.is_empty() {
                String::new()
            } else {
                format!(", meta = ({})", v.meta.join(", "))
            };

            write!(
                out,
                "\n  UFUNCTION({kind}, Category = \"{}\"{meta})\n  {ret} {}({}){constness};\n",
                shim.category,
                v.event,
                param_list(&v.kept)
            )
            .unwrap();

            if native_event(v) {
                writeln!(
                    out,
                    "  virtual {ret} {}_Implementation({}){constness};",
                    v.event,
                    param_list(&v.kept)
                )
                .unwrap();
            }
        }

        for function in &shim.functions {
            let specifiers = if function.spec.specifiers.is_empty() {
                "BlueprintCallable".to_string()
            } else {
                function.spec.specifiers.join(", ")
            };

            let meta = if function.spec.meta.is_empty() {
                String::new()
            } else {
                format!(", meta = ({})", function.spec.meta.join(", "))
            };

            write!(
                out,
                "\n  UFUNCTION({specifiers}, Category = \"{}\"{meta})\n  static {} {}({});\n",
                shim.category,
                function.sig.ret,
                function.sig.name,
                param_list(&function.sig.params)
            )
            .unwrap();
        }

        out.push_str("};\n");
    }

    Ok(out)
}

pub fn generate_source(specs: &[ShimSpec], file_stem: &str) -> Result<String, String> {
    validate(specs)?;
    let shims = specs.iter().map(resolve).collect::<Result<Vec<_>, _>>()?;

    let mut out = format!("#include \"{file_stem}.h\"\n");

    for shim in &shims {
        let class = &shim.cpp_name;

        if !shim.spec.constructor.is_empty() {
            write!(
                out,
                "\n{class}::{class}(const FObjectInitializer &ObjectInitializer)\n    : Super(ObjectInitializer) {{\n"
            )
            .unwrap();

            for line in &shim.spec.constructor {
                writeln!(out, "  {line}").unwrap();
            }

            out.push_str("}\n");
        }

        if !shim.spec.post_init_properties.is_empty() {
            write!(
                out,
                "\nvoid {class}::PostInitProperties() {{\n  Super::PostInitProperties();\n"
            )
            .unwrap();

            for line in &shim.spec.post_init_properties {
                writeln!(out, "  {line}").unwrap();
            }

            out.push_str("}\n");
        }

        for v in &shim.virtuals {
            let constness = if v.sig.is_const { " const" } else { "" };
            let super_call = format!("Super::{}({})", v.sig.name, arg_list(&v.sig.params));
            let event_call = format!("{}({})", v.event, v.args.join(", "));

            write!(
                out,
                "\n{} {class}::{}({}){constness} {{\n",
                v.sig.ret,
                v.sig.name,
                param_list(&v.sig.params)
            )
            .unwrap();

            if native_event(v) {
                if v.sig.ret == "void" {
                    writeln!(out, "  {event_call};").unwrap();
                } else {
                    writeln!(out, "  return {event_call};").unwrap();
                }
            } else if v.order == Order::Before {
                writeln!(out, "  {event_call};\n  {super_call};").unwrap();
            } else {
                writeln!(out, "  {super_call};\n  {event_call};").unwrap();
            }

            out.push_str("}\n");

            if native_event(v) {
                write!(
                    out,
                    "\n{} {class}::{}_Implementation({}){constness} {{\n",
                    v.sig.ret,
                    v.event,
                    param_list(&v.kept)
                )
                .unwrap();

                if let Some(default) = &v.default {
                    for line in default.lines().filter(|line| !line.trim().is_empty()) {
                        writeln!(out, "  {}", line.trim_end()).unwrap();
                    }
                } else if v.pure_interface {
                    if v.sig.ret != "void" {
                        writeln!(out, "  return {{}};").unwrap();
                    }
                } else if v.kept != v.sig.params {
                    return Err(format!(
                        "{}::{}: the event's default calls the parent's virtual with the event's \
                         parameters, which are not the virtual's: give it a `default`",
                        shim.spec.class, v.sig.name
                    ));
                } else if v.sig.ret == "void" {
                    writeln!(out, "  {super_call};").unwrap();
                } else {
                    writeln!(out, "  return {super_call};").unwrap();
                }

                out.push_str("}\n");
            }
        }

        for function in &shim.functions {
            write!(
                out,
                "\n{} {class}::{}({}) {{\n",
                function.sig.ret,
                function.sig.name,
                param_list(&function.sig.params)
            )
            .unwrap();

            for line in &function.spec.body {
                writeln!(out, "  {line}").unwrap();
            }

            out.push_str("}\n");
        }
    }

    Ok(out)
}

/// The curated bases of the binding features whose engine plugin is optional: they go to the
/// project's bases when the project binds the feature.
pub const FEATURE_SHIMS: &[(&str, &str)] = &[(
    "gameplay-abilities",
    include_str!("../../shims/gameplay_abilities.toml"),
)];

pub fn feature_shims(features: &[String]) -> Result<Vec<ShimSpec>, String> {
    let mut specs = Vec::new();

    for (feature, text) in FEATURE_SHIMS {
        if features.iter().any(|f| f == feature) {
            let file: ShimFile = toml::from_str(text)
                .map_err(|e| format!("the bases of the `{feature}` feature: {e}"))?;
            specs.extend(file.shim);
        }
    }

    Ok(specs)
}

/// The bases a project builds: its features' curated ones, then its own `[[codegen.shims]]`.
pub fn project_shim_specs(features: &[String], own: &[ShimSpec]) -> Result<Vec<ShimSpec>, String> {
    let mut specs = feature_shims(features)?;
    specs.extend(own.iter().cloned());
    Ok(specs)
}

pub const PROJECT_SHIMS_STEM: &str = "RustealProjectShims";
pub const PROJECT_SHIMS_DIR: &str = "Shims";
pub const SHIM_MODULE_DEPS: &str = "shim_module_deps.txt";

pub fn project_shim_files(specs: &[ShimSpec]) -> Result<Vec<(String, String)>, String> {
    if specs.is_empty() {
        return Ok(Vec::new());
    }

    let header = generate_header(specs, PROJECT_SHIMS_STEM, "RUSTEAL_API")?;
    let source = generate_source(specs, PROJECT_SHIMS_STEM)?;

    let mut modules: Vec<&str> = specs
        .iter()
        .flat_map(|s| s.modules.iter().map(String::as_str))
        .collect();
    modules.sort();
    modules.dedup();

    Ok(vec![
        (
            format!("{PROJECT_SHIMS_DIR}/{PROJECT_SHIMS_STEM}.h"),
            header,
        ),
        (
            format!("{PROJECT_SHIMS_DIR}/{PROJECT_SHIMS_STEM}.cpp"),
            source,
        ),
        (SHIM_MODULE_DEPS.to_string(), modules.join("\n")),
    ])
}

pub fn write_project_shims(cpp_generated: &Path, specs: &[ShimSpec]) -> Result<bool, String> {
    let files = project_shim_files(specs)?;
    let mut changed = false;

    if files.is_empty() {
        for rel in [
            format!("{PROJECT_SHIMS_DIR}/{PROJECT_SHIMS_STEM}.h"),
            format!("{PROJECT_SHIMS_DIR}/{PROJECT_SHIMS_STEM}.cpp"),
            SHIM_MODULE_DEPS.to_string(),
        ] {
            changed |= std::fs::remove_file(cpp_generated.join(rel)).is_ok();
        }

        let _ = std::fs::remove_dir(cpp_generated.join(PROJECT_SHIMS_DIR));
        return Ok(changed);
    }

    for (rel, text) in files {
        let path = cpp_generated.join(&rel);

        if std::fs::read_to_string(&path).ok().as_deref() == Some(text.as_str()) {
            continue;
        }

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("cannot create {}: {e}", parent.display()))?;
        }

        std::fs::write(&path, text).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
        changed = true;
    }

    Ok(changed)
}

pub fn project_shims_up_to_date(cpp_generated: &Path, specs: &[ShimSpec]) -> bool {
    match project_shim_files(specs) {
        Ok(files) if files.is_empty() => !cpp_generated
            .join(PROJECT_SHIMS_DIR)
            .join(format!("{PROJECT_SHIMS_STEM}.h"))
            .exists(),
        Ok(files) => files.iter().all(|(rel, text)| {
            std::fs::read_to_string(cpp_generated.join(rel))
                .ok()
                .as_deref()
                == Some(text.as_str())
        }),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(toml_text: &str) -> Vec<ShimSpec> {
        toml::from_str::<ShimFile>(toml_text).unwrap().shim
    }

    #[test]
    fn parses_signatures_with_qualifiers_and_defaults() {
        let sig = parse_signature(
            "virtual void TickComponent(float DeltaTime, enum ELevelTick TickType, FActorComponentTickFunction *ThisTickFunction) override",
        )
        .unwrap();

        assert_eq!(sig.ret, "void");
        assert_eq!(sig.name, "TickComponent");
        assert_eq!(sig.params.len(), 3);
        assert_eq!(sig.params[2].ty, "FActorComponentTickFunction*");
        assert_eq!(sig.params[2].name, "ThisTickFunction");

        let sig = parse_signature("float GetMaxSpeed() const").unwrap();
        assert!(sig.is_const);
        assert!(sig.params.is_empty());

        let sig = parse_signature("void OnConstruction(const FTransform &Transform)").unwrap();
        assert_eq!(sig.params[0].ty, "const FTransform&");

        let sig =
            parse_signature("void OnMovementModeChanged(EMovementMode PreviousMovementMode, uint8 PreviousCustomMode = 0)")
                .unwrap();
        assert_eq!(sig.params[1].name, "PreviousCustomMode");

        let sig =
            parse_signature("void Set(TMap<FName, int32> Values, TEnumAsByte<EMovementMode> Mode)")
                .unwrap();
        assert_eq!(sig.params.len(), 2);
        assert_eq!(sig.params[1].ty, "TEnumAsByte<EMovementMode>");
    }

    #[test]
    fn refuses_parameters_an_event_cannot_take() {
        let shims = spec(
            r#"
            [[shim]]
            class = "RustealActorComponent"
            parent = "UActorComponent"
            virtuals = ["void TickComponent(float DeltaTime, FActorComponentTickFunction *ThisTickFunction)"]
            "#,
        );

        let error = generate_header(&shims, "X", "RUSTEAL_API").unwrap_err();
        assert!(error.contains("drop"), "{error}");
    }

    #[test]
    fn void_virtuals_call_the_parent_then_the_event() {
        let shims = spec(
            r#"
            [[shim]]
            class = "RustealActorComponent"
            parent = "UActorComponent"
            include = ["Components/ActorComponent.h"]
            virtuals = [
                { signature = "void TickComponent(float DeltaTime, ELevelTick TickType, FActorComponentTickFunction *ThisTickFunction)", drop = ["TickType", "ThisTickFunction"] },
                { signature = "void OnUnregister()", order = "before" },
            ]
            "#,
        );

        let header = generate_header(&shims, "RustealShims", "RUSTEAL_API").unwrap();
        assert!(
            header.contains("class RUSTEAL_API URustealActorComponent : public UActorComponent {")
        );
        assert!(header.contains(
            "virtual void TickComponent(float DeltaTime, ELevelTick TickType, FActorComponentTickFunction *ThisTickFunction) override;"
        ));
        assert!(header.contains("UFUNCTION(BlueprintImplementableEvent, Category = \"Rusteal|ActorComponent\")\n  void ReceiveTickComponent(float DeltaTime);"));

        let source = generate_source(&shims, "RustealShims").unwrap();
        assert!(source.contains(
            "  Super::TickComponent(DeltaTime, TickType, ThisTickFunction);\n  ReceiveTickComponent(DeltaTime);"
        ));
        assert!(source.contains("  ReceiveOnUnregister();\n  Super::OnUnregister();"));
    }

    #[test]
    fn virtuals_returning_a_value_default_to_the_parent() {
        let shims = spec(
            r#"
            [[shim]]
            class = "RustealCharacterMovementComponent"
            parent = "UCharacterMovementComponent"
            virtuals = ["float GetMaxSpeed() const"]
            "#,
        );

        let header = generate_header(&shims, "RustealShims", "RUSTEAL_API").unwrap();
        assert!(header.contains("UFUNCTION(BlueprintNativeEvent"));
        assert!(header.contains("  float ReceiveGetMaxSpeed() const;"));
        assert!(header.contains("  virtual float ReceiveGetMaxSpeed_Implementation() const;"));

        let source = generate_source(&shims, "RustealShims").unwrap();
        assert!(source.contains("float URustealCharacterMovementComponent::GetMaxSpeed() const {\n  return ReceiveGetMaxSpeed();\n}"));
        assert!(source.contains(
            "float URustealCharacterMovementComponent::ReceiveGetMaxSpeed_Implementation() const {\n  return Super::GetMaxSpeed();\n}"
        ));
    }

    #[test]
    fn interface_virtuals_have_an_empty_default() {
        let shims = spec(
            r#"
            [[shim]]
            class = "RustealTaggedActor"
            parent = "AActor"

            [[shim.interfaces]]
            name = "IGameplayTagAssetInterface"
            include = ["GameplayTagAssetInterface.h"]
            virtuals = ["void GetOwnedGameplayTags(FGameplayTagContainer &TagContainer) const"]
            "#,
        );

        let header = generate_header(&shims, "RustealShims", "RUSTEAL_API").unwrap();
        assert!(header.contains(": public AActor, public IGameplayTagAssetInterface {"));
        assert!(header.contains("#include \"GameplayTagAssetInterface.h\""));

        let source = generate_source(&shims, "RustealShims").unwrap();
        assert!(source.contains(
            "void ARustealTaggedActor::GetOwnedGameplayTags(FGameplayTagContainer &TagContainer) const {\n  ReceiveGetOwnedGameplayTags(TagContainer);\n}"
        ));
        assert!(source.contains(
            "void ARustealTaggedActor::ReceiveGetOwnedGameplayTags_Implementation(FGameplayTagContainer &TagContainer) const {\n}"
        ));
    }

    #[test]
    fn duplicate_classes_and_unknown_drops_are_refused() {
        let shims = spec(
            r#"
            [[shim]]
            class = "A"
            parent = "AActor"

            [[shim]]
            class = "A"
            parent = "AActor"
            "#,
        );
        assert!(validate(&shims).is_err());

        let shims = spec(
            r#"
            [[shim]]
            class = "A"
            parent = "AActor"
            virtuals = [{ signature = "void Destroyed()", drop = ["Nothing"] }]
            "#,
        );
        assert!(validate(&shims).is_err());
    }

    #[test]
    fn native_prefix_and_metadata() {
        let shims = spec(
            r#"
            [[shim]]
            class = "RustealAnimInstance"
            parent = "UAnimInstance"
            virtuals = [
                "void NativeInitializeAnimation()",
                { signature = "void NativeThreadSafeUpdateAnimation(float DeltaSeconds)", meta = ["BlueprintThreadSafe"] },
            ]
            "#,
        );

        let header = generate_header(&shims, "S", "RUSTEAL_API").unwrap();
        assert!(header.contains("  void ReceiveInitializeAnimation();"));
        assert!(header.contains(
            "UFUNCTION(BlueprintImplementableEvent, Category = \"Rusteal|AnimInstance\", meta = (BlueprintThreadSafe))\n  void ReceiveThreadSafeUpdateAnimation(float DeltaSeconds);"
        ));
    }

    #[test]
    fn constructor_and_post_init_lines() {
        let shims = spec(
            r#"
            [[shim]]
            class = "RustealActorComponent"
            parent = "UActorComponent"
            constructor = ["bWantsInitializeComponent = true;"]
            post_init_properties = ["Touch();"]
            "#,
        );

        let source = generate_source(&shims, "S").unwrap();
        assert!(source.contains(
            "URustealActorComponent::URustealActorComponent(const FObjectInitializer &ObjectInitializer)\n    : Super(ObjectInitializer) {\n  bWantsInitializeComponent = true;\n}"
        ));
        assert!(source.contains("  Super::PostInitProperties();\n  Touch();\n"));
    }

    #[test]
    fn event_parameters_are_filled_from_the_virtuals() {
        let shims = spec(
            r#"
            [[shim]]
            class = "RustealSet"
            parent = "UAttributeSet"
            virtuals = [
                { signature = "bool PreExecute(FCallbackData &Data)", event_params = ["const FSpec &Spec", "FEvaluated &Evaluated", "UAbilitySystemComponent *Target"], event_args = ["Data.Spec", "Data.Evaluated", "&Data.Target"], default = "return true;" },
                { signature = "void PostExecute(const FCallbackData &Data)", event_params = ["const FSpec &Spec"], event_args = ["Data.Spec"] },
            ]
            "#,
        );

        let header = generate_header(&shims, "S", "API").unwrap();
        assert!(header.contains(
            "  bool ReceivePreExecute(const FSpec &Spec, FEvaluated &Evaluated, UAbilitySystemComponent *Target);"
        ));
        assert!(header.contains("  void ReceivePostExecute(const FSpec &Spec);"));
        assert!(header.contains("BlueprintImplementableEvent"));

        let source = generate_source(&shims, "S").unwrap();
        assert!(source.contains(
            "bool URustealSet::PreExecute(FCallbackData &Data) {\n  return ReceivePreExecute(Data.Spec, Data.Evaluated, &Data.Target);\n}"
        ));
        assert!(source.contains(
            "bool URustealSet::ReceivePreExecute_Implementation(const FSpec &Spec, FEvaluated &Evaluated, UAbilitySystemComponent *Target) {\n  return true;\n}"
        ));
        assert!(source.contains("  Super::PostExecute(Data);\n  ReceivePostExecute(Data.Spec);\n"));
    }

    #[test]
    fn mapped_parameters_are_checked() {
        let refused = |virtual_spec: &str| {
            let text = format!(
                "[[shim]]\nclass = \"RustealSet\"\nparent = \"UAttributeSet\"\nvirtuals = [{virtual_spec}]\n"
            );
            generate_source(&spec(&text), "S").unwrap_err()
        };

        assert!(
            refused(r#"{ signature = "void F(const FData &Data)", event_params = ["int32 A"], event_args = [] }"#)
                .contains("each parameter needs the expression")
        );
        assert!(
            refused(r#"{ signature = "void F(const FData &Data)", event_params = ["FData *A"], event_args = ["&Data"] }"#)
                .contains("not an object pointer")
        );
        assert!(
            refused(r#"{ signature = "bool F(const FData &Data)", event_params = ["int32 A"], event_args = ["Data.A"] }"#)
                .contains("give it a `default`")
        );
        assert!(
            refused(r#"{ signature = "void F(int32 A)", default = "Touch();" }"#)
                .contains("BlueprintImplementableEvent")
        );
        assert!(
            refused(r#"{ signature = "void F(int32 A)", event_args = ["A"] }"#)
                .contains("goes with `event_params`")
        );
    }

    #[test]
    fn a_default_replaces_the_parent_call() {
        let shims = spec(
            r#"
            [[shim]]
            class = "RustealThing"
            parent = "AActor"
            virtuals = [{ signature = "float Score(float Base, UObject *Context)", drop = ["Context"], default = "return Base;" }]
            "#,
        );

        let source = generate_source(&shims, "S").unwrap();
        assert!(source.contains(
            "float ARustealThing::ReceiveScore_Implementation(float Base) {\n  return Base;\n}"
        ));
    }

    #[test]
    fn feature_bases_come_with_their_feature() {
        let own = spec("[[shim]]\nclass = \"RustealMine\"\nparent = \"AActor\"\n");

        assert_eq!(project_shim_specs(&[], &own).unwrap().len(), 1);

        let features = ["gameplay-abilities".to_string()];
        let all = project_shim_specs(&features, &own).unwrap();
        assert_eq!(all[0].class, "RustealAttributeSet");
        assert_eq!(all.last().unwrap().class, "RustealMine");

        let header = generate_header(&all, PROJECT_SHIMS_STEM, "RUSTEAL_API").unwrap();
        assert!(header.contains(
            "  UFUNCTION(BlueprintPure, Category = \"Rusteal|AttributeSet\")\n  static FGameplayAttribute AttributeOf(TSubclassOf<UAttributeSet> Set, FName Property);"
        ));
        let source = generate_source(&all, PROJECT_SHIMS_STEM).unwrap();
        assert!(source.contains(
            "FGameplayAttribute URustealAttributeSet::AttributeOf(TSubclassOf<UAttributeSet> Set, FName Property) {\n  FProperty *Found"
        ));

        for (feature, _) in FEATURE_SHIMS {
            let specs = feature_shims(&[feature.to_string()]).unwrap();
            generate_header(&specs, PROJECT_SHIMS_STEM, "RUSTEAL_API").unwrap();
            generate_source(&specs, PROJECT_SHIMS_STEM).unwrap();
        }
    }
}
