use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident, quote};
use syn::ext::IdentExt;
use syn::punctuated::Punctuated;
use syn::{
    FnArg, Ident, ImplItem, ImplItemFn, ItemImpl, Meta, ReceiverKind, ReturnType, Token, Type,
    parse2,
};

use crate::prop_type;
use crate::shape;
use crate::uclass::{to_screaming_snake, to_snake_case};
use crate::udelegate;

pub(crate) struct ParamInfo {
    pub rust_name: Ident,
    pub ue_name: String,
    pub rust_ty: Type,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FnKind {
    Callable,
    Pure,
    Override,
    ImplementableEvent,
    InterfaceImpl,
}

struct UFunctionInfo {
    method_ident: Ident,
    ue_name: String,
    params: Vec<ParamInfo>,
    return_type: Option<ParamInfo>,
    is_mut: bool,
    kind: FnKind,
    exec: bool,
    name_expr: TokenStream,
    param_name_exprs: Vec<TokenStream>,
}

pub fn expand_uclass_impl(_attr: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let input: ItemImpl = parse2(item)?;

    let struct_name = match &*input.self_ty {
        Type::Path(tp) => tp
            .path
            .segments
            .last()
            .ok_or_else(|| syn::Error::new_spanned(&input.self_ty, "expected type name"))?
            .ident
            .clone(),
        _ => {
            return Err(syn::Error::new_spanned(
                &input.self_ty,
                "expected type name",
            ));
        }
    };

    let struct_name_str = struct_name.to_string();
    let rust_data_name = format_ident!("__{}RustData", struct_name);

    let class_handle_name = format_ident!(
        "__RUSTEAL_CLASS_HANDLE_{}",
        to_screaming_snake(&struct_name_str)
    );

    let mut ufunctions: Vec<UFunctionInfo> = Vec::new();
    let mut delegates: Vec<udelegate::DelegateInfo> = Vec::new();
    let mut class_defaults: Option<&ImplItemFn> = None;

    let mut shape_parts: Vec<String> = Vec::new();
    let mut clean_impl = input.clone();

    let interface = input.trait_.as_ref().map(|(path, _)| path.clone());

    if let Some(trait_path) = &interface {
        shape_parts.push(format!("impl {}", trait_path.to_token_stream()));

        for item in &input.items {
            let ImplItem::Fn(method) = item else {
                return Err(syn::Error::new_spanned(
                    item,
                    "an interface implementation holds the interface's functions only",
                ));
            };

            shape_parts.push(shape::declaration(&method.attrs, &[], &method.sig));
            ufunctions.push(parse_interface_fn(method, trait_path)?);
        }
    }

    for item in &input.items {
        if interface.is_some() {
            break;
        }

        if let ImplItem::Fn(method) = item {
            let has_ufunction = method.attrs.iter().any(|a| a.path().is_ident("ufunction"));

            if has_ufunction || udelegate::is_udelegate(method) {
                shape_parts.push(shape::declaration(
                    &method.attrs,
                    &["ufunction", "udelegate"],
                    &method.sig,
                ));
            }

            if has_ufunction {
                ufunctions.push(parse_ufunction(method)?);
            }

            if udelegate::is_udelegate(method) {
                if has_ufunction {
                    return Err(syn::Error::new_spanned(
                        &method.sig,
                        "a method is a #[ufunction] or a #[udelegate], not both",
                    ));
                }

                delegates.push(udelegate::parse_udelegate(method)?);
            }

            if method
                .attrs
                .iter()
                .any(|a| a.path().is_ident("class_defaults"))
            {
                if class_defaults.is_some() {
                    return Err(syn::Error::new_spanned(
                        &method.sig,
                        "only one #[class_defaults] method per impl block",
                    ));
                }

                let takes_only_self = method.sig.inputs.len() == 1
                    && matches!(method.sig.inputs.first(), Some(FnArg::Receiver(_)));

                if !takes_only_self || has_ufunction {
                    return Err(syn::Error::new_spanned(
                        &method.sig,
                        "#[class_defaults] takes `&mut self` (the class default object) and nothing else, \
                         and is not a #[ufunction]",
                    ));
                }

                class_defaults = Some(method);
            }
        }
    }

    for item in &mut clean_impl.items {
        if let ImplItem::Fn(method) = item {
            if let Some(uf) = ufunctions
                .iter()
                .find(|uf| uf.method_ident == method.sig.ident)
                && uf.kind == FnKind::ImplementableEvent
            {
                method.block = implementable_event_body(uf);
            }

            if let Some(d) = delegates
                .iter()
                .find(|d| d.method_ident == method.sig.ident)
            {
                method.block = udelegate::broadcast_body(&struct_name, d);
            }

            method.attrs.retain(|a| {
                !a.path().is_ident("ufunction")
                    && !a.path().is_ident("class_defaults")
                    && !a.path().is_ident("udelegate")
            });
        }
    }

    let interface_suffix = interface
        .as_ref()
        .and_then(|path| path.segments.last())
        .map(|segment| format!("_{}", to_snake_case(&segment.ident.to_string())))
        .unwrap_or_default();

    let register_fns_name = format_ident!(
        "__rusteal_register_{}{}_functions",
        to_snake_case(&struct_name_str),
        interface_suffix,
    );

    let mut register_stmts: Vec<TokenStream> = Vec::new();

    for uf in &ufunctions {
        let name_expr = &uf.name_expr;
        let method_ident = &uf.method_ident;

        let flags_expr = match uf.kind {
            FnKind::Override => quote! {
                ::rusteal_runtime::ffi::FUNC_NATIVE | ::rusteal_runtime::ffi::FUNC_BLUEPRINT_EVENT | ::rusteal_runtime::ffi::FUNC_PUBLIC
            },
            FnKind::Callable => quote! {
                ::rusteal_runtime::ffi::FUNC_NATIVE | ::rusteal_runtime::ffi::FUNC_BLUEPRINT_CALLABLE | ::rusteal_runtime::ffi::FUNC_PUBLIC
            },
            FnKind::Pure => quote! {
                ::rusteal_runtime::ffi::FUNC_NATIVE | ::rusteal_runtime::ffi::FUNC_BLUEPRINT_CALLABLE
                    | ::rusteal_runtime::ffi::FUNC_BLUEPRINT_PURE | ::rusteal_runtime::ffi::FUNC_PUBLIC
            },
            FnKind::ImplementableEvent => quote! {
                ::rusteal_runtime::ffi::FUNC_EVENT | ::rusteal_runtime::ffi::FUNC_BLUEPRINT_EVENT
                    | ::rusteal_runtime::ffi::FUNC_BLUEPRINT_CALLABLE | ::rusteal_runtime::ffi::FUNC_PUBLIC
            },
            FnKind::InterfaceImpl => quote! {
                ::rusteal_runtime::ffi::FUNC_NATIVE | ::rusteal_runtime::ffi::FUNC_EVENT
                    | ::rusteal_runtime::ffi::FUNC_BLUEPRINT_EVENT | ::rusteal_runtime::ffi::FUNC_BLUEPRINT_CALLABLE
                    | ::rusteal_runtime::ffi::FUNC_PUBLIC
            },
        };

        let flags_expr = if uf.exec {
            quote! { #flags_expr | ::rusteal_runtime::ffi::FUNC_EXEC }
        } else {
            flags_expr
        };

        let total_offsets = uf.params.len() + if uf.return_type.is_some() { 1 } else { 0 };

        let mut offset_inits: Vec<TokenStream> = Vec::new();

        for param_name in &uf.param_name_exprs {
            offset_inits.push(quote! {
                {
                    let p = ::rusteal_runtime::runtime::ffi_dispatch::reflection_get_function_param(
                        func,
                        #param_name.as_ptr(),
                        #param_name.len() as u32,
                    );

                    ::rusteal_runtime::runtime::ffi_dispatch::reflection_get_property_offset(p)
                }
            });
        }

        if uf.return_type.is_some() {
            offset_inits.push(quote! {
                {
                    let p = ::rusteal_runtime::runtime::ffi_dispatch::reflection_get_function_param(
                        func,
                        b"ReturnValue".as_ptr(),
                        11u32,
                    );

                    ::rusteal_runtime::runtime::ffi_dispatch::reflection_get_property_offset(p)
                }
            });
        }

        let mut param_reads: Vec<TokenStream> = Vec::new();
        let mut param_idents: Vec<&Ident> = Vec::new();

        for (i, param) in uf.params.iter().enumerate() {
            let rust_name = &param.rust_name;
            let rust_ty = &param.rust_ty;
            let idx = syn::Index::from(i);

            let read_handle = quote! {
                ::rusteal_runtime::runtime::ffi_dispatch::native_mem_read::<::rusteal_runtime::ffi::UObjectHandle>(
                    params, offsets[#idx] as usize,
                )
            };

            let is_out_ref = matches!(
                rust_ty,
                Type::Path(tp) if tp.path.segments.last().is_some_and(|seg| seg.ident == "OutRef")
            );

            let read = match prop_type::map_type(rust_ty).map(|info| info.kind) {
                _ if is_out_ref => quote! {
                    ::rusteal_runtime::runtime::out_ref_from_param(params, offsets[#idx] as usize)
                },
                Some(prop_type::PropKind::Struct { .. }) => quote! {
                    ::rusteal_runtime::runtime::struct_ref_from_param(params, offsets[#idx] as usize)
                },
                Some(prop_type::PropKind::Object { .. }) => quote! {
                    <#rust_ty>::from_raw(#read_handle)
                },
                Some(prop_type::PropKind::Class { .. }) => quote! {
                    <#rust_ty>::from_raw(::rusteal_runtime::ffi::UClassHandle(#read_handle.0))
                },
                Some(prop_type::PropKind::Enum { .. }) => quote! {
                    {
                        const _: () = assert!(
                            ::core::mem::size_of::<#rust_ty>() == 1,
                            "a ufunction enum parameter is a one-byte (uint8) Unreal enum"
                        );

                        let raw = ::rusteal_runtime::runtime::ffi_dispatch::native_mem_read::<
                            <#rust_ty as ::rusteal_runtime::runtime::UeEnum>::Repr,
                        >(params, offsets[#idx] as usize);

                        <#rust_ty as ::rusteal_runtime::runtime::UeEnum>::from_i64(
                            i64::try_from(raw).unwrap_or(-1),
                        )
                        .expect(concat!("an invalid value for ", stringify!(#rust_ty)))
                    }
                },
                _ => quote! {
                    ::rusteal_runtime::runtime::ffi_dispatch::native_mem_read::<#rust_ty>(params, offsets[#idx] as usize)
                },
            };

            param_reads.push(quote! {
                let #rust_name: #rust_ty = unsafe { #read };
            });

            param_idents.push(rust_name);
        }

        let (return_zero_init, return_write) = if let Some(ref ret) = uf.return_type {
            let ret_ty = &ret.rust_ty;
            let ret_idx = syn::Index::from(uf.params.len());
            let write = |value: TokenStream| {
                quote! {
                    unsafe {
                        ::rusteal_runtime::runtime::ffi_dispatch::native_mem_write(
                            params, offsets[#ret_idx] as usize, #value,
                        );
                    }
                }
            };
            match prop_type::map_type(ret_ty).map(|info| info.kind) {
                Some(prop_type::PropKind::Object { .. }) => (
                    write(quote! { ::rusteal_runtime::ffi::UObjectHandle::null() }),
                    write(quote! { __ret.raw() }),
                ),
                Some(prop_type::PropKind::Class { .. }) => (
                    write(quote! { ::rusteal_runtime::ffi::UObjectHandle::null() }),
                    write(quote! { ::rusteal_runtime::ffi::UObjectHandle(__ret.raw().0) }),
                ),
                Some(prop_type::PropKind::OwnedStruct { strukt }) => (
                    quote! {},
                    quote! {
                        unsafe {
                            let _ = ::rusteal_runtime::runtime::ffi_dispatch::reflection_copy_struct(
                                <#strukt as ::rusteal_runtime::runtime::UeStruct>::static_struct(),
                                params.add(offsets[#ret_idx] as usize),
                                __ret.as_bytes().as_ptr(),
                            );
                        }
                    },
                ),
                Some(prop_type::PropKind::Enum { .. }) => (
                    write(quote! {
                        {
                            const _: () = assert!(
                                ::core::mem::size_of::<#ret_ty>() == 1,
                                "a ufunction enum return value is a one-byte (uint8) Unreal enum"
                            );

                            <<#ret_ty as ::rusteal_runtime::runtime::UeEnum>::Repr as ::core::default::Default>::default()
                        }
                    }),
                    write(quote! { __ret }),
                ),
                _ => (
                    write(quote! { unsafe { std::mem::zeroed::<#ret_ty>() } }),
                    write(quote! { __ret }),
                ),
            }
        } else {
            (quote! {}, quote! {})
        };

        let this_binding = if uf.is_mut {
            quote! {
                let mut __this = #struct_name {
                    __obj: obj,
                    __rust_data: rust_data as *mut #rust_data_name,
                };
            }
        } else {
            quote! {
                let __this = #struct_name {
                    __obj: obj,
                    __rust_data: rust_data as *mut #rust_data_name,
                };
            }
        };

        let call = match &interface {
            Some(trait_path) if uf.is_mut => {
                quote! { #trait_path::#method_ident(&mut __this, #(#param_idents),*) }
            }
            Some(trait_path) => quote! { #trait_path::#method_ident(&__this, #(#param_idents),*) },
            None => quote! { __this.#method_ident(#(#param_idents),*) },
        };

        let call_expr = if uf.return_type.is_some() {
            quote! { let __ret = #call; }
        } else {
            quote! { #call; }
        };

        let func_var = format_ident!("__func_{}", method_ident);

        if uf.kind == FnKind::ImplementableEvent {
            register_stmts.push(quote! {
                let #func_var = unsafe {
                    ::rusteal_runtime::runtime::ffi_dispatch::reify_add_function(
                        cls,
                        #name_expr.as_ptr(),
                        #name_expr.len() as u32,
                        0u64,
                        #flags_expr,
                    )
                };
            });
        } else {
            register_stmts.push(quote! {
            let __callback_id = {
                let callback_id = ::rusteal_runtime::runtime::reify_registry::register_function(
                    #struct_name::__RUSTEAL_TYPE_ID,
                    move |obj: ::rusteal_runtime::ffi::UObjectHandle, rust_data: *mut u8, params: ::rusteal_runtime::runtime::ffi_dispatch::NativePtr| {
                        static OFFSETS: std::sync::OnceLock<[u32; #total_offsets]> = std::sync::OnceLock::new();

                        let offsets = OFFSETS.get_or_init(|| unsafe {
                            let cls = <#struct_name as ::rusteal_runtime::runtime::UeClass>::static_class();
                            let func = ::rusteal_runtime::runtime::ffi_dispatch::reflection_find_function_by_class(
                                cls,
                                #name_expr.as_ptr(),
                                #name_expr.len() as u32,
                            );
                            [#(#offset_inits),*]
                        });

                        #(#param_reads)*
                        #return_zero_init
                        #this_binding
                        #call_expr
                        #return_write
                    }
                );
                callback_id
            };

            let #func_var = unsafe {
                ::rusteal_runtime::runtime::ffi_dispatch::reify_add_function(
                    cls,
                    #name_expr.as_ptr(),
                    #name_expr.len() as u32,
                    __callback_id,
                    #flags_expr,
                )
            };
        });
        }

        if uf.kind != FnKind::Override {
            for (param, param_name) in uf.params.iter().zip(&uf.param_name_exprs) {
                let reg_ty =
                    event_struct_param(&param.rust_ty).unwrap_or_else(|| param.rust_ty.clone());

                let info = prop_type::map_type(&reg_ty).unwrap();
                let prop_type_expr = &info.prop_type_expr;
                let extra_expr = extra_expr(&info);

                register_stmts.push(quote! {
                    unsafe {
                        ::rusteal_runtime::runtime::ffi_dispatch::reify_add_function_param(
                            #func_var,
                            #param_name.as_ptr(),
                            #param_name.len() as u32,
                            #prop_type_expr as u32,
                            ::rusteal_runtime::ffi::CPF_PARM,
                            #extra_expr,
                        );
                    }
                });
            }

            if let Some(ref ret) = uf.return_type {
                let info = prop_type::map_type(&ret.rust_ty).unwrap();
                let prop_type_expr = &info.prop_type_expr;
                let extra_expr = extra_expr(&info);

                register_stmts.push(quote! {
                    unsafe {
                        ::rusteal_runtime::runtime::ffi_dispatch::reify_add_function_param(
                            #func_var,
                            b"ReturnValue".as_ptr(),
                            11u32,
                            #prop_type_expr as u32,
                            ::rusteal_runtime::ffi::CPF_PARM | ::rusteal_runtime::ffi::CPF_OUT_PARM | ::rusteal_runtime::ffi::CPF_RETURN_PARM,
                            #extra_expr,
                        );
                    }
                });
            }
        }
    }

    for d in &delegates {
        register_stmts.push(udelegate::register_stmts(d));
    }

    let delegate_methods: Vec<TokenStream> = delegates
        .iter()
        .map(|d| udelegate::binding_methods(&struct_name, d))
        .collect();

    let add_interface = interface.as_ref().map(|trait_path| {
        quote! {
            let __interface = <dyn #trait_path>::__rusteal_interface_handle();

            if !__interface.is_null() {
                unsafe { ::rusteal_runtime::runtime::ffi_dispatch::reify_add_interface_class(cls, __interface); }
            }
        }
    });

    let register_functions_fn = quote! {
        #[doc(hidden)]
        pub fn #register_fns_name() {
            let cls = match #class_handle_name.get() {
                Some(&c) if !c.is_null() => c,
                _ => return,
            };

            #add_interface
            #(#register_stmts)*
        }
    };

    let (class_defaults_fn, class_defaults_entry) = match class_defaults {
        Some(method) => {
            let method_ident = &method.sig.ident;

            let fn_name = format_ident!(
                "__rusteal_class_defaults_{}",
                to_snake_case(&struct_name_str)
            );

            (
                quote! {
                    #[doc(hidden)]
                    pub fn #fn_name() {
                        let cls = match #class_handle_name.get() {
                            Some(&c) if !c.is_null() => c,
                            _ => return,
                        };

                        let cdo = unsafe { ::rusteal_runtime::runtime::ffi_dispatch::reify_get_cdo(cls) };

                        if cdo.is_null() {
                            return;
                        }

                        let rust_data = ::rusteal_runtime::runtime::reify_registry::get_instance_data(
                            cdo,
                            #struct_name::__RUSTEAL_TYPE_ID,
                        );

                        #[allow(unused_mut)]
                        let mut __this = #struct_name {
                            __obj: cdo,
                            __rust_data: rust_data as *mut #rust_data_name,
                        };

                        ::rusteal_runtime::runtime::reify_registry::ClassDefaultsOutcome::report(
                            __this.#method_ident(),
                            #struct_name_str,
                        );
                    }
                },
                quote! { Some(#fn_name) },
            )
        }
        None => (quote! {}, quote! { None }),
    };

    let shape_value = shape::hash(shape_parts);

    Ok(quote! {
        #clean_impl
        #(#delegate_methods)*
        #register_functions_fn
        #class_defaults_fn

        ::rusteal_runtime::__inventory::submit! {
            ::rusteal_runtime::runtime::reify_registry::ClassFunctionRegistration {
                type_id: #struct_name::__RUSTEAL_TYPE_ID,
                shape: #shape_value,
                register_functions: #register_fns_name,
                class_defaults: #class_defaults_entry,
            }
        }
    })
}

fn parse_ufunction(method: &ImplItemFn) -> syn::Result<UFunctionInfo> {
    let (specifiers, explicit_name) =
        if let Some(attr) = method.attrs.iter().find(|a| a.path().is_ident("ufunction")) {
            parse_ufunction_specifiers(attr)?
        } else {
            (Vec::new(), None)
        };

    let has = |name: &str| specifiers.iter().any(|s| s == name);

    const KNOWN: &[&str] = &[
        "BlueprintCallable",
        "BlueprintPure",
        "Override",
        "BlueprintImplementableEvent",
        "Exec",
    ];

    if let Some(unknown) = specifiers.iter().find(|s| !KNOWN.contains(&s.as_str())) {
        return Err(syn::Error::new_spanned(
            &method.sig,
            format!(
                "unknown #[ufunction] argument `{unknown}`: expected BlueprintCallable, \
                 BlueprintPure, Override, BlueprintImplementableEvent, Exec or name = \"...\""
            ),
        ));
    }

    let exec = has("Exec");

    let kind = match (
        has("Override"),
        has("BlueprintPure"),
        has("BlueprintImplementableEvent"),
    ) {
        (false, false, false) => FnKind::Callable,
        (true, false, false) => FnKind::Override,
        (false, true, false) => FnKind::Pure,
        (false, false, true) => FnKind::ImplementableEvent,
        _ => {
            return Err(syn::Error::new_spanned(
                &method.sig,
                "a #[ufunction] is one of Override, BlueprintPure and BlueprintImplementableEvent",
            ));
        }
    };

    let is_override = kind == FnKind::Override;

    let check_event_type = |ty: &Type| -> syn::Result<()> {
        if kind == FnKind::ImplementableEvent
            && matches!(
                prop_type::map_type(ty).map(|i| i.kind),
                Some(prop_type::PropKind::Struct { .. })
            )
        {
            return Err(syn::Error::new_spanned(
                ty,
                "a BlueprintImplementableEvent takes scalars, objects, classes and structs \
                 (&OwnedStruct<T>), and returns scalars, objects and classes",
            ));
        }

        Ok(())
    };

    if kind == FnKind::ImplementableEvent && !method.block.stmts.is_empty() {
        return Err(syn::Error::new_spanned(
            &method.block,
            "a BlueprintImplementableEvent has an empty body: its Blueprint child implements it, \
             and calling the method calls that",
        ));
    }

    let method_ident = method.sig.ident.clone();

    let ue_name = explicit_name
        .unwrap_or_else(|| prop_type::to_pascal_case(&method_ident.unraw().to_string()));

    let is_mut = method.sig.inputs.first().is_some_and(|arg| match arg {
        FnArg::Receiver(r) => {
            r.mutability.is_some() || matches!(r.kind, ReceiverKind::Reference(_, _, Some(_)))
        }
        FnArg::Typed(_) => false,
    });

    let mut params = Vec::new();

    for arg in &method.sig.inputs {
        match arg {
            FnArg::Receiver(_) => continue,
            FnArg::Typed(pat_type) => {
                let name = match &*pat_type.pat {
                    syn::Pat::Ident(pi) => pi.ident.clone(),
                    _ => {
                        return Err(syn::Error::new_spanned(
                            &pat_type.pat,
                            "ufunction params must be simple identifiers",
                        ));
                    }
                };

                let ty = (*pat_type.ty).clone();

                let event_struct =
                    kind == FnKind::ImplementableEvent && event_struct_param(&ty).is_some();

                if !is_override && !event_struct && !supported_param(&ty) {
                    return Err(syn::Error::new_spanned(
                        &ty,
                        "unsupported ufunction parameter type: supported are bool/i32/i64/u8/f32/f64, \
                         UObjectRef<T>, SubclassOf<T>, UStructRef<T> and UE enums",
                    ));
                }

                check_event_type(&ty)?;

                if is_override
                    && prop_type::map_type(&ty).is_some_and(|info| info.kind.property_only())
                {
                    return Err(syn::Error::new_spanned(
                        &ty,
                        "an Override takes a struct as UStructRef<T>, and a scalar out \
                         parameter as OutRef<T>",
                    ));
                }

                let ue_name = prop_type::to_ue_name(&name.unraw().to_string());

                params.push(ParamInfo {
                    rust_name: name,
                    ue_name,
                    rust_ty: ty,
                });
            }
        }
    }

    let return_type = match &method.sig.output {
        ReturnType::Default => None,
        ReturnType::Type(_, ty) => {
            if let Type::Tuple(tuple) = &**ty {
                if tuple.elems.is_empty() {
                    None
                } else {
                    return Err(syn::Error::new_spanned(
                        ty,
                        "tuple return types not supported",
                    ));
                }
            } else {
                check_event_type(ty)?;

                if !is_override && !supported_return(ty) {
                    return Err(syn::Error::new_spanned(
                        ty,
                        "unsupported ufunction return type: supported are bool/i32/i64/u8/f32/f64, \
                         UObjectRef<T>, SubclassOf<T> and UE enums",
                    ));
                }

                Some(ParamInfo {
                    rust_name: Ident::new("ReturnValue", proc_macro2::Span::call_site()),
                    ue_name: "ReturnValue".to_string(),
                    rust_ty: (**ty).clone(),
                })
            }
        }
    };

    let name_expr = quote! { #ue_name };

    let param_name_exprs = params
        .iter()
        .map(|param| {
            let name = &param.ue_name;
            quote! { #name }
        })
        .collect();

    Ok(UFunctionInfo {
        method_ident,
        ue_name,
        params,
        return_type,
        is_mut,
        kind,
        exec,
        name_expr,
        param_name_exprs,
    })
}

fn parse_interface_fn(method: &ImplItemFn, trait_path: &syn::Path) -> syn::Result<UFunctionInfo> {
    if let Some(attr) = method.attrs.iter().find(|a| {
        a.path().is_ident("ufunction")
            || a.path().is_ident("udelegate")
            || a.path().is_ident("class_defaults")
    }) {
        return Err(syn::Error::new_spanned(
            attr,
            "the methods of an interface implementation are the interface's functions, \
             with no #[ufunction], #[udelegate] or #[class_defaults]",
        ));
    }

    let signature = crate::uinterface::parse_signature(&method.sig)?;
    let method_ident = method.sig.ident.clone();
    let signature_fn = crate::uinterface::signature_fn(&method_ident);

    let name_expr = quote! { <dyn #trait_path>::#signature_fn().0 };

    let param_name_exprs = (0..signature.params.len())
        .map(|index| quote! { <dyn #trait_path>::#signature_fn().1[#index] })
        .collect();

    Ok(UFunctionInfo {
        method_ident,
        ue_name: signature.ue_name,
        params: signature.params,
        return_type: signature.return_type,
        is_mut: signature.is_mut,
        kind: FnKind::InterfaceImpl,
        exec: false,
        name_expr,
        param_name_exprs,
    })
}

pub(crate) fn supported_param(ty: &Type) -> bool {
    prop_type::map_type(ty).is_some_and(|info| {
        !matches!(info.kind, prop_type::PropKind::Array { .. }) && !info.kind.property_only()
    })
}

pub(crate) fn supported_return(ty: &Type) -> bool {
    prop_type::map_type(ty).is_some_and(|info| {
        !matches!(
            info.kind,
            prop_type::PropKind::Array { .. } | prop_type::PropKind::Struct { .. }
        ) && !info.kind.property_only()
    })
}

pub(crate) fn extra_expr(info: &prop_type::PropTypeInfo) -> TokenStream {
    match info.extra_fields() {
        Some(fields) => quote! {
            &::rusteal_runtime::ffi::RustealReifyPropExtra {
                #fields
                ..::core::default::Default::default()
            }
        },
        None => quote! { std::ptr::null() },
    }
}

fn event_struct_param(ty: &Type) -> Option<Type> {
    let Type::Reference(reference) = ty else {
        return None;
    };

    let inner = (*reference.elem).clone();

    matches!(
        prop_type::map_type(&inner).map(|i| i.kind),
        Some(prop_type::PropKind::OwnedStruct { .. })
    )
    .then_some(inner)
}

fn parse_ufunction_specifiers(attr: &syn::Attribute) -> syn::Result<(Vec<String>, Option<String>)> {
    let mut specifiers = Vec::new();
    let mut name = None;

    if let Meta::Path(_) = attr.meta {
        return Ok((specifiers, name));
    }

    let nested = attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;

    for meta in &nested {
        match meta {
            Meta::Path(p) => {
                if let Some(ident) = p.get_ident() {
                    specifiers.push(ident.to_string());
                }
            }
            Meta::NameValue(nv) if nv.path.is_ident("name") => match &nv.value {
                syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(s),
                    ..
                }) => name = Some(s.value()),
                other => {
                    return Err(syn::Error::new_spanned(
                        other,
                        "`name` is a string: name = \"K2_Foo\"",
                    ));
                }
            },
            other => {
                return Err(syn::Error::new_spanned(
                    other,
                    "unknown #[ufunction] argument",
                ));
            }
        }
    }

    Ok((specifiers, name))
}

fn implementable_event_body(uf: &UFunctionInfo) -> syn::Block {
    let ue_name = &uf.ue_name;

    dynamic_call_body(
        quote! { ::rusteal_runtime::runtime::DynamicCall::new(&self.as_ref(), #ue_name) },
        ue_name,
        &uf.params,
        uf.return_type.as_ref().map(|ret| &ret.rust_ty),
    )
}

pub(crate) fn dynamic_call_body(
    new_call: TokenStream,
    ue_name: &str,
    params: &[ParamInfo],
    return_type: Option<&Type>,
) -> syn::Block {
    let sets: Vec<TokenStream> = params
        .iter()
        .map(|p| {
            let rust_name = &p.rust_name;
            let param = &p.ue_name;

            if event_struct_param(&p.rust_ty).is_some() {
                return quote! { call.set_struct(#param, #rust_name)?; };
            }

            let kind = prop_type::map_type(&p.rust_ty).map(|info| info.kind);

            if matches!(kind, Some(prop_type::PropKind::Struct { .. })) {
                return quote! { call.set_struct_ref(#param, &#rust_name)?; };
            }

            let value = match kind {
                Some(prop_type::PropKind::Object { .. }) => quote! { #rust_name.raw() },
                Some(prop_type::PropKind::Class { .. }) => {
                    quote! { ::rusteal_runtime::ffi::UObjectHandle(#rust_name.raw().0) }
                }
                _ => quote! { #rust_name },
            };

            quote! { call.set(#param, #value)?; }
        })
        .collect();

    let (ret_ty, read, fallback) = match return_type {
        None => (
            quote! { () },
            quote! { let _ = result; Ok(()) },
            TokenStream::new(),
        ),
        Some(ty) => match prop_type::map_type(ty).map(|info| info.kind) {
            Some(prop_type::PropKind::Object { .. }) => (
                quote! { #ty },
                quote! {
                    let h = result.get::<::rusteal_runtime::ffi::UObjectHandle>("ReturnValue")?;
                    Ok(unsafe { <#ty>::from_raw(h) })
                },
                quote! { unsafe { <#ty>::from_raw(::rusteal_runtime::ffi::UObjectHandle::null()) } },
            ),
            _ => (
                quote! { #ty },
                quote! { result.get::<#ty>("ReturnValue") },
                quote! { <#ty as ::core::default::Default>::default() },
            ),
        },
    };

    syn::parse_quote! {{
        let __call = || -> ::rusteal_runtime::runtime::RustealResult<#ret_ty> {
            let mut call = #new_call?;

            #(#sets)*
            let result = call.call()?;

            #read
        };

        match __call() {
            Ok(value) => value,
            Err(e) => {
                ::rusteal_runtime::runtime::ulog!(
                    ::rusteal_runtime::runtime::LOG_WARNING,
                    "[Rusteal] {} failed: {}", #ue_name, e
                );

                #fallback
            }
        }
    }}
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expand(item: TokenStream) -> syn::Result<String> {
        expand_uclass_impl(TokenStream::new(), item).map(|tokens| tokens.to_string())
    }

    #[test]
    fn an_interface_implementation_registers_the_interface_and_its_functions() {
        let out = expand(quote! {
            impl Interactable for Door {
                fn interact(&mut self, instigator: UObjectRef<Actor>) -> bool {
                    true
                }
            }
        })
        .unwrap();

        assert!(out.contains("reify_add_interface_class"));
        assert!(out.contains("< dyn Interactable > :: __rusteal_signature_interact () . 0"));
        assert!(
            out.contains("< dyn Interactable > :: __rusteal_signature_interact () . 1 [0usize]")
        );
        assert!(out.contains("Interactable :: interact (& mut __this , instigator)"));
        assert!(out.contains("FUNC_EVENT"));
        assert!(out.contains("__rusteal_register_door_interactable_functions"));
    }

    #[test]
    fn an_interface_implementation_holds_plain_functions() {
        assert!(
            expand(quote! {
                impl Interactable for Door {
                    #[ufunction]
                    fn interact(&mut self) {}
                }
            })
            .is_err()
        );

        assert!(
            expand(quote! {
                impl Interactable for Door {
                    const X: i32 = 1;
                }
            })
            .is_err()
        );

        assert!(
            expand(quote! {
                impl Interactable for Door {
                    fn interact(&mut self, name: String) {}
                }
            })
            .is_err()
        );
    }
}
