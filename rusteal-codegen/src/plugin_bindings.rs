use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use syn::punctuated::Punctuated;
use syn::{
    Expr, FnArg, GenericArgument, ImplItem, Item, Meta, PathArguments, ReturnType, Token, Type,
};

use crate::context::CodegenContext;
use crate::naming::to_snake_case;
use crate::write_if_changed;

pub const PLUGINS_DIR: &str = "plugins";
pub const INDEX_FILE: &str = "bindings_types.json";

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct TypeIndex {
    pub classes: BTreeMap<String, IndexedClass>,
    pub structs: BTreeMap<String, IndexedType>,
    pub enums: BTreeMap<String, IndexedType>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IndexedClass {
    pub module: String,
    pub feature: Option<String>,
    pub parent: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IndexedType {
    pub module: String,
    pub feature: Option<String>,
}

impl TypeIndex {
    pub fn from_context(ctx: &CodegenContext) -> Self {
        let mut index = TypeIndex::default();
        let feature = |module: &str| ctx.feature_for_module(module).map(str::to_string);

        for (module, classes) in &ctx.module_classes {
            for class in classes {
                index.classes.insert(
                    class.name.clone(),
                    IndexedClass {
                        module: module.clone(),
                        feature: feature(module),
                        parent: class.super_class.clone(),
                    },
                );
            }
        }

        for (module, structs) in &ctx.module_structs {
            for s in structs {
                index.structs.insert(
                    format!("F{}", s.name),
                    IndexedType {
                        module: module.clone(),
                        feature: feature(module),
                    },
                );
            }
        }

        for (module, enums) in &ctx.module_enums {
            for e in enums {
                index.enums.insert(
                    e.name.clone(),
                    IndexedType {
                        module: module.clone(),
                        feature: feature(module),
                    },
                );
            }
        }

        index
    }
}

#[derive(Debug, Clone)]
pub struct PluginSource {
    pub plugin: String,
    pub crate_name: String,
    pub src_dir: PathBuf,
}

#[derive(Debug, Default)]
pub struct PluginModel {
    pub plugin: String,
    pub classes: Vec<PluginClass>,
    pub skipped: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct PluginClass {
    pub name: String,
    pub module_path: String,
    pub parent: String,
    pub props: Vec<PluginProp>,
    pub functions: Vec<PluginFn>,
}

#[derive(Debug, Clone)]
pub struct PluginProp {
    pub ident: String,
    pub ue_name: String,
    pub ty: Type,
}

#[derive(Debug, Clone)]
pub struct PluginParam {
    pub ident: String,
    pub ue_name: String,
    pub ty: Type,
    pub out: bool,
}

#[derive(Debug, Clone)]
pub struct PluginFn {
    pub ident: String,
    pub ue_name: String,
    pub is_static: bool,
    pub params: Vec<PluginParam>,
    pub ret: Option<Type>,
}

fn attr_named<'a>(attrs: &'a [syn::Attribute], name: &str) -> Option<&'a syn::Attribute> {
    attrs.iter().find(|a| a.path().is_ident(name))
}

fn attr_metas(attr: &syn::Attribute) -> Vec<Meta> {
    match &attr.meta {
        Meta::List(_) => attr
            .parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
            .map(|metas| metas.into_iter().collect())
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

fn str_value(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Str(s),
            ..
        }) => Some(s.value()),
        _ => None,
    }
}

fn named_value(metas: &[Meta], key: &str) -> Option<String> {
    metas.iter().find_map(|meta| match meta {
        Meta::NameValue(nv) if nv.path.is_ident(key) => str_value(&nv.value),
        _ => None,
    })
}

fn has_flag(metas: &[Meta], flag: &str) -> bool {
    metas
        .iter()
        .any(|meta| matches!(meta, Meta::Path(path) if path.is_ident(flag)))
}

fn ident_string(ident: &syn::Ident) -> String {
    let text = ident.to_string();
    text.strip_prefix("r#").map(str::to_string).unwrap_or(text)
}

struct Reader {
    crate_ident: String,
    classes: Vec<PluginClass>,
    impls: Vec<(String, Vec<PluginFn>)>,
    skipped: Vec<String>,
}

impl Reader {
    fn read_file(&mut self, file: &Path, module: &[String], is_root: bool) {
        let Ok(text) = std::fs::read_to_string(file) else {
            self.skipped
                .push(format!("{}: cannot be read", file.display()));
            return;
        };

        let parsed = match syn::parse_file(&text) {
            Ok(parsed) => parsed,
            Err(e) => {
                self.skipped.push(format!("{}: {e}", file.display()));
                return;
            }
        };

        let dir = if is_root {
            file.parent().map(Path::to_path_buf).unwrap_or_default()
        } else {
            file.with_extension("")
        };

        self.read_items(&parsed.items, module, &dir);
    }

    fn read_items(&mut self, items: &[Item], module: &[String], dir: &Path) {
        for item in items {
            match item {
                Item::Mod(m) => {
                    let name = ident_string(&m.ident);
                    let mut inner = module.to_vec();
                    inner.push(name.clone());

                    if attr_named(&m.attrs, "path").is_some() {
                        self.skipped
                            .push(format!("mod {name}: a #[path] module is not read"));
                        continue;
                    }

                    match &m.content {
                        Some((_, items)) => self.read_items(items, &inner, &dir.join(&name)),
                        None => {
                            let flat = dir.join(format!("{name}.rs"));
                            let nested = dir.join(&name).join("mod.rs");

                            if flat.is_file() {
                                self.read_file(&flat, &inner, false);
                            } else if nested.is_file() {
                                self.read_file(&nested, &inner, true);
                            }
                        }
                    }
                }
                Item::Struct(s) => {
                    if let Some(attr) = attr_named(&s.attrs, "uclass") {
                        self.read_class(s, attr, module);
                    }
                }
                Item::Impl(imp) => {
                    if attr_named(&imp.attrs, "uclass_impl").is_some() && imp.trait_.is_none() {
                        self.read_impl(imp);
                    }
                }
                _ => {}
            }
        }
    }

    fn module_path(&self, module: &[String]) -> String {
        std::iter::once(self.crate_ident.as_str())
            .chain(module.iter().map(String::as_str))
            .collect::<Vec<_>>()
            .join("::")
    }

    fn read_class(&mut self, s: &syn::ItemStruct, attr: &syn::Attribute, module: &[String]) {
        let name = ident_string(&s.ident);

        let parent = attr_metas(attr).into_iter().find_map(|meta| match meta {
            Meta::NameValue(nv) if nv.path.is_ident("parent") => match nv.value {
                Expr::Path(p) => p.path.segments.last().map(|seg| ident_string(&seg.ident)),
                _ => None,
            },
            _ => None,
        });

        let Some(parent) = parent else {
            self.skipped.push(format!("{name}: no parent"));
            return;
        };

        let mut props = Vec::new();

        if let syn::Fields::Named(fields) = &s.fields {
            for field in &fields.named {
                let Some(ident) = &field.ident else { continue };
                let ident = ident_string(ident);

                let explicit = attr_named(&field.attrs, "uproperty")
                    .or_else(|| attr_named(&field.attrs, "component"))
                    .map(|a| named_value(&attr_metas(a), "name"));

                let Some(explicit) = explicit else { continue };

                let ue_name = explicit.unwrap_or_else(|| {
                    if attr_named(&field.attrs, "component").is_some() {
                        rusteal_ue_flags::to_pascal_case(&ident)
                    } else {
                        rusteal_ue_flags::to_ue_name(&ident)
                    }
                });

                let ty = if attr_named(&field.attrs, "component").is_some() {
                    let component = field.ty.clone();
                    syn::parse_quote! { UObjectRef<#component> }
                } else {
                    field.ty.clone()
                };

                props.push(PluginProp { ident, ue_name, ty });
            }
        }

        self.classes.push(PluginClass {
            name,
            module_path: self.module_path(module),
            parent,
            props,
            functions: Vec::new(),
        });
    }

    fn read_impl(&mut self, imp: &syn::ItemImpl) {
        let Type::Path(self_ty) = &*imp.self_ty else {
            return;
        };

        let Some(class) = self_ty.path.segments.last().map(|s| ident_string(&s.ident)) else {
            return;
        };

        let mut functions = Vec::new();

        for item in &imp.items {
            let ImplItem::Fn(method) = item else { continue };
            let Some(attr) = attr_named(&method.attrs, "ufunction") else {
                continue;
            };

            let metas = attr_metas(attr);
            let ident = ident_string(&method.sig.ident);

            if has_flag(&metas, "Override")
                || has_flag(&metas, "ThreadSafe")
                || method.sig.asyncness.is_some()
            {
                continue;
            }

            let ue_name = named_value(&metas, "name")
                .unwrap_or_else(|| rusteal_ue_flags::to_pascal_case(&ident));

            let is_static = !matches!(method.sig.inputs.first(), Some(FnArg::Receiver(_)));
            let mut params = Vec::new();

            for input in &method.sig.inputs {
                let FnArg::Typed(pat) = input else { continue };
                let syn::Pat::Ident(pi) = &*pat.pat else {
                    continue;
                };
                let ident = ident_string(&pi.ident);

                let (ty, out) = match &*pat.ty {
                    Type::Reference(r) if r.mutability.is_some() => ((*r.elem).clone(), true),
                    other => (other.clone(), false),
                };

                params.push(PluginParam {
                    ue_name: rusteal_ue_flags::to_ue_name(&ident),
                    ident,
                    ty,
                    out,
                });
            }

            let ret = match &method.sig.output {
                ReturnType::Default => None,
                ReturnType::Type(_, ty) => Some(result_inner(ty).unwrap_or((**ty).clone())),
            };

            let ret = ret.filter(|ty| !matches!(ty, Type::Tuple(t) if t.elems.is_empty()));

            functions.push(PluginFn {
                ident,
                ue_name,
                is_static,
                params,
                ret,
            });
        }

        self.impls.push((class, functions));
    }
}

fn result_inner(ty: &Type) -> Option<Type> {
    let Type::Path(path) = ty else { return None };
    let last = path.path.segments.last()?;

    if last.ident != "RustealResult" {
        return None;
    }

    let PathArguments::AngleBracketed(args) = &last.arguments else {
        return None;
    };

    match args.args.first()? {
        GenericArgument::Type(inner) => Some(inner.clone()),
        _ => None,
    }
}

pub fn read_plugin(source: &PluginSource) -> PluginModel {
    let mut reader = Reader {
        crate_ident: source.crate_name.replace('-', "_"),
        classes: Vec::new(),
        impls: Vec::new(),
        skipped: Vec::new(),
    };

    reader.read_file(&source.src_dir.join("lib.rs"), &[], true);

    let mut classes = reader.classes;

    for (class, functions) in reader.impls {
        if let Some(target) = classes.iter_mut().find(|c| c.name == class) {
            target.functions.extend(functions);
        }
    }

    PluginModel {
        plugin: source.plugin.clone(),
        classes,
        skipped: reader.skipped,
    }
}

struct Resolved {
    text: String,
    features: BTreeSet<String>,
    kind: ValueKind,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ValueKind {
    Dynamic,
    Enum,
}

struct Resolver<'a> {
    index: &'a TypeIndex,
    local: &'a HashMap<String, String>,
}

const SCALARS: &[&str] = &[
    "bool", "i8", "i16", "i32", "i64", "u8", "u16", "u32", "u64", "f32", "f64",
];

impl Resolver<'_> {
    fn class_path(&self, name: &str, features: &mut BTreeSet<String>) -> Option<String> {
        if self.local.contains_key(name) {
            return Some(name.to_string());
        }

        let class = self.index.classes.get(name)?;
        features.extend(class.feature.clone());
        Some(format!("crate::{}::{name}", class.module))
    }

    fn named(&self, name: &str, features: &mut BTreeSet<String>) -> Option<(String, ValueKind)> {
        if let Some(path) = self.class_path(name, features) {
            return Some((path, ValueKind::Dynamic));
        }

        if let Some(s) = self.index.structs.get(name) {
            features.extend(s.feature.clone());
            return Some((format!("crate::{}::{name}", s.module), ValueKind::Dynamic));
        }

        if let Some(e) = self.index.enums.get(name) {
            features.extend(e.feature.clone());
            return Some((format!("crate::{}::{name}", e.module), ValueKind::Enum));
        }

        None
    }

    fn args(
        &self,
        segment: &syn::PathSegment,
        features: &mut BTreeSet<String>,
    ) -> Option<Vec<String>> {
        let PathArguments::AngleBracketed(args) = &segment.arguments else {
            return Some(Vec::new());
        };

        args.args
            .iter()
            .map(|arg| match arg {
                GenericArgument::Type(ty) => {
                    let resolved = self.resolve(ty)?;
                    features.extend(resolved.features);
                    Some(resolved.text)
                }
                _ => None,
            })
            .collect()
    }

    fn resolve(&self, ty: &Type) -> Option<Resolved> {
        let Type::Path(path) = ty else { return None };
        let segment = path.path.segments.last()?;
        let ident = ident_string(&segment.ident);
        let mut features = BTreeSet::new();

        let wrap = |name: &str, args: Vec<String>| format!("{name}<{}>", args.join(", "));

        let (text, kind) = match ident.as_str() {
            scalar if SCALARS.contains(&scalar) => (scalar.to_string(), ValueKind::Dynamic),
            "String" => ("String".to_string(), ValueKind::Dynamic),
            "FName" | "Text" => (format!("rusteal_core::{ident}"), ValueKind::Dynamic),
            "UObjectRef" | "SubclassOf" | "SoftObjectRef" | "SoftClassRef" | "TWeakObjectPtr" => {
                let args = self.args(segment, &mut features)?;
                let inner = args.first()?;

                if inner.starts_with("crate::") || self.local.contains_key(inner) {
                    (
                        wrap(&format!("rusteal_core::{ident}"), args),
                        ValueKind::Dynamic,
                    )
                } else {
                    return None;
                }
            }
            "OwnedStruct" => (
                wrap(
                    "rusteal_core::OwnedStruct",
                    self.args(segment, &mut features)?,
                ),
                ValueKind::Dynamic,
            ),
            "UeArray" | "Vec" => (
                wrap("Vec", self.args(segment, &mut features)?),
                ValueKind::Dynamic,
            ),
            "UeSet" | "HashSet" => (
                wrap(
                    "std::collections::HashSet",
                    self.args(segment, &mut features)?,
                ),
                ValueKind::Dynamic,
            ),
            "UeMap" | "HashMap" => (
                wrap(
                    "std::collections::HashMap",
                    self.args(segment, &mut features)?,
                ),
                ValueKind::Dynamic,
            ),
            other => self.named(other, &mut features)?,
        };

        Some(Resolved {
            text,
            features,
            kind,
        })
    }
}

fn cfg(features: &BTreeSet<String>) -> String {
    match features.len() {
        0 => String::new(),
        1 => format!(
            "#[cfg(feature = \"{}\")]\n",
            features.iter().next().unwrap()
        ),
        _ => format!(
            "#[cfg(all({}))]\n",
            features
                .iter()
                .map(|f| format!("feature = \"{f}\""))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

fn indent(text: &str, by: &str) -> String {
    text.lines()
        .map(|line| {
            if line.is_empty() {
                String::new()
            } else {
                format!("{by}{line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

fn ancestors(
    class: &PluginClass,
    model: &PluginModel,
    index: &TypeIndex,
) -> Option<Vec<(String, Option<String>)>> {
    let mut chain = Vec::new();
    let mut current = class.parent.clone();

    for _ in 0..64 {
        if let Some(local) = model.classes.iter().find(|c| c.name == current) {
            chain.push((local.name.clone(), None));
            current = local.parent.clone();
            continue;
        }

        let indexed = index.classes.get(&current)?;
        chain.push((
            format!("crate::{}::{current}", indexed.module),
            indexed.feature.clone(),
        ));

        match &indexed.parent {
            Some(parent) => current = parent.clone(),
            None => return Some(chain),
        }
    }

    None
}

fn generate_function(class: &PluginClass, f: &PluginFn, resolver: &Resolver) -> Option<String> {
    let mut features = BTreeSet::new();
    let mut params = Vec::new();
    let mut sets = Vec::new();
    let mut reads = Vec::new();

    for p in &f.params {
        let resolved = resolver.resolve(&p.ty)?;
        features.extend(resolved.features.iter().cloned());
        let ident = crate::naming::escape_reserved(&to_snake_case(&p.ident));

        if p.out {
            params.push(format!("{ident}: &mut {}", resolved.text));
            reads.push(match resolved.kind {
                ValueKind::Dynamic => format!("*{ident} = __result.value({:?})?;", p.ue_name),
                ValueKind::Enum => format!("*{ident} = __result.get({:?})?;", p.ue_name),
            });
        } else {
            params.push(format!("{ident}: {}", resolved.text));
            sets.push(match resolved.kind {
                ValueKind::Dynamic => format!("__call.set_value({:?}, &{ident})?;", p.ue_name),
                ValueKind::Enum => format!("__call.set({:?}, {ident})?;", p.ue_name),
            });
        }
    }

    let (ret_ty, ret_expr) = match &f.ret {
        None => ("()".to_string(), "Ok(())".to_string()),
        Some(ty) => {
            let resolved = resolver.resolve(ty)?;
            features.extend(resolved.features.iter().cloned());
            let expr = match resolved.kind {
                ValueKind::Dynamic => "__result.value(\"ReturnValue\")",
                ValueKind::Enum => "__result.get(\"ReturnValue\")",
            };
            (resolved.text, expr.to_string())
        }
    };

    let receiver = if f.is_static { "" } else { "&self, " };
    let target = if f.is_static {
        format!(
            "let __object = rusteal_core::reflected::default_object(<{} as rusteal_core::UeClass>::static_class())?;\n",
            class.name
        )
    } else {
        "let __object = self.handle();\n".to_string()
    };

    let ident = crate::naming::escape_reserved(&f.ident);
    let signature = format!(
        "fn {ident}({receiver}{}) -> rusteal_core::RustealResult<{ret_ty}>",
        params.join(", ")
    )
    .replace("(&self, )", "(&self)");

    let mut body = target;
    writeln!(
        body,
        "let __target = unsafe {{ rusteal_core::UObjectRef::<{}>::from_raw(__object) }};",
        class.name
    )
    .unwrap();
    let mutability = if sets.is_empty() { "" } else { "mut " };
    writeln!(
        body,
        "let {mutability}__call = rusteal_core::DynamicCall::new(&__target, {:?})?;",
        f.ue_name
    )
    .unwrap();

    for set in &sets {
        writeln!(body, "{set}").unwrap();
    }

    let result_binding = if reads.is_empty() && f.ret.is_none() {
        "let _ = __call.call()?;".to_string()
    } else {
        "let __result = __call.call()?;".to_string()
    };
    writeln!(body, "{result_binding}").unwrap();

    for read in &reads {
        writeln!(body, "{read}").unwrap();
    }

    writeln!(body, "{ret_expr}").unwrap();

    let visibility = if f.is_static { "pub " } else { "" };

    Some(format!(
        "{}{visibility}{signature} {{\n{}}}\n",
        cfg(&features),
        indent(&body, "    ")
    ))
}

fn generate_property(class: &PluginClass, p: &PluginProp, resolver: &Resolver) -> Option<String> {
    let resolved = resolver.resolve(&p.ty)?;
    let ident = crate::naming::escape_reserved(&p.ident);
    let setter = format!("set_{}", p.ident);
    let class_expr = format!("<{} as rusteal_core::UeClass>::static_class()", class.name);
    let slot = format!(
        "static __SLOT: rusteal_core::reflected::PropertySlot = rusteal_core::reflected::PropertySlot::new({:?});",
        p.ue_name
    );

    let (getter_ty, get, set) = match resolved.kind {
        ValueKind::Dynamic => (
            resolved.text.clone(),
            format!("__SLOT.get(self.handle(), {class_expr})"),
            format!("__SLOT.set(self.handle(), {class_expr}, &value)"),
        ),
        ValueKind::Enum => (
            format!("Option<{}>", resolved.text),
            format!("__SLOT.get_copy(self.handle(), {class_expr})"),
            format!("__SLOT.set_copy(self.handle(), {class_expr}, value)"),
        ),
    };

    let cfg = cfg(&resolved.features);

    Some(format!(
        "{cfg}#[track_caller]\nfn {ident}(&self) -> {getter_ty} {{\n    {slot}\n    {get}\n}}\n\n{cfg}#[track_caller]\nfn {setter}(&self, value: {}) {{\n    {slot}\n    {set}\n}}\n",
        resolved.text
    ))
}

pub fn generate_plugin_module(model: &PluginModel, index: &TypeIndex) -> String {
    let local: HashMap<String, String> = model
        .classes
        .iter()
        .map(|c| (c.name.clone(), c.module_path.clone()))
        .collect();
    let resolver = Resolver {
        index,
        local: &local,
    };

    let mut out = format!(
        "// Auto-generated by rusteal-codegen from the Rust sources of the {} plugin. Do not edit.\n\n\
         use rusteal_core::{{UeClass, ValidHandle}};\n",
        model.plugin
    );

    for class in &model.classes {
        let Some(chain) = ancestors(class, model, index) else {
            continue;
        };

        let chain_features: BTreeSet<String> =
            chain.iter().filter_map(|(_, f)| f.clone()).collect();
        let class_cfg = cfg(&chain_features);
        let name = &class.name;
        let path = format!("/Script/{}.{name}", model.plugin);

        write!(
            out,
            "\n/// The `{name}` class of the {} plugin, a Rust class read from its sources.\n\
             {class_cfg}pub struct {name};\n\n\
             {class_cfg}impl rusteal_core::UeClass for {name} {{\n    \
             const RUST_TYPE_ID: u64 = rusteal_core::type_id({:?});\n\n    \
             fn static_class() -> rusteal_core::UClassHandle {{\n        \
             rusteal_core::reflected::class_at({path:?})\n    }}\n}}\n\n\
             {class_cfg}impl rusteal_core::HasParent for {name} {{\n    type Parent = {};\n}}\n\n\
             {class_cfg}impl rusteal_core::Inherits<{name}> for {name} {{}}\n",
            model.plugin,
            format!("{}::{name}", class.module_path),
            chain[0].0
        )
        .unwrap();

        for (ancestor, _) in &chain {
            writeln!(
                out,
                "{class_cfg}impl rusteal_core::Inherits<{ancestor}> for {name} {{}}"
            )
            .unwrap();
        }

        let mut members = Vec::new();

        for prop in &class.props {
            if let Some(code) = generate_property(class, prop, &resolver) {
                members.push(code);
            }
        }

        let mut statics = Vec::new();

        for f in &class.functions {
            if let Some(code) = generate_function(class, f, &resolver) {
                if f.is_static {
                    statics.push(code);
                } else {
                    members.push(code);
                }
            }
        }

        write!(
            out,
            "\n{class_cfg}pub trait {name}Ext: rusteal_core::ValidHandle {{\n{}}}\n\n\
             {class_cfg}impl {name}Ext for rusteal_core::Checked<{name}> {{}}\n\
             {class_cfg}impl {name}Ext for rusteal_core::Pinned<{name}> {{}}\n",
            members
                .iter()
                .map(|m| indent(m, "    "))
                .collect::<Vec<_>>()
                .join("\n")
        )
        .unwrap();

        if !statics.is_empty() {
            write!(
                out,
                "\n{class_cfg}impl {name} {{\n{}}}\n",
                statics
                    .iter()
                    .map(|m| indent(m, "    "))
                    .collect::<Vec<_>>()
                    .join("\n")
            )
            .unwrap();
        }
    }

    out
}

pub fn plugin_module_name(plugin: &str) -> String {
    to_snake_case(plugin)
}

pub fn write_plugin_bindings(
    rust_src: &Path,
    sources: &[PluginSource],
    index: &TypeIndex,
) -> Vec<PluginModel> {
    let dir = rust_src.join(PLUGINS_DIR);
    std::fs::create_dir_all(&dir)
        .unwrap_or_else(|e| panic!("Failed to create {}: {e}", dir.display()));

    let mut models = Vec::new();
    let mut mod_rs = String::from("// Auto-generated by rusteal-codegen. Do not edit.\n");
    let mut written = BTreeSet::new();

    for source in sources {
        let model = read_plugin(source);
        let module = plugin_module_name(&source.plugin);
        let file = format!("{module}.rs");

        write_if_changed(&dir.join(&file), &generate_plugin_module(&model, index))
            .unwrap_or_else(|e| panic!("Failed to write plugins/{file}: {e}"));

        writeln!(mod_rs, "\npub mod {module};").unwrap();
        written.insert(file);
        models.push(model);
    }

    write_if_changed(&dir.join("mod.rs"), &mod_rs).expect("Failed to write plugins/mod.rs");
    written.insert("mod.rs".to_string());

    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();

            if !written.contains(&name) && name.ends_with(".rs") {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }

    models
}

pub fn write_index(path: &Path, index: &TypeIndex) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let text = serde_json::to_string(index).expect("the type index serializes");
    write_if_changed(path, &text)
        .unwrap_or_else(|e| panic!("Failed to write {}: {e}", path.display()));
}

pub fn read_index(path: &Path) -> Option<TypeIndex> {
    serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn index() -> TypeIndex {
        let mut index = TypeIndex::default();
        index.classes.insert(
            "Object".into(),
            IndexedClass {
                module: "core_ue".into(),
                feature: Some("core".into()),
                parent: None,
            },
        );
        index.classes.insert(
            "Actor".into(),
            IndexedClass {
                module: "engine".into(),
                feature: Some("engine".into()),
                parent: Some("Object".into()),
            },
        );
        index.structs.insert(
            "FVector".into(),
            IndexedType {
                module: "core_ue".into(),
                feature: Some("core".into()),
            },
        );
        index.enums.insert(
            "EMovementMode".into(),
            IndexedType {
                module: "engine".into(),
                feature: Some("engine".into()),
            },
        );
        index
    }

    fn plugin(files: &[(&str, &str)]) -> (tempdir::Dir, PluginSource) {
        let dir = tempdir::Dir::new();

        for (name, text) in files {
            let path = dir.path.join(name);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        }

        let source = PluginSource {
            plugin: "RustealProbe".into(),
            crate_name: "rusteal-probe".into(),
            src_dir: dir.path.clone(),
        };

        (dir, source)
    }

    mod tempdir {
        pub struct Dir {
            pub path: std::path::PathBuf,
        }

        impl Dir {
            pub fn new() -> Self {
                static NEXT: std::sync::atomic::AtomicUsize =
                    std::sync::atomic::AtomicUsize::new(0);
                let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                let path = std::env::temp_dir().join(format!(
                    "rusteal-plugin-bindings-{}-{n}",
                    std::process::id()
                ));
                let _ = std::fs::remove_dir_all(&path);
                std::fs::create_dir_all(&path).unwrap();
                Dir { path }
            }
        }

        impl Drop for Dir {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.path);
            }
        }
    }

    const PROBE: &str = r#"
use bindings::engine::{Actor, EMovementMode};

#[uclass(parent = Actor)]
pub struct ProbeTarget {
    #[uproperty(BlueprintReadWrite)]
    charge: i32,
    #[uproperty(name = "DisplayName")]
    label: String,
    #[uproperty]
    b_armed: bool,
    #[uproperty]
    mode: EMovementMode,
    #[uproperty]
    friend: UObjectRef<ProbeTarget>,
    secret: i32,
}

#[uclass_impl]
impl ProbeTarget {
    #[ufunction]
    pub fn add_charge(&mut self, amount: i32) -> i32 { 0 }
    #[ufunction]
    pub fn split(&self, full: String, first: &mut String) -> RustealResult<bool> { Ok(true) }
    #[ufunction]
    pub fn make() -> OwnedStruct<FVector> { todo!() }
    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {}
    #[ufunction]
    pub fn uses_unknown(&self, thing: Gadget) {}
    fn plain(&self) {}
}
"#;

    #[test]
    fn reads_classes_through_modules_with_their_paths() {
        let (_dir, source) = plugin(&[
            ("lib.rs", "mod probe;\nmod nested;\n"),
            ("probe.rs", PROBE),
            ("nested/mod.rs", "pub mod deep;\n"),
            (
                "nested/deep.rs",
                "#[uclass(parent = ProbeTarget)]\npub struct DeepProbe {}\n",
            ),
        ]);

        let model = read_plugin(&source);
        let names: Vec<_> = model
            .classes
            .iter()
            .map(|c| (c.name.as_str(), c.module_path.as_str()))
            .collect();

        assert_eq!(
            names,
            vec![
                ("ProbeTarget", "rusteal_probe::probe"),
                ("DeepProbe", "rusteal_probe::nested::deep")
            ]
        );

        let probe = &model.classes[0];
        let props: Vec<_> = probe.props.iter().map(|p| p.ue_name.as_str()).collect();
        assert_eq!(
            props,
            vec!["Charge", "DisplayName", "bArmed", "Mode", "Friend"]
        );

        let functions: Vec<_> = probe.functions.iter().map(|f| f.ue_name.as_str()).collect();
        assert_eq!(functions, vec!["AddCharge", "Split", "Make", "UsesUnknown"]);
        assert!(probe.functions[2].is_static);
        assert!(probe.functions[1].params[1].out);
    }

    #[test]
    fn generates_classes_with_their_chain_properties_and_calls() {
        let (_dir, source) = plugin(&[("lib.rs", "mod probe;\n"), ("probe.rs", PROBE)]);

        let code = generate_plugin_module(&read_plugin(&source), &index());

        assert!(code.contains("pub struct ProbeTarget;"));
        assert!(code.contains("rusteal_core::type_id(\"rusteal_probe::probe::ProbeTarget\")"));
        assert!(
            code.contains(
                "rusteal_core::reflected::class_at(\"/Script/RustealProbe.ProbeTarget\")"
            )
        );
        assert!(code.contains("type Parent = crate::engine::Actor;"));
        assert!(
            code.contains("impl rusteal_core::Inherits<crate::core_ue::Object> for ProbeTarget {}")
        );
        assert!(code.contains("fn charge(&self) -> i32 {"));
        assert!(code.contains("PropertySlot::new(\"DisplayName\")"));
        assert!(code.contains("fn mode(&self) -> Option<crate::engine::EMovementMode> {"));
        assert!(code.contains("fn friend(&self) -> rusteal_core::UObjectRef<ProbeTarget> {"));
        assert!(
            code.contains(
                "fn add_charge(&self, amount: i32) -> rusteal_core::RustealResult<i32> {"
            )
        );
        assert!(code.contains("fn split(&self, full: String, first: &mut String) -> rusteal_core::RustealResult<bool> {"));
        assert!(code.contains("*first = __result.value(\"First\")?;"));
        assert!(code.contains("pub fn make() -> rusteal_core::RustealResult<rusteal_core::OwnedStruct<crate::core_ue::FVector>> {"));
        assert!(!code.contains("secret"));
        assert!(!code.contains("receive_begin_play"));
        assert!(!code.contains("uses_unknown"));
        assert!(!code.contains("fn plain"));
    }
}
