use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::ext::IdentExt;
use syn::{FnArg, Ident, ImplItemFn, Meta, ReceiverKind, ReturnType, Type};

use crate::prop_type::{self, PropKind};

pub(crate) struct DelegateParam {
    rust_name: Ident,
    ue_name: String,
    rust_ty: Type,
    kind: ParamKind,
}

enum ParamKind {
    Scalar,
    Object,
    Class,
    Struct(Box<Type>),
}

pub(crate) struct DelegateInfo {
    pub(crate) method_ident: Ident,
    ue_name: String,
    params: Vec<DelegateParam>,
}

pub(crate) fn is_udelegate(method: &ImplItemFn) -> bool {
    method.attrs.iter().any(|a| a.path().is_ident("udelegate"))
}

pub(crate) fn parse_udelegate(method: &ImplItemFn) -> syn::Result<DelegateInfo> {
    let attr = method
        .attrs
        .iter()
        .find(|a| a.path().is_ident("udelegate"))
        .unwrap();

    let mut explicit_name = None;

    if let Meta::List(_) = attr.meta {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("name") {
                let value: syn::LitStr = meta.value()?.parse()?;
                explicit_name = Some(value.value());

                Ok(())
            } else {
                Err(meta.error("unknown #[udelegate] argument: expected name = \"...\""))
            }
        })?;
    }

    let takes_ref_self = matches!(
        method.sig.inputs.first(),
        Some(FnArg::Receiver(r)) if matches!(r.kind, ReceiverKind::Reference(_, _, None))
    );

    if !takes_ref_self {
        return Err(syn::Error::new_spanned(
            &method.sig,
            "a #[udelegate] takes `&self`",
        ));
    }

    if !matches!(method.sig.output, ReturnType::Default) {
        return Err(syn::Error::new_spanned(
            &method.sig.output,
            "a #[udelegate] returns nothing",
        ));
    }

    if !method.block.stmts.is_empty() {
        return Err(syn::Error::new_spanned(
            &method.block,
            "a #[udelegate] has an empty body: calling the method broadcasts the delegate",
        ));
    }

    let mut params = Vec::new();

    for arg in method.sig.inputs.iter().skip(1) {
        let FnArg::Typed(pat_type) = arg else {
            continue;
        };

        let syn::Pat::Ident(pi) = &*pat_type.pat else {
            return Err(syn::Error::new_spanned(
                &pat_type.pat,
                "#[udelegate] parameters are simple identifiers",
            ));
        };

        let ty = (*pat_type.ty).clone();

        let kind = param_kind(&ty).ok_or_else(|| {
            syn::Error::new_spanned(
                &ty,
                "a #[udelegate] takes bool/i32/i64/u8/f32/f64, UObjectRef<T>, SubclassOf<T> \
                 and structs as &OwnedStruct<T>",
            )
        })?;

        params.push(DelegateParam {
            rust_name: pi.ident.clone(),
            ue_name: prop_type::to_ue_name(&pi.ident.unraw().to_string()),
            rust_ty: ty,
            kind,
        });
    }

    let method_ident = method.sig.ident.clone();

    let ue_name = explicit_name
        .unwrap_or_else(|| prop_type::to_pascal_case(&method_ident.unraw().to_string()));

    Ok(DelegateInfo {
        method_ident,
        ue_name,
        params,
    })
}

fn param_kind(ty: &Type) -> Option<ParamKind> {
    if let Type::Reference(reference) = ty {
        let inner = (*reference.elem).clone();

        return matches!(
            prop_type::map_type(&inner)?.kind,
            PropKind::OwnedStruct { .. }
        )
        .then_some(ParamKind::Struct(Box::new(inner)));
    }

    match prop_type::map_type(ty)?.kind {
        PropKind::Scalar { .. } => Some(ParamKind::Scalar),
        PropKind::Object { .. } => Some(ParamKind::Object),
        PropKind::Class { .. } => Some(ParamKind::Class),
        _ => None,
    }
}

fn prop_lookup(struct_name: &Ident, ue_name: &str) -> TokenStream {
    let bytes = ue_name.as_bytes();
    let len = ue_name.len() as u32;

    quote! {{
        static PROP: std::sync::OnceLock<::rusteal_runtime::ffi::FPropertyHandle> = std::sync::OnceLock::new();

        *PROP.get_or_init(|| unsafe {
            ::rusteal_runtime::runtime::ffi_dispatch::reflection_find_property(
                <#struct_name as ::rusteal_runtime::runtime::UeClass>::static_class(),
                [#(#bytes),*].as_ptr(),
                #len,
            )
        })
    }}
}

pub(crate) fn broadcast_body(struct_name: &Ident, d: &DelegateInfo) -> syn::Block {
    let ue_name = &d.ue_name;
    let prop = prop_lookup(struct_name, ue_name);

    let sets: Vec<TokenStream> = d
        .params
        .iter()
        .map(|p| {
            let name = &p.rust_name;
            let param = &p.ue_name;

            match &p.kind {
                ParamKind::Scalar => quote! { call.set(#param, #name)?; },
                ParamKind::Object => quote! { call.set(#param, #name.raw())?; },
                ParamKind::Class => {
                    quote! { call.set(#param, ::rusteal_runtime::ffi::UObjectHandle(#name.raw().0))?; }
                }
                ParamKind::Struct(_) => quote! { call.set_struct(#param, #name)?; },
            }
        })
        .collect();

    syn::parse_quote! {{
        let __broadcast = || -> ::rusteal_runtime::runtime::RustealResult<()> {
            let mut call = ::rusteal_runtime::runtime::DynamicCall::for_delegate(&self.as_ref(), #prop)?;

            #(#sets)*
            call.broadcast()
        };

        if let Err(e) = __broadcast() {
            ::rusteal_runtime::runtime::ulog!(
                ::rusteal_runtime::runtime::LOG_WARNING,
                "[Rusteal] broadcasting {} failed: {}", #ue_name, e
            );
        }
    }}
}

pub(crate) fn register_stmts(d: &DelegateInfo) -> TokenStream {
    let bytes = d.ue_name.as_bytes();
    let len = d.ue_name.len() as u32;

    let params: Vec<TokenStream> = d
        .params
        .iter()
        .map(|p| {
            let reg_ty = match &p.kind {
                ParamKind::Struct(inner) => (**inner).clone(),
                _ => p.rust_ty.clone(),
            };

            let info = prop_type::map_type(&reg_ty).unwrap();
            let prop_type_expr = &info.prop_type_expr;
            let extra = crate::uclass_impl::extra_expr(&info);
            let pbytes = p.ue_name.as_bytes();
            let plen = p.ue_name.len() as u32;

            quote! {
                ::rusteal_runtime::runtime::ffi_dispatch::reify_add_function_param(
                    __signature,
                    [#(#pbytes),*].as_ptr(),
                    #plen,
                    #prop_type_expr as u32,
                    ::rusteal_runtime::ffi::CPF_PARM,
                    #extra,
                );
            }
        })
        .collect();

    quote! {
        unsafe {
            let __signature = ::rusteal_runtime::runtime::ffi_dispatch::reify_add_delegate(
                cls,
                [#(#bytes),*].as_ptr(),
                #len,
                0u64,
            );

            if !__signature.is_null() {
                #(#params)*
            }
        }
    }
}

pub(crate) fn binding_methods(struct_name: &Ident, d: &DelegateInfo) -> TokenStream {
    let add = format_ident!("{}_add", d.method_ident.unraw());
    let add_ufunction = format_ident!("{}_add_ufunction", d.method_ident.unraw());
    let ue_name = &d.ue_name;
    let prop = prop_lookup(struct_name, ue_name);
    let n = d.params.len();

    let param_bytes: Vec<TokenStream> = d
        .params
        .iter()
        .map(|p| {
            let b = p.ue_name.as_bytes();
            let l = p.ue_name.len() as u32;

            quote! {
                ::rusteal_runtime::runtime::ffi_dispatch::reflection_get_property_offset(
                    ::rusteal_runtime::runtime::ffi_dispatch::reflection_get_function_param(
                        signature, [#(#b),*].as_ptr(), #l,
                    ),
                )
            }
        })
        .collect();

    let callback_types: Vec<TokenStream> = d
        .params
        .iter()
        .map(|p| match &p.kind {
            ParamKind::Struct(inner) => quote! { #inner },
            _ => {
                let ty = &p.rust_ty;
                quote! { #ty }
            }
        })
        .collect();

    let reads: Vec<TokenStream> = d
        .params
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let ty = &p.rust_ty;
            let idx = syn::Index::from(i);

            let handle = quote! {
                ::rusteal_runtime::runtime::ffi_dispatch::native_mem_read::<::rusteal_runtime::ffi::UObjectHandle>(
                    params, offsets[#idx] as usize,
                )
            };

            match &p.kind {
                ParamKind::Scalar => quote! {
                    ::rusteal_runtime::runtime::ffi_dispatch::native_mem_read::<#ty>(params, offsets[#idx] as usize)
                },
                ParamKind::Object => quote! { <#ty>::from_raw(#handle) },
                ParamKind::Class => quote! {
                    <#ty>::from_raw(::rusteal_runtime::ffi::UClassHandle(#handle.0))
                },
                ParamKind::Struct(inner) => quote! {
                    {
                        let r: ::rusteal_runtime::runtime::UStructRef<_> =
                            ::rusteal_runtime::runtime::struct_ref_from_param(params, offsets[#idx] as usize);

                        let owned: #inner = r.to_owned();

                        owned
                    }
                },
            }
        })
        .collect();

    let add_doc = format!(
        " Bind a closure to `{ue_name}`; dropping the returned binding unbinds it, `detach()` keeps it."
    );

    let add_ufunction_doc = format!(
        " Bind `target`'s UFunction named `function` to `{ue_name}`, as C++'s `AddDynamic` does: \
         once, however many times it is called."
    );

    quote! {
        impl #struct_name {
            #[doc = #add_doc]
            pub fn #add(
                &self,
                mut callback: impl FnMut(#(#callback_types),*) + Send + 'static,
            ) -> ::rusteal_runtime::runtime::RustealResult<::rusteal_runtime::runtime::DelegateBinding> {
                let prop = #prop;
                let owner = self.as_ref().checked()?.raw();
                static OFFSETS: std::sync::OnceLock<[u32; #n]> = std::sync::OnceLock::new();

                let offsets = *OFFSETS.get_or_init(|| unsafe {
                    #[allow(unused_variables)]
                    let signature = ::rusteal_runtime::runtime::ffi_dispatch::reflection_get_delegate_signature(prop);
                    [#(#param_bytes),*]
                });

                ::rusteal_runtime::runtime::delegate_registry::bind_multicast(
                    owner,
                    prop,
                    move |params: ::rusteal_runtime::runtime::ffi_dispatch::NativePtr| {
                        #[allow(unused_variables)]
                        let (params, offsets) = (params, offsets);

                        callback(#(unsafe { #reads }),*)
                    },
                )
            }

            #[doc = #add_ufunction_doc]
            pub fn #add_ufunction(
                &self,
                target: &::rusteal_runtime::runtime::UObjectRef<impl ::rusteal_runtime::runtime::UeClass>,
                function: &str,
            ) -> ::rusteal_runtime::runtime::RustealResult<()> {
                let prop = #prop;
                let owner = self.as_ref().checked()?.raw();
                let target = target.checked()?.raw();

                ::rusteal_runtime::runtime::check_ffi(unsafe {
                    ::rusteal_runtime::runtime::ffi_dispatch::delegate_add_function(
                        owner,
                        prop,
                        target,
                        function.as_ptr(),
                        function.len() as u32,
                    )
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn parses_a_delegate() {
        let method: ImplItemFn = parse_quote! {
            #[udelegate]
            pub fn on_health_changed(&self, health: f32, at: &OwnedStruct<FVector>) {}
        };

        let d = parse_udelegate(&method).unwrap();
        assert_eq!(d.ue_name, "OnHealthChanged");
        assert_eq!(d.params.len(), 2);
        assert_eq!(d.params[1].ue_name, "At");

        let renamed: ImplItemFn = parse_quote! {
            #[udelegate(name = "OnEmptied")]
            pub fn emptied(&self) {}
        };

        assert_eq!(parse_udelegate(&renamed).unwrap().ue_name, "OnEmptied");
    }

    #[test]
    fn rejects_what_a_delegate_cannot_be() {
        let bad: Vec<ImplItemFn> = vec![
            parse_quote! { #[udelegate] fn a(&mut self) {} },
            parse_quote! { #[udelegate] fn b(&self) -> i32 {} },
            parse_quote! { #[udelegate] fn c(&self) { let x = 1; } },
            parse_quote! { #[udelegate] fn d(&self, s: String) {} },
            parse_quote! { #[udelegate(other = "x")] fn e(&self) {} },
        ];

        for method in &bad {
            assert!(parse_udelegate(method).is_err(), "{}", method.sig.ident);
        }
    }
}
