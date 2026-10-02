// #[uclass_impl] macro: generates function registration for #[ufunction] methods
// on a Rust-defined UE class.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::ext::IdentExt;
use syn::{parse2, FnArg, Ident, ImplItem, ImplItemFn, ItemImpl, Meta, ReceiverKind, ReturnType, Token, Type};
use syn::punctuated::Punctuated;

use crate::prop_type;
use crate::uclass::{to_snake_case, to_screaming_snake};

// ---------------------------------------------------------------------------
// Parsed ufunction info
// ---------------------------------------------------------------------------

struct ParamInfo {
    rust_name: Ident,
    ue_name: String,
    rust_ty: Type,
}

/// What a `#[ufunction]` is to UE.
#[derive(Clone, Copy, PartialEq, Eq)]
enum FnKind {
    /// `BlueprintCallable` (the default): Rust code Blueprints can call.
    Callable,
    /// `BlueprintPure`: callable, without execution pins.
    Pure,
    /// `Override`: Rust code for an engine event (`ReceiveBeginPlay`).
    Override,
    /// `BlueprintImplementableEvent`: an event a Blueprint child implements
    /// and Rust calls; the method's body is left empty and becomes the call.
    ImplementableEvent,
}

struct UFunctionInfo {
    method_ident: Ident,
    ue_name: String,
    params: Vec<ParamInfo>,
    return_type: Option<ParamInfo>,
    is_mut: bool,
    kind: FnKind,
}

// ---------------------------------------------------------------------------
// Main expansion
// ---------------------------------------------------------------------------

pub fn expand_uclass_impl(_attr: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let input: ItemImpl = parse2(item)?;

    // Extract struct name from impl target
    let struct_name = match &*input.self_ty {
        Type::Path(tp) => tp.path.segments.last()
            .ok_or_else(|| syn::Error::new_spanned(&input.self_ty, "expected type name"))?
            .ident.clone(),
        _ => return Err(syn::Error::new_spanned(&input.self_ty, "expected type name")),
    };

    let struct_name_str = struct_name.to_string();
    let rust_data_name = format_ident!("__{}RustData", struct_name);
    let class_handle_name = format_ident!("__RUSTEAL_CLASS_HANDLE_{}", to_screaming_snake(&struct_name_str));

    // Classify methods: collect #[ufunction] info and the #[class_defaults]
    // method, strip attrs
    let mut ufunctions: Vec<UFunctionInfo> = Vec::new();
    let mut class_defaults: Option<&ImplItemFn> = None;
    let mut clean_impl = input.clone();

    for item in &input.items {
        if let ImplItem::Fn(method) = item {
            let has_ufunction = method.attrs.iter().any(|a| a.path().is_ident("ufunction"));
            if has_ufunction {
                ufunctions.push(parse_ufunction(method)?);
            }
            if method.attrs.iter().any(|a| a.path().is_ident("class_defaults")) {
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

    // Strip #[ufunction] and #[class_defaults] attrs from the emitted impl
    // block; an implementable event's empty body becomes the call into UE.
    for item in &mut clean_impl.items {
        if let ImplItem::Fn(method) = item {
            if let Some(uf) = ufunctions.iter().find(|uf| uf.method_ident == method.sig.ident)
                && uf.kind == FnKind::ImplementableEvent
            {
                method.block = implementable_event_body(uf);
            }
            method
                .attrs
                .retain(|a| !a.path().is_ident("ufunction") && !a.path().is_ident("class_defaults"));
        }
    }

    // Generate register_functions body
    let register_fns_name = format_ident!(
        "__rusteal_register_{}_functions",
        to_snake_case(&struct_name_str),
    );

    let mut register_stmts: Vec<TokenStream> = Vec::new();

    for uf in &ufunctions {
        let ue_name = &uf.ue_name;
        let ue_name_bytes = ue_name.as_bytes();
        let ue_name_len = ue_name.len() as u32;
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
            // No native code: a script event, as UHT makes a
            // BlueprintImplementableEvent; the Blueprint child's graph is
            // its body.
            FnKind::ImplementableEvent => quote! {
                ::rusteal_runtime::ffi::FUNC_EVENT | ::rusteal_runtime::ffi::FUNC_BLUEPRINT_EVENT
                    | ::rusteal_runtime::ffi::FUNC_BLUEPRINT_CALLABLE | ::rusteal_runtime::ffi::FUNC_PUBLIC
            },
        };

        // Total number of offsets to cache (params + optional return)
        let total_offsets = uf.params.len() + if uf.return_type.is_some() { 1 } else { 0 };

        // Generate offset init expressions
        let mut offset_inits: Vec<TokenStream> = Vec::new();
        for param in &uf.params {
            let param_ue_name = &param.ue_name;
            let param_ue_bytes = param_ue_name.as_bytes();
            let param_ue_len = param_ue_name.len() as u32;
            offset_inits.push(quote! {
                {
                    let p = ::rusteal_runtime::runtime::ffi_dispatch::reflection_get_function_param(
                        func,
                        [#(#param_ue_bytes),*].as_ptr(),
                        #param_ue_len,
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

        // Generate param reads from the params buffer (via native_mem_read).
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
                // A scalar out parameter, written in place.
                _ if is_out_ref => quote! {
                    ::rusteal_runtime::runtime::out_ref_from_param(params, offsets[#idx] as usize)
                },
                // A typed reference to the struct inside the params buffer.
                Some(prop_type::PropKind::Struct { .. }) => quote! {
                    ::rusteal_runtime::runtime::struct_ref_from_param(params, offsets[#idx] as usize)
                },
                // Objects and classes travel as pointers; the parameter's type
                // only admits `T` (and its subclasses).
                Some(prop_type::PropKind::Object { .. }) => quote! {
                    <#rust_ty>::from_raw(#read_handle)
                },
                Some(prop_type::PropKind::Class { .. }) => quote! {
                    <#rust_ty>::from_raw(::rusteal_runtime::ffi::UClassHandle(#read_handle.0))
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

        // Generate return value zero-init (before user call) and write (after).
        // Zero-init ensures C++ reads 0/false instead of garbage if the user
        // method panics and ffi_boundary catches the unwind.
        let (return_zero_init, return_write) = if let Some(ref ret) = uf.return_type {
            let ret_ty = &ret.rust_ty;
            let ret_idx = syn::Index::from(uf.params.len());
            let write = |value: TokenStream| quote! {
                unsafe {
                    ::rusteal_runtime::runtime::ffi_dispatch::native_mem_write(
                        params, offsets[#ret_idx] as usize, #value,
                    );
                }
            };
            match prop_type::map_type(ret_ty).map(|info| info.kind) {
                // Objects and classes are written as pointers.
                Some(prop_type::PropKind::Object { .. }) => (
                    write(quote! { ::rusteal_runtime::ffi::UObjectHandle::null() }),
                    write(quote! { __ret.raw() }),
                ),
                Some(prop_type::PropKind::Class { .. }) => (
                    write(quote! { ::rusteal_runtime::ffi::UObjectHandle::null() }),
                    write(quote! { ::rusteal_runtime::ffi::UObjectHandle(__ret.raw().0) }),
                ),
                _ => (
                    write(quote! { unsafe { std::mem::zeroed::<#ret_ty>() } }),
                    write(quote! { __ret }),
                ),
            }
        } else {
            (quote! {}, quote! {})
        };

        // Construct this binding
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

        // Method call expression
        let call_expr = if uf.return_type.is_some() {
            quote! { let __ret = __this.#method_ident(#(#param_idents),*); }
        } else {
            quote! { __this.#method_ident(#(#param_idents),*); }
        };

        // Register callback and add function + params
        let func_var = format_ident!("__func_{}", method_ident);

        if uf.kind == FnKind::ImplementableEvent {
            register_stmts.push(quote! {
                let #func_var = unsafe {
                    ::rusteal_runtime::runtime::ffi_dispatch::reify_add_function(
                        cls,
                        [#(#ue_name_bytes),*].as_ptr(),
                        #ue_name_len,
                        0u64,
                        #flags_expr,
                    )
                };
            });
        } else {
        register_stmts.push(quote! {
            let __callback_id = {
                let callback_id = ::rusteal_runtime::runtime::reify_registry::register_function(
                    move |obj: ::rusteal_runtime::ffi::UObjectHandle, rust_data: *mut u8, params: ::rusteal_runtime::runtime::ffi_dispatch::NativePtr| {
                        static OFFSETS: std::sync::OnceLock<[u32; #total_offsets]> = std::sync::OnceLock::new();
                        let offsets = OFFSETS.get_or_init(|| unsafe {
                            let cls = <#struct_name as ::rusteal_runtime::runtime::UeClass>::static_class();
                            let func = ::rusteal_runtime::runtime::ffi_dispatch::reflection_find_function_by_class(
                                cls,
                                [#(#ue_name_bytes),*].as_ptr(),
                                #ue_name_len,
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
                    [#(#ue_name_bytes),*].as_ptr(),
                    #ue_name_len,
                    __callback_id,
                    #flags_expr,
                )
            };
        });

        }

        // Add function params — skip for Override (C++ copies from parent function)
        if uf.kind != FnKind::Override {
            for param in &uf.params {
                let info = prop_type::map_type(&param.rust_ty).unwrap();
                let param_ue_name = &param.ue_name;
                let param_ue_bytes = param_ue_name.as_bytes();
                let param_ue_len = param_ue_name.len() as u32;
                let prop_type_expr = &info.prop_type_expr;
                let extra_expr = extra_expr(&info);

                register_stmts.push(quote! {
                    unsafe {
                        ::rusteal_runtime::runtime::ffi_dispatch::reify_add_function_param(
                            #func_var,
                            [#(#param_ue_bytes),*].as_ptr(),
                            #param_ue_len,
                            #prop_type_expr as u32,
                            ::rusteal_runtime::ffi::CPF_PARM,
                            #extra_expr,
                        );
                    }
                });
            }

            // Add return param if any
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

    let register_functions_fn = quote! {
        #[doc(hidden)]
        pub fn #register_fns_name() {
            let cls = match #class_handle_name.get() {
                Some(&c) if !c.is_null() => c,
                _ => return,
            };
            #(#register_stmts)*
        }
    };

    // The #[class_defaults] method, run on the class default object once the
    // class is finalized: what a C++ constructor sets on inherited properties.
    let (class_defaults_fn, class_defaults_entry) = match class_defaults {
        Some(method) => {
            let method_ident = &method.sig.ident;
            let fn_name = format_ident!("__rusteal_class_defaults_{}", to_snake_case(&struct_name_str));
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
                        let rust_data = ::rusteal_runtime::runtime::reify_registry::get_instance_data(cdo);
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

    Ok(quote! {
        #clean_impl
        #register_functions_fn
        #class_defaults_fn

        ::rusteal_runtime::__inventory::submit! {
            ::rusteal_runtime::runtime::reify_registry::ClassFunctionRegistration {
                register_functions: #register_fns_name,
                class_defaults: #class_defaults_entry,
            }
        }
    })
}

// ---------------------------------------------------------------------------
// Parsing helpers
// ---------------------------------------------------------------------------

fn parse_ufunction(method: &ImplItemFn) -> syn::Result<UFunctionInfo> {
    let (specifiers, explicit_name) =
        if let Some(attr) = method.attrs.iter().find(|a| a.path().is_ident("ufunction")) {
            parse_ufunction_specifiers(attr)?
        } else {
            (Vec::new(), None)
        };
    let has = |name: &str| specifiers.iter().any(|s| s == name);
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
            && matches!(prop_type::map_type(ty).map(|i| i.kind), Some(prop_type::PropKind::Struct { .. }))
        {
            return Err(syn::Error::new_spanned(
                ty,
                "a BlueprintImplementableEvent takes and returns scalars, objects and classes",
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
    // `fn r#move` is `Move`, as the template names its handler; `name = "..."`
    // gives a name no Rust identifier maps to (`K2_OnMovementModeChanged`).
    let ue_name = explicit_name
        .unwrap_or_else(|| prop_type::to_pascal_case(&method_ident.unraw().to_string()));

    // Check for self receiver and its mutability. syn 3 splits `&mut self`
    // (ReceiverKind::Reference) from `mut self` (Receiver::mutability).
    let is_mut = method.sig.inputs.first().is_some_and(|arg| match arg {
        FnArg::Receiver(r) => {
            r.mutability.is_some() || matches!(r.kind, ReceiverKind::Reference(_, _, Some(_)))
        }
        FnArg::Typed(_) => false,
    });

    // Parse params (skip self)
    let mut params = Vec::new();
    for arg in &method.sig.inputs {
        match arg {
            FnArg::Receiver(_) => continue,
            FnArg::Typed(pat_type) => {
                let name = match &*pat_type.pat {
                    syn::Pat::Ident(pi) => pi.ident.clone(),
                    _ => return Err(syn::Error::new_spanned(
                        &pat_type.pat,
                        "ufunction params must be simple identifiers",
                    )),
                };
                let ty = (*pat_type.ty).clone();
                // Override functions get their param types from the parent UFunction
                // (C++ copies them), so any Copy+repr(C) type is valid.
                // Non-override functions must use types known to reify_add_function_param.
                if !is_override && !supported_param(&ty) {
                    return Err(syn::Error::new_spanned(
                        &ty,
                        "unsupported ufunction parameter type: supported are bool/i32/i64/u8/f32/f64, \
                         UObjectRef<T>, SubclassOf<T> and UStructRef<T>",
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
                params.push(ParamInfo { rust_name: name, ue_name, rust_ty: ty });
            }
        }
    }

    // Parse return type
    let return_type = match &method.sig.output {
        ReturnType::Default => None,
        ReturnType::Type(_, ty) => {
            if let Type::Tuple(tuple) = &**ty {
                if tuple.elems.is_empty() {
                    None
                } else {
                    return Err(syn::Error::new_spanned(ty, "tuple return types not supported"));
                }
            } else {
                check_event_type(ty)?;
                if !is_override && !supported_return(ty) {
                    return Err(syn::Error::new_spanned(
                        ty,
                        "unsupported ufunction return type: supported are bool/i32/i64/u8/f32/f64, \
                         UObjectRef<T> and SubclassOf<T>",
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

    Ok(UFunctionInfo {
        method_ident,
        ue_name,
        params,
        return_type,
        is_mut,
        kind,
    })
}

/// Parameters: scalars, objects, classes and structs (by reference into the
/// call's parameters). Arrays are not supported.
fn supported_param(ty: &Type) -> bool {
    prop_type::map_type(ty).is_some_and(|info| {
        !matches!(info.kind, prop_type::PropKind::Array { .. } | prop_type::PropKind::Enum { .. })
            && !info.kind.property_only()
    })
}

/// Returns: scalars, objects and classes. A struct cannot be returned by reference.
fn supported_return(ty: &Type) -> bool {
    prop_type::map_type(ty).is_some_and(|info| {
        !matches!(
            info.kind,
            prop_type::PropKind::Array { .. } | prop_type::PropKind::Struct { .. } | prop_type::PropKind::Enum { .. }
        ) && !info.kind.property_only()
    })
}

/// The `RustealReifyPropExtra` argument for a parameter of this type.
fn extra_expr(info: &prop_type::PropTypeInfo) -> TokenStream {
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

/// The bare specifiers of `#[ufunction(...)]` and its `name = "..."`, if any.
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
                syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(s), .. }) => name = Some(s.value()),
                other => {
                    return Err(syn::Error::new_spanned(other, "`name` is a string: name = \"K2_Foo\""));
                }
            },
            other => return Err(syn::Error::new_spanned(other, "unknown #[ufunction] argument")),
        }
    }
    Ok((specifiers, name))
}

/// The body of a BlueprintImplementableEvent method: call the event by name
/// on the object, which runs its Blueprint child's graph (nothing when the
/// child does not implement it). A failed call is logged; a return value is
/// then zero.
fn implementable_event_body(uf: &UFunctionInfo) -> syn::Block {
    let ue_name = &uf.ue_name;
    let sets: Vec<TokenStream> = uf
        .params
        .iter()
        .map(|p| {
            let rust_name = &p.rust_name;
            let param = &p.ue_name;
            let value = match prop_type::map_type(&p.rust_ty).map(|info| info.kind) {
                Some(prop_type::PropKind::Object { .. }) => quote! { #rust_name.raw() },
                Some(prop_type::PropKind::Class { .. }) => {
                    quote! { ::rusteal_runtime::ffi::UObjectHandle(#rust_name.raw().0) }
                }
                _ => quote! { #rust_name },
            };
            quote! { call.set(#param, #value)?; }
        })
        .collect();
    let (ret_ty, read, fallback) = match &uf.return_type {
        None => (quote! { () }, quote! { let _ = result; Ok(()) }, quote! { () }),
        Some(ret) => {
            let ty = &ret.rust_ty;
            match prop_type::map_type(ty).map(|info| info.kind) {
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
            }
        }
    };
    syn::parse_quote! {{
        let __call = || -> ::rusteal_runtime::runtime::RustealResult<#ret_ty> {
            let mut call = ::rusteal_runtime::runtime::DynamicCall::new(&self.as_ref(), #ue_name)?;
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
