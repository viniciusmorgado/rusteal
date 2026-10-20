use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::ext::IdentExt;
use syn::punctuated::Punctuated;
use syn::{Expr, Fields, Ident, ItemStruct, Meta, Token, parse2};

use crate::prop_type;
use crate::shape;

struct UClassArgs {
    parent_path: syn::Path,
    parent_name: String,
    implements: Vec<String>,
    rust_interfaces: Vec<syn::Path>,
    config: Option<String>,
}

fn parse_uclass_args(attr: TokenStream) -> syn::Result<UClassArgs> {
    let metas: Punctuated<Meta, Token![,]> = parse2::<syn::parse::Nothing>(attr.clone())
        .map(|_| Punctuated::new())
        .unwrap_or_else(|_| {
            syn::parse::Parser::parse2(Punctuated::<Meta, Token![,]>::parse_terminated, attr)
                .unwrap_or_default()
        });

    let mut parent_path: Option<syn::Path> = None;
    let mut implements = Vec::new();
    let mut rust_interfaces = Vec::new();
    let mut config = None;

    for meta in &metas {
        if let Meta::NameValue(nv) = meta
            && nv.path.is_ident("config")
        {
            config = Some(str_value(&nv.value, "config")?);
            continue;
        }

        if let Meta::NameValue(nv) = meta
            && nv.path.is_ident("implements")
        {
            let paths = match &nv.value {
                Expr::Array(array) => array.elems.iter().collect::<Vec<_>>(),
                other => vec![other],
            };

            for path in paths {
                match path {
                    Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(s),
                        ..
                    }) if s.value().starts_with('/') => {
                        implements.push(s.value());
                    }
                    Expr::Path(expr_path) if expr_path.qself.is_none() => {
                        rust_interfaces.push(expr_path.path.clone());
                    }
                    other => {
                        return Err(syn::Error::new_spanned(
                            other,
                            "`implements` lists #[uinterface] traits and UE interface class paths: \
                             implements = [Interactable, \"/Script/Module.Interface\", \"/Game/Path/BPI_Foo.BPI_Foo_C\"]",
                        ));
                    }
                }
            }

            continue;
        }

        if let Meta::NameValue(nv) = meta
            && nv.path.is_ident("parent")
        {
            if let Expr::Path(expr_path) = &nv.value {
                parent_path = Some(expr_path.path.clone());
            } else {
                return Err(syn::Error::new_spanned(
                    &nv.value,
                    "`parent` must be a type path, not a string literal.\n\n\
                     Example: #[uclass(parent = Actor)]",
                ));
            }
        }
    }

    let parent_path = parent_path.ok_or_else(|| {
        syn::Error::new(
            proc_macro2::Span::call_site(),
            "#[uclass] requires a `parent` attribute specifying the UE parent class.\n\n\
             Example:\n\
             \x20   #[uclass(parent = Actor)]\n\
             \x20   pub struct MyActor { ... }\n\n\
             Common parents: Actor, Pawn, Character, PlayerController,\n\
             \x20               GameModeBase, ActorComponent, SceneComponent",
        )
    })?;

    let parent_name = parent_path
        .segments
        .last()
        .map(|s| s.ident.to_string())
        .unwrap_or_default();

    Ok(UClassArgs {
        parent_path,
        parent_name,
        implements,
        rust_interfaces,
        config,
    })
}

#[derive(Default)]
pub(crate) struct UPropertyArgs {
    blueprint_read_write: bool,
    blueprint_read_only: bool,
    edit_anywhere: bool,
    edit_defaults_only: bool,
    visible_anywhere: bool,
    save_game: bool,
    config: bool,
    transient: bool,
    pub(crate) default_expr: Option<Expr>,
    name: Option<String>,
    category: Option<String>,
    bind_widget: Option<&'static str>,
    meta: Vec<(String, String)>,
}

fn meta_value(value: &Expr) -> syn::Result<String> {
    match value {
        Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Str(s),
            ..
        }) => Ok(s.value()),
        Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Bool(b),
            ..
        }) => Ok(b.value.to_string()),
        Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Int(i),
            ..
        }) => Ok(i.base10_digits().to_string()),
        Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Float(f),
            ..
        }) => Ok(f.base10_digits().to_string()),
        Expr::Unary(syn::ExprUnary {
            op: syn::UnOp::Neg(_),
            expr,
            ..
        }) => Ok(format!("-{}", meta_value(expr)?)),
        other => Err(syn::Error::new_spanned(
            other,
            "a meta value is a string, a number or a bool",
        )),
    }
}

pub(crate) fn parse_meta_list(list: &syn::MetaList) -> syn::Result<Vec<(String, String)>> {
    let entries = list.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;
    let mut out = Vec::new();

    for entry in &entries {
        match entry {
            Meta::Path(p) => {
                let key = p.get_ident().ok_or_else(|| {
                    syn::Error::new_spanned(p, "a meta key is a plain name, such as ClampMin")
                })?;

                out.push((key.to_string(), "true".to_string()));
            }
            Meta::NameValue(nv) => {
                let key = nv.path.get_ident().ok_or_else(|| {
                    syn::Error::new_spanned(
                        &nv.path,
                        "a meta key is a plain name, such as ClampMin",
                    )
                })?;

                out.push((key.to_string(), meta_value(&nv.value)?));
            }
            Meta::List(l) => {
                return Err(syn::Error::new_spanned(
                    l,
                    "a meta entry is Key = value, or a Key alone for true",
                ));
            }
        }
    }

    Ok(out)
}

pub(crate) fn doc_tooltip(attrs: &[syn::Attribute]) -> Option<String> {
    let lines: Vec<String> = attrs
        .iter()
        .filter(|a| a.path().is_ident("doc"))
        .filter_map(|a| match &a.meta {
            Meta::NameValue(nv) => match &nv.value {
                Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(s),
                    ..
                }) => Some(s.value().trim().to_string()),
                _ => None,
            },
            _ => None,
        })
        .collect();

    let text = lines.join("\n").trim().to_string();
    (!text.is_empty()).then_some(text)
}

impl UPropertyArgs {
    pub(crate) fn add_doc_tooltip(&mut self, attrs: &[syn::Attribute]) {
        if self.meta.iter().any(|(key, _)| key == "ToolTip") {
            return;
        }

        if let Some(tooltip) = doc_tooltip(attrs) {
            self.meta.push(("ToolTip".to_string(), tooltip));
        }
    }
}

fn str_value(value: &Expr, what: &str) -> syn::Result<String> {
    match value {
        Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Str(s),
            ..
        }) => Ok(s.value()),
        other => Err(syn::Error::new_spanned(
            other,
            format!("{what} is a string literal"),
        )),
    }
}

pub(crate) fn parse_uproperty_args(attr: &syn::Attribute) -> syn::Result<UPropertyArgs> {
    let mut args = UPropertyArgs::default();

    if let Meta::Path(_) = attr.meta {
        return Ok(args);
    }

    let nested = attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;

    for meta in &nested {
        match meta {
            Meta::Path(p) => {
                if p.is_ident("BlueprintReadWrite") {
                    args.blueprint_read_write = true;
                } else if p.is_ident("BlueprintReadOnly") {
                    args.blueprint_read_only = true;
                } else if p.is_ident("EditAnywhere") {
                    args.edit_anywhere = true;
                } else if p.is_ident("EditDefaultsOnly") {
                    args.edit_defaults_only = true;
                } else if p.is_ident("VisibleAnywhere") {
                    args.visible_anywhere = true;
                } else if p.is_ident("SaveGame") {
                    args.save_game = true;
                } else if p.is_ident("Config") {
                    args.config = true;
                } else if p.is_ident("Transient") {
                    args.transient = true;
                } else if p.is_ident("BindWidget") {
                    args.bind_widget = Some("BindWidget");
                } else if p.is_ident("BindWidgetOptional") {
                    args.bind_widget = Some("BindWidgetOptional");
                } else if p.is_ident("BindWidgetAnim") {
                    args.bind_widget = Some("BindWidgetAnim");
                    args.transient = true;
                } else if p.is_ident("BindWidgetAnimOptional") {
                    args.bind_widget = Some("BindWidgetAnimOptional");
                    args.transient = true;
                } else {
                    return Err(syn::Error::new_spanned(
                        p,
                        "unknown #[uproperty] argument: expected BlueprintReadWrite, \
                         BlueprintReadOnly, EditAnywhere, EditDefaultsOnly, VisibleAnywhere, \
                         SaveGame, Config, Transient, BindWidget, BindWidgetOptional, \
                         BindWidgetAnim, BindWidgetAnimOptional, default = ..., \
                         name = \"...\", category = \"...\" or meta(...)",
                    ));
                }
            }
            Meta::NameValue(nv) if nv.path.is_ident("default") => {
                args.default_expr = Some(nv.value.clone());
            }
            Meta::NameValue(nv) if nv.path.is_ident("name") => {
                args.name = Some(str_value(&nv.value, "name")?);
            }
            Meta::NameValue(nv) if nv.path.is_ident("category") => {
                args.category = Some(str_value(&nv.value, "category")?);
            }
            Meta::List(list) if list.path.is_ident("meta") => {
                args.meta.extend(parse_meta_list(list)?);
            }
            other => {
                return Err(syn::Error::new_spanned(
                    other,
                    "unknown #[uproperty] argument",
                ));
            }
        }
    }

    Ok(args)
}

struct ComponentArgs {
    is_root: bool,
    attach_to: Option<String>,
    socket: Option<String>,
    name: Option<String>,
}

fn parse_component_args(attr: &syn::Attribute) -> syn::Result<ComponentArgs> {
    let mut args = ComponentArgs {
        is_root: false,
        attach_to: None,
        socket: None,
        name: None,
    };

    let nested = match attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated) {
        Ok(n) => n,
        Err(_) => return Ok(args),
    };

    for meta in &nested {
        match meta {
            Meta::Path(p) => {
                if p.is_ident("root") {
                    args.is_root = true;
                }
            }
            Meta::NameValue(nv) if nv.path.is_ident("attach") => {
                if let Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(s),
                    ..
                }) = &nv.value
                {
                    args.attach_to = Some(s.value());
                } else {
                    return Err(syn::Error::new_spanned(
                        &nv.value,
                        "attach must be a string literal, e.g. attach = \"root_scene\"",
                    ));
                }
            }
            Meta::NameValue(nv) if nv.path.is_ident("socket") => {
                if let Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(s),
                    ..
                }) = &nv.value
                {
                    args.socket = Some(s.value());
                } else {
                    return Err(syn::Error::new_spanned(
                        &nv.value,
                        "socket must be a string literal, e.g. socket = \"SpringEndpoint\"",
                    ));
                }
            }
            Meta::NameValue(nv) if nv.path.is_ident("name") => {
                args.name = Some(str_value(&nv.value, "name")?);
            }
            _ => {}
        }
    }

    if args.socket.is_some() && args.attach_to.is_none() {
        return Err(syn::Error::new_spanned(
            attr,
            "socket needs attach, e.g. attach = \"camera_boom\", socket = \"SpringEndpoint\"",
        ));
    }

    Ok(args)
}

pub(crate) struct UPropertyField {
    pub ident: Ident,
    pub ty: syn::Type,
    pub args: UPropertyArgs,
}

impl UPropertyField {
    fn ue_name(&self) -> String {
        self.args
            .name
            .clone()
            .unwrap_or_else(|| prop_type::to_ue_name(&self.ident.unraw().to_string()))
    }
}

struct RustPrivateField {
    ident: Ident,
    ty: syn::Type,
}

struct ComponentField {
    ident: Ident,
    component_type: syn::Path,
    is_root: bool,
    attach_to: Option<String>,
    socket: Option<String>,
    subobject_name: String,
}

pub fn expand_uclass(attr: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let attr_shape = format!("uclass({attr})");
    let args = parse_uclass_args(attr)?;
    let input: ItemStruct = parse2(item)?;

    let struct_name = &input.ident;
    let struct_vis = &input.vis;
    let struct_name_str = struct_name.to_string();

    let mut uprops: Vec<UPropertyField> = Vec::new();
    let mut rust_fields: Vec<RustPrivateField> = Vec::new();
    let mut components: Vec<ComponentField> = Vec::new();

    let fields = match &input.fields {
        Fields::Named(f) => &f.named,
        _ => {
            return Err(syn::Error::new_spanned(
                &input,
                "#[uclass] requires a struct with named fields.\n\n\
                 Example:\n\
                 \x20   #[uclass(parent = Actor)]\n\
                 \x20   pub struct MyActor {\n\
                 \x20       #[uproperty(BlueprintReadWrite, EditAnywhere)]\n\
                 \x20       health: f32,\n\
                 \x20   }",
            ));
        }
    };

    for field in fields {
        let field_ident = field.ident.as_ref().unwrap().clone();
        let field_ty = field.ty.clone();

        let comp_attr = field.attrs.iter().find(|a| a.path().is_ident("component"));

        let uprop_attr = field.attrs.iter().find(|a| a.path().is_ident("uproperty"));

        if let Some(attr) = comp_attr {
            let cargs = parse_component_args(attr)?;

            let component_type = match &field_ty {
                syn::Type::Path(tp) => tp.path.clone(),
                _ => {
                    return Err(syn::Error::new_spanned(
                        &field_ty,
                        "#[component] field type must be a path (e.g. SceneComponent)",
                    ));
                }
            };

            let subobject_name = cargs
                .name
                .unwrap_or_else(|| prop_type::to_pascal_case(&field_ident.unraw().to_string()));

            components.push(ComponentField {
                ident: field_ident,
                component_type,
                is_root: cargs.is_root,
                attach_to: cargs.attach_to,
                socket: cargs.socket,
                subobject_name,
            });
        } else if let Some(attr) = uprop_attr {
            let mut pargs = parse_uproperty_args(attr)?;
            pargs.add_doc_tooltip(&field.attrs);

            let Some(info) = prop_type::map_type(&field_ty) else {
                return Err(syn::Error::new_spanned(
                    &field_ty,
                    "unsupported uproperty type: supported are bool, the integers, f32, f64, \
                     String, FName, Text, UE enums, UObjectRef<T>, TWeakObjectPtr<T>, \
                     SubclassOf<T>, SoftObjectRef<T>, SoftClassRef<T>, ScriptInterface<dyn I>, \
                     OwnedStruct<T>, and UeArray, UeMap and UeSet of those",
                ));
            };

            if matches!(info.kind, prop_type::PropKind::Struct { .. }) {
                return Err(syn::Error::new_spanned(
                    &field_ty,
                    "UStructRef is only supported as a #[ufunction] parameter; a struct \
                     #[uproperty] is an OwnedStruct<T>",
                ));
            }

            if pargs.default_expr.is_some()
                && !matches!(
                    info.kind,
                    prop_type::PropKind::Scalar { .. }
                        | prop_type::PropKind::RawScalar { .. }
                        | prop_type::PropKind::Enum { .. }
                        | prop_type::PropKind::Str
                        | prop_type::PropKind::Name
                        | prop_type::PropKind::Text
                )
            {
                return Err(syn::Error::new_spanned(
                    &field_ty,
                    "`default = ...` is supported for bool, integer, float, enum, String, FName \
                     and Text properties; the others are set in #[class_defaults] or a Blueprint child",
                ));
            }

            uprops.push(UPropertyField {
                ident: field_ident,
                ty: field_ty,
                args: pargs,
            });
        } else {
            rust_fields.push(RustPrivateField {
                ident: field_ident,
                ty: field_ty,
            });
        }
    }

    let rust_data_name = format_ident!("__{}RustData", struct_name);

    let class_handle_name = format_ident!(
        "__RUSTEAL_CLASS_HANDLE_{}",
        to_screaming_snake(&struct_name_str)
    );

    let register_fn_name = format_ident!("__rusteal_register_{}", to_snake_case(&struct_name_str));
    let members_fn_name = format_ident!("__rusteal_members_{}", to_snake_case(&struct_name_str));
    let finalize_fn_name = format_ident!("__rusteal_finalize_{}", to_snake_case(&struct_name_str));

    let after_defaults_fn_name = format_ident!(
        "__rusteal_after_defaults_{}",
        to_snake_case(&struct_name_str)
    );

    let type_id_value = quote! {
        ::rusteal_runtime::runtime::type_id(concat!(module_path!(), "::", #struct_name_str))
    };

    let parent_path_for_id = &args.parent_path;

    let parent_type_id = quote! {
        <#parent_path_for_id as ::rusteal_runtime::runtime::UeClass>::RUST_TYPE_ID
    };

    let component_type_ids: Vec<TokenStream> = components
        .iter()
        .map(|c| {
            let ty = &c.component_type;
            quote! { <#ty as ::rusteal_runtime::runtime::UeClass>::RUST_TYPE_ID }
        })
        .collect();

    let shape_value = shape::hash(
        std::iter::once(attr_shape).chain(
            fields
                .iter()
                .filter_map(|f| shape::field(f, &["uproperty", "component"])),
        ),
    );

    let user_struct = quote! {
        #struct_vis struct #struct_name {
            #[doc(hidden)]
            pub __obj: ::rusteal_runtime::ffi::UObjectHandle,
            #[doc(hidden)]
            pub __rust_data: *mut #rust_data_name,
        }
    };

    let rust_data_fields: Vec<TokenStream> = rust_fields
        .iter()
        .map(|f| {
            let ident = &f.ident;
            let ty = &f.ty;
            quote! { pub #ident: #ty, }
        })
        .collect();

    let rust_data_defaults: Vec<TokenStream> = rust_fields
        .iter()
        .map(|f| {
            let ident = &f.ident;
            quote! { #ident: Default::default(), }
        })
        .collect();

    let rust_data_struct = quote! {
        #[doc(hidden)]
        pub struct #rust_data_name {
            #(#rust_data_fields)*
        }

        impl Default for #rust_data_name {
            fn default() -> Self {
                Self {
                    #(#rust_data_defaults)*
                }
            }
        }
    };

    let static_handle = quote! {
        #[doc(hidden)]
        pub static #class_handle_name: std::sync::OnceLock<::rusteal_runtime::ffi::UClassHandle> = std::sync::OnceLock::new();
    };

    let ue_class_impl = quote! {
        impl ::rusteal_runtime::runtime::UeClass for #struct_name {
            const RUST_TYPE_ID: u64 = #struct_name::__RUSTEAL_TYPE_ID;

            fn static_class() -> ::rusteal_runtime::ffi::UClassHandle {
                *#class_handle_name.get().expect(concat!("Failed to find UClass for ", stringify!(#struct_name), " — was it registered?"))
            }
        }
    };

    let mut accessor_methods: Vec<TokenStream> = Vec::new();
    let container = quote! { self.__obj };
    let owner = quote! { <Self as ::rusteal_runtime::runtime::UeClass>::static_class() };

    for accessor in property_accessors(
        &uprops,
        &container,
        &format_ident!("reflection_find_property"),
        &owner,
    ) {
        let signature = accessor.signature();
        let body = &accessor.body;

        accessor_methods.push(quote! {
            pub #signature {
                #body
            }
        });
    }

    for f in &rust_fields {
        let ident = &f.ident;
        let ty = &f.ty;
        let setter_ident = format_ident!("set_{}", ident);
        let mut_ident = format_ident!("{}_mut", ident);

        accessor_methods.push(quote! {
            #[allow(clippy::clone_on_copy)]
            pub fn #ident(&self) -> #ty {
                unsafe { (*self.__rust_data).#ident.clone() }
            }
            pub fn #setter_ident(&mut self, val: #ty) {
                unsafe { (*self.__rust_data).#ident = val; }
            }
            #[allow(dead_code)]
            pub fn #mut_ident(&mut self) -> &mut #ty {
                unsafe { &mut (*self.__rust_data).#ident }
            }
        });
    }

    for comp in &components {
        let field_ident = &comp.ident;
        let comp_type = &comp.component_type;
        let comp_name = &comp.subobject_name;
        let comp_name_bytes = comp_name.as_bytes();
        let comp_name_len = comp_name.len() as u32;

        accessor_methods.push(quote! {
            #[track_caller]
            pub fn #field_ident(&self) -> ::rusteal_runtime::runtime::RustealResult<::rusteal_runtime::runtime::UObjectRef<#comp_type>> {
                let h = unsafe {
                    ::rusteal_runtime::runtime::ffi_dispatch::reify_find_default_subobject(
                        self.__obj,
                        [#(#comp_name_bytes),*].as_ptr(), #comp_name_len,
                    )
                };

                if h.is_null() {
                    return Err(::rusteal_runtime::runtime::RustealError::invalid_operation(
                        concat!("subobject '", #comp_name, "' not found")
                    ).on_handle(self.__obj));
                }

                Ok(unsafe { ::rusteal_runtime::runtime::UObjectRef::from_raw(h) })
            }
        });
    }

    let as_ref_parent = &args.parent_path;

    accessor_methods.push(quote! {
        pub fn as_ref(&self) -> ::rusteal_runtime::runtime::UObjectRef<#as_ref_parent> {
            unsafe { ::rusteal_runtime::runtime::UObjectRef::from_raw(self.__obj) }
        }
    });

    accessor_methods.push(quote! {
        #[track_caller]
        pub fn from_obj(obj: ::rusteal_runtime::runtime::UObjectRef<impl ::rusteal_runtime::runtime::UeClass>) -> ::rusteal_runtime::runtime::RustealResult<Self> {
            let handle = obj.checked()?.raw();

            let is_a = unsafe {
                ::rusteal_runtime::runtime::ffi_dispatch::core_is_a(handle, <Self as ::rusteal_runtime::runtime::UeClass>::static_class())
            };

            if !is_a {
                return Err(::rusteal_runtime::runtime::RustealError::invalid_cast_to(
                    handle,
                    <Self as ::rusteal_runtime::runtime::UeClass>::static_class(),
                ));
            }

            let rust_data = ::rusteal_runtime::runtime::reify_registry::get_instance_data(
                handle,
                Self::__RUSTEAL_TYPE_ID,
            ) as *mut #rust_data_name;

            if rust_data.is_null() {
                return Err(::rusteal_runtime::runtime::RustealError::internal(
                    concat!("no Rust data for ", stringify!(#struct_name), ": its library was reloaded or unloaded")
                ).on_handle(handle));
            }

            Ok(Self { __obj: handle, __rust_data: rust_data })
        }
    });

    accessor_methods.push(quote! {
        #[track_caller]
        pub fn get_default() -> ::rusteal_runtime::runtime::RustealResult<Self> {
            let class = <Self as ::rusteal_runtime::runtime::UeClass>::static_class();
            let cdo = unsafe { ::rusteal_runtime::runtime::ffi_dispatch::reify_get_cdo(class) };

            Self::from_obj(unsafe { ::rusteal_runtime::runtime::UObjectRef::<Self>::from_raw(cdo) })
        }
    });

    let accessors_impl = quote! {
        impl #struct_name {
            #[doc(hidden)]
            pub const __RUSTEAL_TYPE_ID: u64 = #type_id_value;

            #(#accessor_methods)*
        }
    };

    let parent_path = &args.parent_path;
    let parent_name = args.parent_name.as_str();
    let parent_name_bytes = parent_name.as_bytes();
    let parent_name_len = parent_name.len() as u32;
    let struct_name_bytes = struct_name_str.as_bytes();
    let struct_name_byte_len = struct_name_str.len() as u32;

    let add_prop_stmts = add_property_statements(&uprops);

    let add_interface_stmts: Vec<TokenStream> = args
        .implements
        .iter()
        .map(|path| {
            let bytes = path.as_bytes();
            let len = path.len() as u32;

            quote! {
                unsafe {
                    ::rusteal_runtime::runtime::ffi_dispatch::reify_add_interface(
                        class,
                        [#(#bytes),*].as_ptr(),
                        #len,
                    );
                }
            }
        })
        .collect();

    let add_rust_interface_stmts: Vec<TokenStream> = args
        .rust_interfaces
        .iter()
        .map(|path| {
            quote! {
                {
                    let interface = <dyn #path>::__rusteal_interface_handle();

                    if !interface.is_null() {
                        unsafe {
                            ::rusteal_runtime::runtime::ffi_dispatch::reify_add_interface_class(class, interface);
                        }
                    }
                }
            }
        })
        .collect();

    let mut add_comp_stmts: Vec<TokenStream> = Vec::new();

    for comp in &components {
        let comp_type = &comp.component_type;
        let comp_name_bytes = comp.subobject_name.as_bytes();
        let comp_name_len = comp.subobject_name.len() as u32;

        let property_name = prop_type::to_pascal_case(&comp.ident.unraw().to_string());
        let property_bytes = property_name.as_bytes();
        let property_len = property_name.len() as u32;

        let mut flags: u32 = 0;

        if comp.is_root {
            flags |= 1;
        }

        let name_arg = |name: Option<String>| match name {
            Some(name) => {
                let bytes = name.into_bytes();
                let len = bytes.len() as u32;

                (quote! { [#(#bytes),*].as_ptr() }, quote! { #len })
            }
            None => (quote! { std::ptr::null() }, quote! { 0u32 }),
        };

        let (attach_ptr, attach_len) =
            name_arg(comp.attach_to.as_deref().map(prop_type::to_pascal_case));

        let (socket_ptr, socket_len) = name_arg(comp.socket.clone());

        add_comp_stmts.push(quote! {
            {
                let comp_class = <#comp_type as ::rusteal_runtime::runtime::UeClass>::static_class();

                unsafe {
                    ::rusteal_runtime::runtime::ffi_dispatch::reify_add_default_subobject(
                        class,
                        [#(#comp_name_bytes),*].as_ptr(), #comp_name_len,
                        [#(#property_bytes),*].as_ptr(), #property_len,
                        comp_class,
                        #flags,
                        #attach_ptr, #attach_len,
                        #socket_ptr, #socket_len,
                    );
                }
            }
        });
    }

    let mut finalize_cdo_stmts: Vec<TokenStream> = Vec::new();

    for prop in &uprops {
        if let Some(ref default_expr) = prop.args.default_expr {
            let set_default = match prop_type::map_type(&prop.ty).unwrap().kind {
                prop_type::PropKind::Scalar { setter_fn, .. } => {
                    let setter_dispatch = format_ident!("property_{}", setter_fn);
                    quote! { ::rusteal_runtime::runtime::ffi_dispatch::#setter_dispatch(cdo, prop, #default_expr) }
                }
                prop_type::PropKind::Enum { .. } => quote! {
                    ::rusteal_runtime::runtime::ffi_dispatch::property_set_enum(
                        cdo, prop, ::rusteal_runtime::runtime::UeEnum::to_i64(#default_expr),
                    )
                },
                prop_type::PropKind::RawScalar { .. } => {
                    let rust_ty = prop_type::map_type(&prop.ty).unwrap().rust_type;

                    quote! {
                        {
                            let value: #rust_ty = #default_expr;
                            ::rusteal_runtime::runtime::ffi_dispatch::property_set_property_at(
                                cdo, prop, 0, &value as *const #rust_ty as *const u8,
                                std::mem::size_of::<#rust_ty>() as u32,
                            )
                        }
                    }
                }
                prop_type::PropKind::Str => quote! {
                    {
                        let value: &str = #default_expr;
                        ::rusteal_runtime::runtime::ffi_dispatch::property_set_string(
                            cdo, prop, value.as_ptr(), value.len() as u32,
                        )
                    }
                },
                prop_type::PropKind::Name => quote! {
                    ::rusteal_runtime::runtime::ffi_dispatch::property_set_fname(
                        cdo, prop, ::rusteal_runtime::runtime::FName::new(#default_expr).handle(),
                    )
                },
                prop_type::PropKind::Text => quote! {
                    {
                        let value: ::rusteal_runtime::runtime::Text = (#default_expr).into();
                        ::rusteal_runtime::runtime::ffi_dispatch::property_set_text(cdo, prop, value.as_raw())
                    }
                },
                _ => continue,
            };

            let ue_name = prop.ue_name();
            let ue_name_bytes = ue_name.as_bytes();
            let ue_name_len = ue_name.len() as u32;

            finalize_cdo_stmts.push(quote! {
                {
                    let prop = unsafe {
                        ::rusteal_runtime::runtime::ffi_dispatch::reflection_find_property(
                            class,
                            [#(#ue_name_bytes),*].as_ptr(),
                            #ue_name_len,
                        )
                    };

                    if !prop.is_null() {
                        unsafe { #set_default; }
                    }
                }
            });
        }
    }

    let register_fn = quote! {
        #[doc(hidden)]
        pub fn #register_fn_name(last_try: bool, shape: u64) -> bool {
            const TYPE_ID: u64 = #struct_name::__RUSTEAL_TYPE_ID;

            let parent = unsafe {
                ::rusteal_runtime::runtime::ffi_dispatch::reflection_find_class(
                    [#(#parent_name_bytes),*].as_ptr(),
                    #parent_name_len,
                )
            };

            if parent.is_null() {
                if !last_try {
                    return false;
                }

                let msg = concat!("[Rusteal] ", stringify!(#struct_name), ": failed to find parent class '", #parent_name, "'");
                let bytes = msg.as_bytes();
                unsafe { ::rusteal_runtime::runtime::ffi_dispatch::logging_log(2, bytes.as_ptr(), bytes.len() as u32); }
                return true;
            }

            ::rusteal_runtime::runtime::reify_registry::register_type(
                TYPE_ID,
                ::rusteal_runtime::runtime::reify_registry::RustTypeInfo {
                    name: #struct_name_str,
                    construct_fn: || {
                        Box::into_raw(Box::new(#rust_data_name::default())) as *mut u8
                    },
                    drop_fn: |ptr| {
                        if !ptr.is_null() {
                            let _ = unsafe { Box::from_raw(ptr as *mut #rust_data_name) };
                        }
                    },
                },
            );

            let class = unsafe {
                ::rusteal_runtime::runtime::ffi_dispatch::reify_create_class(
                    [#(#struct_name_bytes),*].as_ptr(),
                    #struct_name_byte_len,
                    parent,
                    TYPE_ID,
                    shape,
                )
            };

            if class.is_null() {
                let msg = concat!("[Rusteal] ", stringify!(#struct_name), ": create_class failed");
                let bytes = msg.as_bytes();
                unsafe { ::rusteal_runtime::runtime::ffi_dispatch::logging_log(2, bytes.as_ptr(), bytes.len() as u32); }
                return true;
            }

            #class_handle_name.set(class).ok();

            true
        }

        #[doc(hidden)]
        pub fn #members_fn_name() {
            let class = match #class_handle_name.get() {
                Some(&c) if !c.is_null() => c,
                _ => return,
            };

            #(#add_prop_stmts)*

            #(#add_interface_stmts)*

            #(#add_rust_interface_stmts)*

            #(#add_comp_stmts)*
        }
    };

    let finalize_fn = quote! {
        #[doc(hidden)]
        pub fn #finalize_fn_name() {
            let class = match #class_handle_name.get() {
                Some(&c) if !c.is_null() => c,
                _ => return,
            };

            let result = unsafe { ::rusteal_runtime::runtime::ffi_dispatch::reify_finalize_class(class) };

            if result != ::rusteal_runtime::ffi::RustealErrorCode::Ok {
                let msg = concat!("[Rusteal] ", stringify!(#struct_name), ": finalize_class failed");
                let bytes = msg.as_bytes();
                unsafe { ::rusteal_runtime::runtime::ffi_dispatch::logging_log(2, bytes.as_ptr(), bytes.len() as u32); }
                return;
            }

            let cdo = unsafe { ::rusteal_runtime::runtime::ffi_dispatch::reify_get_cdo(class) };

            if !cdo.is_null() {
                #(#finalize_cdo_stmts)*
            }

            let msg = concat!("[Rusteal] ", stringify!(#struct_name), " registered successfully");
            let bytes = msg.as_bytes();
            unsafe { ::rusteal_runtime::runtime::ffi_dispatch::logging_log(0, bytes.as_ptr(), bytes.len() as u32); }
        }
    };

    let config_stmt = args.config.as_ref().map(|config| {
        let config_bytes = config.as_bytes();
        let config_len = config.len() as u32;

        quote! {
            let result = unsafe {
                ::rusteal_runtime::runtime::ffi_dispatch::reify_set_class_config(
                    class,
                    [#(#config_bytes),*].as_ptr(),
                    #config_len,
                )
            };
            if result != ::rusteal_runtime::ffi::RustealErrorCode::Ok {
                let msg = concat!("[Rusteal] ", stringify!(#struct_name), ": set_class_config failed");
                let bytes = msg.as_bytes();
                unsafe { ::rusteal_runtime::runtime::ffi_dispatch::logging_log(2, bytes.as_ptr(), bytes.len() as u32); }
            }
        }
    });

    let after_defaults_fn = quote! {
        #[doc(hidden)]
        pub fn #after_defaults_fn_name() {
            let Some(&class) = #class_handle_name.get() else {
                return;
            };

            if class.is_null() {
                return;
            }

            #config_stmt
        }
    };

    let comp_type_checks: Vec<TokenStream> = components
        .iter()
        .map(|c| {
            let ct = &c.component_type;
            quote! { _assert_ue_class::<#ct>(); }
        })
        .collect();

    let parent_check = quote! {
        const _: () = {
            fn _rusteal_parent_check() {
                fn _assert_ue_class<T: ::rusteal_runtime::runtime::UeClass>() {}
                _assert_ue_class::<#parent_path>();

                #(#comp_type_checks)*
            }
        };
    };

    let has_parent_impl = quote! {
        impl ::rusteal_runtime::runtime::HasParent for #struct_name {
            type Parent = #parent_path;
        }
    };

    let inherits_impl = quote! {
        impl<U: ::rusteal_runtime::runtime::UeClass> ::rusteal_runtime::runtime::Inherits<U> for #struct_name
        where
            #parent_path: ::rusteal_runtime::runtime::Inherits<U>,
        {
        }
    };

    let deref_impl = quote! {
        impl std::ops::Deref for #struct_name {
            type Target = ::rusteal_runtime::runtime::UObjectRef<#parent_path>;
            fn deref(&self) -> &::rusteal_runtime::runtime::UObjectRef<#parent_path> {
                unsafe { &*(&self.__obj as *const ::rusteal_runtime::ffi::UObjectHandle as *const ::rusteal_runtime::runtime::UObjectRef<#parent_path>) }
            }
        }
    };

    Ok(quote! {
        #user_struct
        #rust_data_struct
        #static_handle
        #ue_class_impl
        #has_parent_impl
        #inherits_impl
        #parent_check
        #deref_impl
        #accessors_impl
        #register_fn
        #finalize_fn
        #after_defaults_fn

        ::rusteal_runtime::__inventory::submit! {
            ::rusteal_runtime::runtime::reify_registry::ClassRegistration {
                krate: env!("CARGO_CRATE_NAME"),
                name: #struct_name_str,
                type_id: #struct_name::__RUSTEAL_TYPE_ID,
                shape: #shape_value,
                depends_on: &[#parent_type_id, #(#component_type_ids),*],
                create: #register_fn_name,
                register: #members_fn_name,
                finalize: #finalize_fn_name,
                after_defaults: #after_defaults_fn_name,
            }
        }
    })
}

pub(crate) fn to_snake_case(s: &str) -> String {
    let mut result = String::new();

    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('_');
            }

            result.push(c.to_lowercase().next().unwrap());
        } else {
            result.push(c);
        }
    }

    result
}

pub(crate) fn to_screaming_snake(s: &str) -> String {
    to_snake_case(s).to_uppercase()
}

pub(crate) struct Accessor {
    pub name: Ident,
    pub param: Option<TokenStream>,
    pub ret: Option<TokenStream>,
    pub body: TokenStream,
}

impl Accessor {
    pub fn signature(&self) -> TokenStream {
        let name = &self.name;
        let param = self.param.iter();
        let ret = self.ret.iter();
        quote! { fn #name(&self #(, #param)*) #(-> #ret)* }
    }
}

pub(crate) type SetterParts = Option<(TokenStream, TokenStream)>;

pub(crate) fn accessor_parts(
    kind: &prop_type::PropKind,
    rust_ty: &TokenStream,
    container: &TokenStream,
    ue_name: &str,
) -> (TokenStream, SetterParts) {
    match kind {
        prop_type::PropKind::Scalar {
            getter_fn,
            setter_fn,
            zero_expr,
        } => {
            let getter_dispatch = format_ident!("property_{}", getter_fn);
            let setter_dispatch = format_ident!("property_{}", setter_fn);

            (
                quote! {
                    let mut val: #rust_ty = #zero_expr;
                    unsafe { ::rusteal_runtime::runtime::ffi_dispatch::#getter_dispatch(#container, prop, &mut val); }
                    val
                },
                Some((
                    quote! { #rust_ty },
                    quote! {
                        unsafe { ::rusteal_runtime::runtime::ffi_dispatch::#setter_dispatch(#container, prop, val); }
                    },
                )),
            )
        }
        prop_type::PropKind::Object { .. } => (
            quote! {
                let mut h = ::rusteal_runtime::ffi::UObjectHandle::null();
                unsafe { ::rusteal_runtime::runtime::ffi_dispatch::property_get_object(#container, prop, &mut h); }
                unsafe { <#rust_ty>::from_raw(h) }
            },
            Some((
                quote! { #rust_ty },
                quote! {
                    unsafe { ::rusteal_runtime::runtime::ffi_dispatch::property_set_object(#container, prop, val.raw()); }
                },
            )),
        ),
        prop_type::PropKind::Class { .. } => (
            quote! {
                let mut h = ::rusteal_runtime::ffi::UObjectHandle::null();
                unsafe { ::rusteal_runtime::runtime::ffi_dispatch::property_get_object(#container, prop, &mut h); }
                unsafe { <#rust_ty>::from_raw(::rusteal_runtime::ffi::UClassHandle(h.0)) }
            },
            Some((
                quote! { #rust_ty },
                quote! {
                    unsafe {
                        ::rusteal_runtime::runtime::ffi_dispatch::property_set_object(
                            #container, prop, ::rusteal_runtime::ffi::UObjectHandle(val.raw().0),
                        );
                    }
                },
            )),
        ),
        prop_type::PropKind::Array { .. } => (quote! { <#rust_ty>::new(#container, prop) }, None),
        prop_type::PropKind::OwnedStruct { .. } => (
            quote! {
                let size = unsafe { ::rusteal_runtime::runtime::ffi_dispatch::reflection_get_property_size(prop) } as usize;
                let mut buf = vec![0u8; size];
                ::rusteal_runtime::runtime::ffi_infallible_ctx(unsafe {
                    ::rusteal_runtime::runtime::ffi_dispatch::property_get_struct(#container, prop, buf.as_mut_ptr(), size as u32)
                }, #ue_name);
                unsafe { ::rusteal_runtime::runtime::OwnedStruct::from_constructed(buf) }
            },
            Some((
                quote! { &#rust_ty },
                quote! {
                    let bytes = val.to_bytes();
                    ::rusteal_runtime::runtime::ffi_infallible_ctx(unsafe {
                        ::rusteal_runtime::runtime::ffi_dispatch::property_set_struct(#container, prop, bytes.as_ptr(), bytes.len() as u32)
                    }, #ue_name);
                },
            )),
        ),
        prop_type::PropKind::SoftObject { .. } => (
            quote! {
                let mut len: u32 = 0;
                unsafe {
                    ::rusteal_runtime::runtime::ffi_dispatch::property_get_soft_object_path(
                        #container, prop, std::ptr::null_mut(), 0, &mut len,
                    );
                }
                let mut buf = vec![0u8; len as usize];
                unsafe {
                    ::rusteal_runtime::runtime::ffi_dispatch::property_get_soft_object_path(
                        #container, prop, buf.as_mut_ptr(), len, &mut len,
                    );
                }
                buf.truncate(len as usize);
                ::rusteal_runtime::runtime::SoftObjectRef::new(String::from_utf8(buf).unwrap_or_default())
            },
            Some((
                quote! { &#rust_ty },
                quote! {
                    let path = val.path();
                    unsafe {
                        ::rusteal_runtime::runtime::ffi_dispatch::property_set_soft_object_path(
                            #container, prop, path.as_ptr(), path.len() as u32,
                        );
                    }
                },
            )),
        ),
        prop_type::PropKind::Name => (
            quote! {
                let mut h = ::rusteal_runtime::runtime::FName::NONE.handle();
                unsafe { ::rusteal_runtime::runtime::ffi_dispatch::property_get_fname(#container, prop, &mut h); }
                ::rusteal_runtime::runtime::FName(h)
            },
            Some((
                quote! { #rust_ty },
                quote! {
                    unsafe { ::rusteal_runtime::runtime::ffi_dispatch::property_set_fname(#container, prop, val.handle()); }
                },
            )),
        ),
        prop_type::PropKind::Str => (
            quote! {
                let mut len: u32 = 0;
                unsafe {
                    ::rusteal_runtime::runtime::ffi_dispatch::property_get_string(
                        #container, prop, std::ptr::null_mut(), 0, &mut len,
                    );
                }
                let mut buf = vec![0u8; len as usize];
                unsafe {
                    ::rusteal_runtime::runtime::ffi_dispatch::property_get_string(
                        #container, prop, buf.as_mut_ptr(), len, &mut len,
                    );
                }
                buf.truncate(len as usize);
                String::from_utf8(buf).unwrap_or_default()
            },
            Some((
                quote! { &str },
                quote! {
                    unsafe {
                        ::rusteal_runtime::runtime::ffi_dispatch::property_set_string(
                            #container, prop, val.as_ptr(), val.len() as u32,
                        );
                    }
                },
            )),
        ),
        prop_type::PropKind::Enum { ty } => (
            quote! {
                let mut raw: i64 = 0;
                unsafe { ::rusteal_runtime::runtime::ffi_dispatch::property_get_enum(#container, prop, &mut raw); }
                <#ty as ::rusteal_runtime::runtime::UeEnum>::from_i64(raw)
                    .unwrap_or_else(|| panic!("{} holds {}, not a {}", #ue_name, raw, stringify!(#ty)))
            },
            Some((
                quote! { #rust_ty },
                quote! {
                    unsafe {
                        ::rusteal_runtime::runtime::ffi_dispatch::property_set_enum(
                            #container, prop, ::rusteal_runtime::runtime::UeEnum::to_i64(val),
                        );
                    }
                },
            )),
        ),
        prop_type::PropKind::RawScalar { zero_expr } => (
            quote! {
                let mut val: #rust_ty = #zero_expr;
                unsafe {
                    ::rusteal_runtime::runtime::ffi_dispatch::property_get_property_at(
                        #container, prop, 0, &mut val as *mut #rust_ty as *mut u8,
                        std::mem::size_of::<#rust_ty>() as u32,
                    );
                }
                val
            },
            Some((
                quote! { #rust_ty },
                quote! {
                    unsafe {
                        ::rusteal_runtime::runtime::ffi_dispatch::property_set_property_at(
                            #container, prop, 0, &val as *const #rust_ty as *const u8,
                            std::mem::size_of::<#rust_ty>() as u32,
                        );
                    }
                },
            )),
        ),
        prop_type::PropKind::Text => (
            quote! {
                let mut raw: *mut u8 = std::ptr::null_mut();
                ::rusteal_runtime::runtime::ffi_infallible_ctx(unsafe {
                    ::rusteal_runtime::runtime::ffi_dispatch::property_get_text(#container, prop, &mut raw)
                }, #ue_name);
                unsafe { ::rusteal_runtime::runtime::Text::from_raw(raw) }
            },
            Some((
                quote! { impl Into<::rusteal_runtime::runtime::Text> },
                quote! {
                    let val: ::rusteal_runtime::runtime::Text = val.into();
                    ::rusteal_runtime::runtime::ffi_infallible_ctx(unsafe {
                        ::rusteal_runtime::runtime::ffi_dispatch::property_set_text(#container, prop, val.as_raw())
                    }, #ue_name);
                },
            )),
        ),
        prop_type::PropKind::Weak { .. } => (
            quote! {
                let mut h = ::rusteal_runtime::ffi::UObjectHandle::null();
                unsafe { ::rusteal_runtime::runtime::ffi_dispatch::property_get_object(#container, prop, &mut h); }
                unsafe { <#rust_ty as ::rusteal_runtime::runtime::ObjectPointer>::from_object_handle(h) }
            },
            Some((
                quote! { #rust_ty },
                quote! {
                    let h = ::rusteal_runtime::runtime::ObjectPointer::object_handle(&val);
                    unsafe { ::rusteal_runtime::runtime::ffi_dispatch::property_set_object(#container, prop, h); }
                },
            )),
        ),
        prop_type::PropKind::SoftClass { .. } => (
            quote! {
                let mut len: u32 = 0;
                unsafe {
                    ::rusteal_runtime::runtime::ffi_dispatch::property_get_soft_object_path(
                        #container, prop, std::ptr::null_mut(), 0, &mut len,
                    );
                }
                let mut buf = vec![0u8; len as usize];
                unsafe {
                    ::rusteal_runtime::runtime::ffi_dispatch::property_get_soft_object_path(
                        #container, prop, buf.as_mut_ptr(), len, &mut len,
                    );
                }
                buf.truncate(len as usize);
                <#rust_ty>::new(String::from_utf8(buf).unwrap_or_default())
            },
            Some((
                quote! { &#rust_ty },
                quote! {
                    let path = val.path();
                    unsafe {
                        ::rusteal_runtime::runtime::ffi_dispatch::property_set_soft_object_path(
                            #container, prop, path.as_ptr(), path.len() as u32,
                        );
                    }
                },
            )),
        ),
        prop_type::PropKind::Interface { .. } => (
            quote! {
                let mut h = ::rusteal_runtime::ffi::UObjectHandle::null();
                unsafe { ::rusteal_runtime::runtime::ffi_dispatch::property_get_object(#container, prop, &mut h); }
                <#rust_ty>::new(h).ok()
            },
            Some((
                quote! { Option<#rust_ty> },
                quote! {
                    let h = val.map_or(::rusteal_runtime::ffi::UObjectHandle::null(), |v| v.raw());
                    unsafe { ::rusteal_runtime::runtime::ffi_dispatch::property_set_object(#container, prop, h); }
                },
            )),
        ),
        prop_type::PropKind::Map { .. } | prop_type::PropKind::Set { .. } => {
            (quote! { <#rust_ty>::new(#container, prop) }, None)
        }
        prop_type::PropKind::Vec { element } => {
            let elem = &element.rust_type;

            (
                quote! {
                    ::rusteal_runtime::runtime::UeArray::<#elem>::new(#container, prop)
                        .to_vec()
                        .unwrap_or_default()
                },
                Some((
                    quote! { #rust_ty },
                    quote! {
                        let _ = ::rusteal_runtime::runtime::UeArray::<#elem>::new(#container, prop).set_all(&val);
                    },
                )),
            )
        }
        prop_type::PropKind::HashSet { element } => {
            let elem = &element.rust_type;

            (
                quote! {
                    ::rusteal_runtime::runtime::UeSet::<#elem>::new(#container, prop)
                        .to_hash_set()
                        .unwrap_or_default()
                },
                Some((
                    quote! { #rust_ty },
                    quote! {
                        let set = ::rusteal_runtime::runtime::UeSet::<#elem>::new(#container, prop);
                        let _ = set.clear();

                        for item in &val {
                            let _ = set.add(item);
                        }
                    },
                )),
            )
        }
        prop_type::PropKind::HashMap { key, value } => {
            let key_ty = &key.rust_type;
            let value_ty = &value.rust_type;

            (
                quote! {
                    ::rusteal_runtime::runtime::UeMap::<#key_ty, #value_ty>::new(#container, prop)
                        .to_hash_map()
                        .unwrap_or_default()
                },
                Some((
                    quote! { #rust_ty },
                    quote! {
                        let map = ::rusteal_runtime::runtime::UeMap::<#key_ty, #value_ty>::new(#container, prop);
                        let _ = map.clear();

                        for (key, value) in &val {
                            let _ = map.add(key, value);
                        }
                    },
                )),
            )
        }
        prop_type::PropKind::Struct { .. } => {
            unreachable!("rejected when the field was parsed")
        }
    }
}

pub(crate) fn property_accessors(
    uprops: &[UPropertyField],
    container: &TokenStream,
    find_fn: &Ident,
    owner: &TokenStream,
) -> Vec<Accessor> {
    let mut accessors = Vec::new();

    for prop in uprops {
        let info = prop_type::map_type(&prop.ty).unwrap();
        let field_ident = &prop.ident;
        let ue_name = prop.ue_name();
        let ue_name_bytes = ue_name.as_bytes();
        let ue_name_len = ue_name.len() as u32;
        let rust_ty = &info.rust_type;

        let find_prop = quote! {
            static PROP: std::sync::OnceLock<::rusteal_runtime::ffi::FPropertyHandle> = std::sync::OnceLock::new();
            let prop = *PROP.get_or_init(|| unsafe {
                ::rusteal_runtime::runtime::ffi_dispatch::#find_fn(
                    #owner,
                    [#(#ue_name_bytes),*].as_ptr(),
                    #ue_name_len,
                )
            });
        };

        let (getter_body, setter_body) = accessor_parts(&info.kind, rust_ty, container, &ue_name);

        let getter_ret = if matches!(info.kind, prop_type::PropKind::Interface { .. }) {
            quote! { Option<#rust_ty> }
        } else {
            quote! { #rust_ty }
        };

        accessors.push(Accessor {
            name: format_ident!("{}", field_ident),
            param: None,
            ret: Some(getter_ret),
            body: quote! {
                #find_prop
                #getter_body
            },
        });

        if let Some((setter_ty, setter_body)) = setter_body {
            accessors.push(Accessor {
                name: format_ident!("set_{}", field_ident),
                param: Some(quote! { val: #setter_ty }),
                ret: None,
                body: quote! {
                    #find_prop
                    #setter_body
                },
            });
        }
    }

    accessors
}

pub(crate) fn add_property_statements(uprops: &[UPropertyField]) -> Vec<TokenStream> {
    let mut add_prop_stmts: Vec<TokenStream> = Vec::new();

    for prop in uprops {
        let info = prop_type::map_type(&prop.ty).unwrap();
        let ue_name = prop.ue_name();
        let ue_name_bytes = ue_name.as_bytes();
        let ue_name_len = ue_name.len() as u32;
        let prop_type_expr = &info.prop_type_expr;
        let prop_var = format_ident!("_prop_{}", prop.ident);

        let mut flag_parts = Vec::new();

        if prop.args.blueprint_read_write || prop.args.blueprint_read_only {
            flag_parts.push(quote! { ::rusteal_runtime::ffi::CPF_BLUEPRINT_VISIBLE });
        }

        if prop.args.blueprint_read_only {
            flag_parts.push(quote! { ::rusteal_runtime::ffi::CPF_BLUEPRINT_READ_ONLY });

            if !prop.args.edit_anywhere && !prop.args.edit_defaults_only {
                flag_parts.push(quote! { ::rusteal_runtime::ffi::CPF_EDIT });
                flag_parts.push(quote! { ::rusteal_runtime::ffi::CPF_EDIT_CONST });
            }
        }

        if prop.args.edit_anywhere || prop.args.blueprint_read_write {
            flag_parts.push(quote! { ::rusteal_runtime::ffi::CPF_EDIT });
        }

        if prop.args.edit_defaults_only {
            flag_parts.push(quote! { ::rusteal_runtime::ffi::CPF_EDIT });
            flag_parts.push(quote! { ::rusteal_runtime::ffi::CPF_DISABLE_EDIT_ON_INSTANCE });
        }

        if prop.args.visible_anywhere {
            flag_parts.push(quote! { ::rusteal_runtime::ffi::CPF_EDIT });
            flag_parts.push(quote! { ::rusteal_runtime::ffi::CPF_EDIT_CONST });
        }

        if prop.args.save_game {
            flag_parts.push(quote! { ::rusteal_runtime::ffi::CPF_SAVE_GAME });
        }

        if prop.args.config {
            flag_parts.push(quote! { ::rusteal_runtime::ffi::CPF_CONFIG });
        }

        if prop.args.transient {
            flag_parts.push(quote! { ::rusteal_runtime::ffi::CPF_TRANSIENT });
        }

        if flag_parts.is_empty() {
            flag_parts.push(quote! { 0u64 });
        }

        let flags_expr = quote! { #(#flag_parts)|* };

        let extra_expr = match info.extra_fields() {
            Some(fields) => quote! {
                &::rusteal_runtime::ffi::RustealReifyPropExtra {
                    #fields
                    ..::core::default::Default::default()
                }
            },
            None => quote! { std::ptr::null() },
        };

        let metadata = prop
            .args
            .category
            .iter()
            .map(|category| ("Category", category.as_str()))
            .chain(prop.args.bind_widget.map(|key| (key, "true")))
            .chain(
                prop.args
                    .meta
                    .iter()
                    .map(|(key, value)| (key.as_str(), value.as_str())),
            );

        let meta_stmts: Vec<TokenStream> = metadata
            .map(|(key, value)| {
                let key_bytes = key.as_bytes();
                let key_len = key.len() as u32;
                let value_bytes = value.as_bytes();
                let value_len = value.len() as u32;

                quote! {
                    unsafe {
                        ::rusteal_runtime::runtime::ffi_dispatch::reify_set_property_metadata(
                            #prop_var, [#(#key_bytes),*].as_ptr(), #key_len,
                            [#(#value_bytes),*].as_ptr(), #value_len,
                        );
                    }
                }
            })
            .collect();

        add_prop_stmts.push(quote! {
            let #prop_var = unsafe {
                ::rusteal_runtime::runtime::ffi_dispatch::reify_add_property(
                    class,
                    [#(#ue_name_bytes),*].as_ptr(),
                    #ue_name_len,
                    #prop_type_expr as u32,
                    #flags_expr,
                    #extra_expr,
                )
            };
            if !#prop_var.is_null() {
                #(#meta_stmts)*
            }
        });
    }

    add_prop_stmts
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn implements_takes_interface_paths() {
        let args = parse_uclass_args(quote::quote!(
            parent = Actor,
            implements = ["/Script/Game.Usable", "/Game/BPI_X.BPI_X_C"]
        ))
        .unwrap();

        assert_eq!(
            args.implements,
            ["/Script/Game.Usable", "/Game/BPI_X.BPI_X_C"]
        );

        let one = parse_uclass_args(quote::quote!(
            parent = Actor,
            implements = "/Script/Game.Usable"
        ))
        .unwrap();

        assert_eq!(one.implements, ["/Script/Game.Usable"]);
        assert!(parse_uclass_args(quote::quote!(parent = Actor, implements = ["Usable"])).is_err());
        assert!(parse_uclass_args(quote::quote!(parent = Actor, implements = [1])).is_err());
    }

    #[test]
    fn implements_takes_rust_interfaces() {
        let args = parse_uclass_args(quote::quote!(
            parent = Actor,
            implements = [Usable, crate::doors::Lockable, "/Game/BPI_X.BPI_X_C"]
        ))
        .unwrap();

        assert_eq!(args.implements, ["/Game/BPI_X.BPI_X_C"]);
        assert_eq!(args.rust_interfaces.len(), 2);

        let out = expand_uclass(
            quote::quote!(parent = Actor, implements = [Usable]),
            quote::quote!(
                pub struct Door {}
            ),
        )
        .unwrap()
        .to_string();

        assert!(out.contains("< dyn Usable > :: __rusteal_interface_handle"));
        assert!(out.contains("reify_add_interface_class"));
    }

    #[test]
    fn type_ids_follow_the_module_path_and_parents_give_their_own() {
        let out = expand_uclass(
            quote::quote!(parent = super::BaseItem),
            quote::quote!(
                pub struct Item {
                    #[component(root)]
                    mesh: StaticMeshComponent,
                }
            ),
        )
        .unwrap()
        .to_string();

        assert!(
            out.contains("concat ! (module_path ! () , \"::\" , \"Item\")"),
            "{out}"
        );
        assert!(out.contains(
            "< super :: BaseItem as :: rusteal_runtime :: runtime :: UeClass > :: RUST_TYPE_ID"
        ));
        assert!(out.contains(
            "< StaticMeshComponent as :: rusteal_runtime :: runtime :: UeClass > :: RUST_TYPE_ID"
        ));
        assert!(out.contains("krate : env ! (\"CARGO_CRATE_NAME\")"));
    }

    #[test]
    fn config_names_the_ini_category() {
        let args =
            parse_uclass_args(quote::quote!(parent = DeveloperSettings, config = "Game")).unwrap();

        assert_eq!(args.config.as_deref(), Some("Game"));

        assert!(
            parse_uclass_args(quote::quote!(parent = Actor))
                .unwrap()
                .config
                .is_none()
        );

        assert!(parse_uclass_args(quote::quote!(parent = Actor, config = Game)).is_err());
    }

    #[test]
    fn config_sets_the_property_flag() {
        let field = UPropertyField {
            ident: parse_quote!(max_items),
            ty: parse_quote!(i32),
            args: parse_uproperty_args(&parse_quote!(#[uproperty(Config, EditAnywhere)])).unwrap(),
        };

        assert!(
            add_property_statements(&[field])[0]
                .to_string()
                .contains("CPF_CONFIG")
        );
    }

    #[test]
    fn save_game_is_a_uproperty_argument() {
        let attr: syn::Attribute = parse_quote!(#[uproperty(EditAnywhere, SaveGame)]);
        let args = parse_uproperty_args(&attr).unwrap();
        assert!(args.save_game && args.edit_anywhere);
    }

    #[test]
    fn save_game_sets_the_property_flag() {
        let field = |attr: syn::Attribute| UPropertyField {
            ident: parse_quote!(best_time),
            ty: parse_quote!(f32),
            args: parse_uproperty_args(&attr).unwrap(),
        };

        let flagged = add_property_statements(&[field(parse_quote!(#[uproperty(SaveGame)]))]);
        assert!(flagged[0].to_string().contains("CPF_SAVE_GAME"));
        let plain = add_property_statements(&[field(parse_quote!(#[uproperty(EditAnywhere)]))]);
        assert!(!plain[0].to_string().contains("CPF_SAVE_GAME"));
    }

    #[test]
    fn meta_entries_and_doc_comments_become_metadata() {
        let attr: syn::Attribute = parse_quote!(#[uproperty(
            EditAnywhere,
            meta(ClampMin = 0, ClampMax = 1.5, UIMin = -10, Units = "cm", InlineEditConditionToggle)
        )]);
        let mut args = parse_uproperty_args(&attr).unwrap();
        let field: syn::Field = parse_quote! {
            /// How far it reaches.
            ///
            /// In centimetres.
            range: f32
        };
        args.add_doc_tooltip(&field.attrs);

        let meta: Vec<(&str, &str)> = args
            .meta
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();

        assert_eq!(
            meta,
            [
                ("ClampMin", "0"),
                ("ClampMax", "1.5"),
                ("UIMin", "-10"),
                ("Units", "cm"),
                ("InlineEditConditionToggle", "true"),
                ("ToolTip", "How far it reaches.\n\nIn centimetres."),
            ]
        );

        let attr: syn::Attribute = parse_quote!(#[uproperty(meta(ToolTip = "Given"))]);
        let mut args = parse_uproperty_args(&attr).unwrap();
        args.add_doc_tooltip(&field.attrs);
        assert_eq!(args.meta, [("ToolTip".to_string(), "Given".to_string())]);

        let attr: syn::Attribute = parse_quote!(#[uproperty(meta(ClampMin(0)))]);
        assert!(parse_uproperty_args(&attr).is_err());
    }

    #[test]
    fn bind_widget_properties_carry_their_metadata() {
        let attr: syn::Attribute = parse_quote!(#[uproperty(BindWidget, BlueprintReadOnly)]);
        assert_eq!(
            parse_uproperty_args(&attr).unwrap().bind_widget,
            Some("BindWidget")
        );
        let attr: syn::Attribute = parse_quote!(#[uproperty(BindWidgetAnim)]);
        let args = parse_uproperty_args(&attr).unwrap();
        assert_eq!(args.bind_widget, Some("BindWidgetAnim"));
        assert!(args.transient);
        let attr: syn::Attribute = parse_quote!(#[uproperty(BindWidgetOptional)]);
        assert_eq!(
            parse_uproperty_args(&attr).unwrap().bind_widget,
            Some("BindWidgetOptional")
        );
    }

    #[test]
    fn unknown_uproperty_arguments_are_errors() {
        let attr: syn::Attribute = parse_quote!(#[uproperty(EditAnywhere, Replicated)]);
        let err = parse_uproperty_args(&attr).err().unwrap();
        assert!(err.to_string().contains("unknown #[uproperty] argument"));
        let attr: syn::Attribute = parse_quote!(#[uproperty(meta = "x")]);
        assert!(parse_uproperty_args(&attr).is_err());
    }
}
