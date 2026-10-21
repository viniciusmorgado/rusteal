use std::fmt::Write;
use std::path::Path;

use serde::Deserialize;

#[derive(Deserialize, Default)]
pub struct ShimFile {
    #[serde(default)]
    pub shim: Vec<ShimSpec>,
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
    pub virtuals: Vec<VirtualSpec>,
    #[serde(default)]
    pub interfaces: Vec<InterfaceSpec>,
    #[serde(default)]
    pub constructor: Vec<String>,
    #[serde(default)]
    pub post_init_properties: Vec<String>,
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
    order: Order,
    kept: Vec<Param>,
    pure_interface: bool,
}

struct Shim<'a> {
    spec: &'a ShimSpec,
    cpp_name: String,
    category: String,
    virtuals: Vec<Virtual>,
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

        let kept: Vec<Param> = sig
            .params
            .iter()
            .filter(|p| !details.drop.contains(&p.name))
            .cloned()
            .collect();

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

        let event = details
            .event
            .clone()
            .unwrap_or_else(|| format!("Receive{}", sig.name));

        if virtuals.iter().any(|v: &Virtual| v.event == event) {
            return Err(format!(
                "{}: two virtuals raise the event {event}",
                spec.class
            ));
        }

        virtuals.push(Virtual {
            sig,
            event,
            order: details.order,
            kept,
            pure_interface,
        });
    }

    Ok(Shim {
        spec,
        cpp_name,
        category,
        virtuals,
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

            write!(
                out,
                "\n  UFUNCTION({kind}, Category = \"{}\")\n  {ret} {}({}){constness};\n",
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
            let event_call = format!("{}({})", v.event, arg_list(&v.kept));

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

                if v.pure_interface {
                    if v.sig.ret != "void" {
                        writeln!(out, "  return {{}};").unwrap();
                    }
                } else if v.kept.len() != v.sig.params.len() {
                    return Err(format!(
                        "{}::{}: a virtual whose event has a default implementation cannot drop \
                         parameters, since that default calls the parent's with them",
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
    }

    Ok(out)
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
}
