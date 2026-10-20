use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::ext::IdentExt;
use syn::{FnArg, Ident, ItemTrait, ReceiverKind, ReturnType, Signature, TraitItem, Type, parse2};

use crate::prop_type;
use crate::shape;
use crate::uclass::{to_screaming_snake, to_snake_case};
use crate::uclass_impl::{
    ParamInfo, dynamic_call_body, extra_expr, legacy_param as supported_param,
    legacy_return as supported_return,
};

pub(crate) struct InterfaceSignature {
    pub ue_name: String,
    pub params: Vec<ParamInfo>,
    pub return_type: Option<ParamInfo>,
    pub is_mut: bool,
}

pub(crate) fn signature_fn(method: &Ident) -> Ident {
    format_ident!("__rusteal_signature_{}", method.unraw())
}

pub(crate) fn parse_signature(sig: &Signature) -> syn::Result<InterfaceSignature> {
    if !sig.generics.params.is_empty() || sig.generics.where_clause.is_some() {
        return Err(syn::Error::new_spanned(
            &sig.generics,
            "an interface function takes no generic parameters",
        ));
    }

    if let Some(token) = &sig.asyncness {
        return Err(syn::Error::new_spanned(
            token,
            "an interface function is not async",
        ));
    }

    let is_mut = match sig.inputs.first() {
        Some(FnArg::Receiver(receiver)) => match &receiver.kind {
            ReceiverKind::Reference(_, _, mutability) => mutability.is_some(),
            _ => {
                return Err(syn::Error::new_spanned(
                    receiver,
                    "an interface function takes `&self` or `&mut self` first",
                ));
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                sig,
                "an interface function takes `&self` or `&mut self` first",
            ));
        }
    };

    let mut params = Vec::new();

    for arg in sig.inputs.iter().skip(1) {
        let FnArg::Typed(pat_type) = arg else {
            continue;
        };

        let syn::Pat::Ident(pat) = &*pat_type.pat else {
            return Err(syn::Error::new_spanned(
                &pat_type.pat,
                "interface function parameters are simple identifiers",
            ));
        };

        let ty = (*pat_type.ty).clone();

        if !supported_param(&ty) {
            return Err(syn::Error::new_spanned(
                &ty,
                "unsupported interface parameter type: supported are bool/i32/i64/u8/f32/f64, \
                 UObjectRef<T>, SubclassOf<T>, UStructRef<T> and UE enums",
            ));
        }

        params.push(ParamInfo {
            rust_name: pat.ident.clone(),
            ue_name: prop_type::to_ue_name(&pat.ident.unraw().to_string()),
            rust_ty: ty,
        });
    }

    let return_type = match &sig.output {
        ReturnType::Default => None,
        ReturnType::Type(_, ty) if matches!(&**ty, Type::Tuple(t) if t.elems.is_empty()) => None,
        ReturnType::Type(_, ty) => {
            if !supported_return(ty) {
                return Err(syn::Error::new_spanned(
                    ty,
                    "unsupported interface return type: supported are bool/i32/i64/u8/f32/f64, \
                     UObjectRef<T>, SubclassOf<T> and UE enums",
                ));
            }

            Some(ParamInfo {
                rust_name: Ident::new("ReturnValue", proc_macro2::Span::call_site()),
                ue_name: "ReturnValue".to_string(),
                rust_ty: (**ty).clone(),
            })
        }
    };

    Ok(InterfaceSignature {
        ue_name: prop_type::to_pascal_case(&sig.ident.unraw().to_string()),
        params,
        return_type,
        is_mut,
    })
}

pub fn expand_uinterface(attr: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    if !attr.is_empty() {
        return Err(syn::Error::new_spanned(
            attr,
            "#[uinterface] takes no arguments",
        ));
    }

    let input: ItemTrait = parse2(item)?;

    if !input.generics.params.is_empty() || input.generics.where_clause.is_some() {
        return Err(syn::Error::new_spanned(
            &input.generics,
            "a #[uinterface] trait takes no generic parameters",
        ));
    }

    if !input.supertraits.is_empty() {
        return Err(syn::Error::new_spanned(
            &input.supertraits,
            "a #[uinterface] trait has no supertraits",
        ));
    }

    if let Some(token) = &input.unsafety {
        return Err(syn::Error::new_spanned(
            token,
            "a #[uinterface] trait is not unsafe",
        ));
    }

    let name = &input.ident;
    let name_str = name.unraw().to_string();
    let snake = to_snake_case(&name_str);

    let mut functions = Vec::new();

    for item in &input.items {
        let TraitItem::Fn(method) = item else {
            return Err(syn::Error::new_spanned(
                item,
                "a #[uinterface] trait holds functions only",
            ));
        };

        if let Some(body) = &method.default {
            return Err(syn::Error::new_spanned(
                body,
                "an interface function has no body: each class implementing the interface \
                 writes its own",
            ));
        }

        functions.push((method, parse_signature(&method.sig)?));
    }

    let handle = format_ident!(
        "__RUSTEAL_INTERFACE_HANDLE_{}",
        to_screaming_snake(&name_str)
    );
    let create_fn = format_ident!("__rusteal_create_interface_{}", snake);
    let register_fn = format_ident!("__rusteal_register_interface_{}", snake);
    let finalize_fn = format_ident!("__rusteal_finalize_interface_{}", snake);

    let shape_value = shape::hash(
        std::iter::once(format!("uinterface {name_str}")).chain(
            functions
                .iter()
                .map(|(method, _)| shape::declaration(&[], &[], &method.sig)),
        ),
    );

    let signature_fns: Vec<TokenStream> = functions
        .iter()
        .map(|(method, signature)| {
            let ident = signature_fn(&method.sig.ident);
            let ue_name = &signature.ue_name;
            let params = signature.params.iter().map(|p| &p.ue_name);

            quote! {
                #[doc(hidden)]
                pub fn #ident() -> (&'static str, &'static [&'static str]) {
                    (#ue_name, &[#(#params),*])
                }
            }
        })
        .collect();

    let dispatch_fns: Vec<TokenStream> = functions
        .iter()
        .map(|(method, signature)| {
            let sig = &method.sig;
            let ue_name = &signature.ue_name;

            let body = dynamic_call_body(
                quote! { self.call(#ue_name) },
                ue_name,
                &signature.params,
                signature.return_type.as_ref().map(|ret| &ret.rust_ty),
            );

            quote! { #sig #body }
        })
        .collect();

    let register_stmts: Vec<TokenStream> = functions
        .iter()
        .map(|(_, signature)| {
            let ue_name = &signature.ue_name;

            let params: Vec<TokenStream> = signature
                .params
                .iter()
                .map(|param| (param, quote! { ::rusteal_runtime::ffi::CPF_PARM }))
                .chain(signature.return_type.iter().map(|ret| {
                    (
                        ret,
                        quote! {
                            ::rusteal_runtime::ffi::CPF_PARM | ::rusteal_runtime::ffi::CPF_OUT_PARM
                                | ::rusteal_runtime::ffi::CPF_RETURN_PARM
                        },
                    )
                }))
                .map(|(param, flags)| {
                    let info = prop_type::map_type(&param.rust_ty).unwrap();
                    let param_name = &param.ue_name;
                    let prop_type_expr = &info.prop_type_expr;
                    let extra = extra_expr(&info);

                    quote! {
                        unsafe {
                            ::rusteal_runtime::runtime::ffi_dispatch::reify_add_function_param(
                                func,
                                #param_name.as_ptr(),
                                #param_name.len() as u32,
                                #prop_type_expr as u32,
                                #flags,
                                #extra,
                            );
                        }
                    }
                })
                .collect();

            quote! {
                {
                    let func = unsafe {
                        ::rusteal_runtime::runtime::ffi_dispatch::reify_add_function(
                            interface,
                            #ue_name.as_ptr(),
                            #ue_name.len() as u32,
                            0u64,
                            ::rusteal_runtime::ffi::FUNC_EVENT | ::rusteal_runtime::ffi::FUNC_BLUEPRINT_EVENT
                                | ::rusteal_runtime::ffi::FUNC_BLUEPRINT_CALLABLE | ::rusteal_runtime::ffi::FUNC_PUBLIC,
                        )
                    };

                    if !func.is_null() {
                        #(#params)*
                    }
                }
            }
        })
        .collect();

    Ok(quote! {
        #input

        #[doc(hidden)]
        static #handle: std::sync::OnceLock<::rusteal_runtime::ffi::UClassHandle> = std::sync::OnceLock::new();

        impl ::rusteal_runtime::runtime::UeInterface for dyn #name {
            fn static_class() -> ::rusteal_runtime::ffi::UClassHandle {
                *#handle.get().expect(concat!("Failed to find the UE interface for ", stringify!(#name), " — was it registered?"))
            }
        }

        impl dyn #name {
            #[doc(hidden)]
            pub fn __rusteal_interface_handle() -> ::rusteal_runtime::ffi::UClassHandle {
                #handle.get().copied().unwrap_or(::rusteal_runtime::ffi::UClassHandle::null())
            }

            #(#signature_fns)*
        }

        impl #name for ::rusteal_runtime::runtime::ScriptInterface<dyn #name> {
            #(#dispatch_fns)*
        }

        #[doc(hidden)]
        pub fn #create_fn() {
            let interface = unsafe {
                ::rusteal_runtime::runtime::ffi_dispatch::reify_create_interface(
                    #name_str.as_ptr(),
                    #name_str.len() as u32,
                    #shape_value,
                )
            };

            if interface.is_null() {
                let msg = concat!("[Rusteal] ", stringify!(#name), ": create_interface failed");
                let bytes = msg.as_bytes();
                unsafe { ::rusteal_runtime::runtime::ffi_dispatch::logging_log(2, bytes.as_ptr(), bytes.len() as u32); }
                return;
            }

            #handle.set(interface).ok();
        }

        #[doc(hidden)]
        pub fn #register_fn() {
            let interface = match #handle.get() {
                Some(&i) if !i.is_null() => i,
                _ => return,
            };

            #(#register_stmts)*
        }

        #[doc(hidden)]
        pub fn #finalize_fn() {
            if let Some(&interface) = #handle.get()
                && !interface.is_null()
            {
                unsafe { ::rusteal_runtime::runtime::ffi_dispatch::reify_finalize_interface(interface); }
            }
        }

        ::rusteal_runtime::__inventory::submit! {
            ::rusteal_runtime::runtime::reify_registry::InterfaceRegistration {
                krate: env!("CARGO_CRATE_NAME"),
                name: #name_str,
                create: #create_fn,
                register: #register_fn,
                finalize: #finalize_fn,
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expand(item: TokenStream) -> syn::Result<String> {
        expand_uinterface(TokenStream::new(), item).map(|tokens| tokens.to_string())
    }

    #[test]
    fn functions_become_blueprint_events_of_the_interface() {
        let out = expand(quote! {
            pub trait Interactable {
                fn interact(&mut self, instigator: UObjectRef<Actor>) -> bool;
                fn highlight(&self, on: bool);
            }
        })
        .unwrap();

        assert!(out.contains("reify_create_interface"));
        assert!(out.contains("\"Interactable\""));
        assert!(out.contains("\"Interact\""));
        assert!(out.contains("\"Instigator\""));
        assert!(out.contains("\"Highlight\""));
        assert!(out.contains("FUNC_BLUEPRINT_EVENT"));
        assert!(out.contains("CPF_RETURN_PARM"));
        assert!(out.contains("ScriptInterface < dyn Interactable >"));
        assert!(out.contains("__rusteal_signature_interact"));
    }

    #[test]
    fn the_shape_follows_the_signatures_only() {
        let shape = |item: TokenStream| {
            let out = expand(item).unwrap();
            let start = out.find("reify_create_interface").unwrap();
            out[start..start + 200].to_string()
        };

        let plain = shape(quote! { trait Usable { fn use_it(&mut self, power: f32); } });

        let documented = shape(quote! {
            trait Usable {
                /// Uses it.
                fn use_it(&mut self, power: f32);
            }
        });

        let changed = shape(quote! { trait Usable { fn use_it(&mut self, power: i32); } });

        assert_eq!(plain, documented);
        assert_ne!(plain, changed);
    }

    #[test]
    fn what_ue_cannot_hold_is_refused() {
        assert!(expand(quote! { trait A { fn f(&self) { } } }).is_err());
        assert!(expand(quote! { trait A { fn f(self); } }).is_err());
        assert!(expand(quote! { trait A { fn f(); } }).is_err());
        assert!(expand(quote! { trait A { fn f<T>(&self, t: T); } }).is_err());
        assert!(expand(quote! { trait A { fn f(&self, s: String); } }).is_err());
        assert!(expand(quote! { trait A { const X: i32; } }).is_err());
        assert!(expand(quote! { trait A<T> { fn f(&self); } }).is_err());
        assert!(expand(quote! { trait A: Clone { fn f(&self); } }).is_err());
        assert!(expand(quote! { trait A { fn f(&self, (a, b): (i32, i32)); } }).is_err());
    }
}
