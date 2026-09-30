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

struct UFunctionInfo {
    method_ident: Ident,
    ue_name: String,
    params: Vec<ParamInfo>,
    return_type: Option<ParamInfo>,
    is_mut: bool,
    is_override: bool,
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

    // Classify methods: collect #[ufunction] info, strip attrs
    let mut ufunctions: Vec<UFunctionInfo> = Vec::new();
    let mut clean_impl = input.clone();

    for item in &input.items {
        if let ImplItem::Fn(method) = item {
            let has_ufunction = method.attrs.iter().any(|a| a.path().is_ident("ufunction"));
            if has_ufunction {
                ufunctions.push(parse_ufunction(method)?);
            }
        }
    }

    // Strip #[ufunction] attrs from the emitted impl block
    for item in &mut clean_impl.items {
        if let ImplItem::Fn(method) = item {
            method.attrs.retain(|a| !a.path().is_ident("ufunction"));
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

        let flags_expr = if uf.is_override {
            quote! {
                ::rusteal_runtime::ffi::FUNC_NATIVE | ::rusteal_runtime::ffi::FUNC_BLUEPRINT_EVENT | ::rusteal_runtime::ffi::FUNC_PUBLIC
            }
        } else {
            quote! {
                ::rusteal_runtime::ffi::FUNC_NATIVE | ::rusteal_runtime::ffi::FUNC_BLUEPRINT_CALLABLE | ::rusteal_runtime::ffi::FUNC_PUBLIC
            }
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
            let read = match prop_type::map_type(rust_ty).map(|info| info.kind) {
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

        // Add function params — skip for Override (C++ copies from parent function)
        if !uf.is_override {
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

    Ok(quote! {
        #clean_impl
        #register_functions_fn

        ::rusteal_runtime::__inventory::submit! {
            ::rusteal_runtime::runtime::reify_registry::ClassFunctionRegistration {
                register_functions: #register_fns_name,
            }
        }
    })
}

// ---------------------------------------------------------------------------
// Parsing helpers
// ---------------------------------------------------------------------------

fn parse_ufunction(method: &ImplItemFn) -> syn::Result<UFunctionInfo> {
    let specifiers = if let Some(attr) = method.attrs.iter().find(|a| a.path().is_ident("ufunction")) {
        parse_ufunction_specifiers(attr)?
    } else {
        Vec::new()
    };
    let is_override = specifiers.iter().any(|s| s == "Override");

    let method_ident = method.sig.ident.clone();
    // `fn r#move` is `Move`, as the template names its handler.
    let ue_name = prop_type::to_pascal_case(&method_ident.unraw().to_string());

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
                let ue_name = prop_type::to_pascal_case(&name.unraw().to_string());
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
        is_override,
    })
}

/// Parameters: scalars, objects, classes and structs (by reference into the
/// call's parameters). Arrays are not supported.
fn supported_param(ty: &Type) -> bool {
    prop_type::map_type(ty).is_some_and(|info| !matches!(info.kind, prop_type::PropKind::Array { .. }))
}

/// Returns: scalars, objects and classes. A struct cannot be returned by reference.
fn supported_return(ty: &Type) -> bool {
    prop_type::map_type(ty).is_some_and(|info| {
        !matches!(info.kind, prop_type::PropKind::Array { .. } | prop_type::PropKind::Struct { .. })
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

fn parse_ufunction_specifiers(attr: &syn::Attribute) -> syn::Result<Vec<String>> {
    let mut specifiers = Vec::new();
    if let Ok(nested) = attr.parse_args_with(
        Punctuated::<Meta, Token![,]>::parse_terminated,
    ) {
        for meta in &nested {
            if let Meta::Path(p) = meta
                && let Some(ident) = p.get_ident()
            {
                specifiers.push(ident.to_string());
            }
        }
    }
    Ok(specifiers)
}
