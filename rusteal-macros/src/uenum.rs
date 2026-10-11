use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Expr, ExprLit, Fields, ItemEnum, Lit, parse2};

use crate::uclass::to_screaming_snake;

pub fn expand_uenum(attr: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    if !attr.is_empty() {
        return Err(syn::Error::new_spanned(attr, "#[uenum] takes no arguments"));
    }

    let input: ItemEnum = parse2(item)?;
    let name = &input.ident;
    let vis = &input.vis;
    let name_str = name.to_string();

    if input.attrs.iter().any(|a| a.path().is_ident("repr")) {
        return Err(syn::Error::new_spanned(
            name,
            "#[uenum] sets the representation itself: u8, as Blueprint enums are",
        ));
    }

    let mut variants = Vec::new();
    let mut next: i64 = 0;

    for variant in &input.variants {
        if !matches!(variant.fields, Fields::Unit) {
            return Err(syn::Error::new_spanned(
                variant,
                "#[uenum] variants carry no data",
            ));
        }

        if let Some((_, expr)) = &variant.discriminant {
            next = match expr {
                Expr::Lit(ExprLit {
                    lit: Lit::Int(int), ..
                }) => int.base10_parse()?,
                _ => {
                    return Err(syn::Error::new_spanned(
                        expr,
                        "a #[uenum] discriminant is an integer literal",
                    ));
                }
            };
        }

        if !(0..u8::MAX as i64).contains(&next) {
            return Err(syn::Error::new_spanned(
                variant,
                "a #[uenum] value fits in 0..=254 (255 is the enum's MAX entry)",
            ));
        }

        variants.push((variant.ident.clone(), next));
        next += 1;
    }

    if variants.is_empty() {
        return Err(syn::Error::new_spanned(name, "#[uenum] needs a variant"));
    }

    let attrs = &input.attrs;
    let enum_variants = input.variants.iter().map(|v| {
        let ident = &v.ident;
        let docs = v.attrs.iter().filter(|a| a.path().is_ident("doc"));
        let value = variants.iter().find(|(i, _)| i == ident).unwrap().1 as u8;
        quote! { #(#docs)* #ident = #value }
    });

    let names = variants
        .iter()
        .map(|(ident, _)| ident.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    let names_len = names.len() as u32;
    let values: Vec<i64> = variants.iter().map(|(_, v)| *v).collect();
    let count = values.len() as u32;
    let from_arms = variants
        .iter()
        .map(|(ident, value)| quote! { #value => Some(#name::#ident) });

    let handle_name = format_ident!("__RUSTEAL_ENUM_HANDLE_{}", to_screaming_snake(&name_str));
    let create_fn = format_ident!("__rusteal_create_enum_{}", name_str.to_lowercase());
    let name_bytes = name_str.as_bytes();
    let name_len = name_str.len() as u32;

    Ok(quote! {
        #(#attrs)*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #[repr(u8)]
        #vis enum #name {
            #(#enum_variants,)*
        }

        #[doc(hidden)]
        pub static #handle_name: std::sync::OnceLock<::rusteal_runtime::ffi::UClassHandle> =
            std::sync::OnceLock::new();

        impl ::rusteal_runtime::runtime::UeEnum for #name {
            type Repr = u8;

            fn static_enum() -> ::rusteal_runtime::ffi::UClassHandle {
                *#handle_name.get().expect(concat!(
                    "Failed to find the UEnum for ", stringify!(#name), " — was it registered?"
                ))
            }

            fn to_i64(self) -> i64 {
                self as u8 as i64
            }

            fn from_i64(value: i64) -> Option<Self> {
                match value {
                    #(#from_arms,)*
                    _ => None,
                }
            }
        }

        #[doc(hidden)]
        pub fn #create_fn() {
            let names: &[u8] = #names.as_bytes();
            let values: [i64; #count as usize] = [#(#values),*];

            let handle = unsafe {
                ::rusteal_runtime::runtime::ffi_dispatch::reify_create_enum(
                    [#(#name_bytes),*].as_ptr(),
                    #name_len,
                    names.as_ptr(),
                    #names_len,
                    values.as_ptr(),
                    #count,
                )
            };

            if handle.is_null() {
                let msg = concat!("[Rusteal] ", stringify!(#name), ": create_enum failed");
                let bytes = msg.as_bytes();
                unsafe { ::rusteal_runtime::runtime::ffi_dispatch::logging_log(2, bytes.as_ptr(), bytes.len() as u32); }
                return;
            }

            #handle_name.set(handle).ok();
        }

        ::rusteal_runtime::__inventory::submit! {
            ::rusteal_runtime::runtime::reify_registry::EnumRegistration {
                create: #create_fn,
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    fn expand(item: TokenStream) -> syn::Result<TokenStream> {
        expand_uenum(TokenStream::new(), item)
    }

    #[test]
    fn enums_get_their_unreal_registration() {
        let out = expand(quote! { pub enum EGrabType { None, Snap, Custom = 3, Free } })
            .unwrap()
            .to_string();

        assert!(out.contains("repr (u8)"));
        assert!(out.contains("Custom = 3u8"));
        assert!(out.contains("Free = 4u8"));
        assert!(out.contains("reify_create_enum"));
        assert!(out.contains("EnumRegistration"));
    }

    #[test]
    fn data_variants_explicit_reprs_and_large_values_are_refused() {
        assert!(expand(quote! { enum E { A(u8) } }).is_err());
        assert!(expand(quote! { #[repr(u16)] enum E { A } }).is_err());
        assert!(expand(quote! { enum E { A = 255 } }).is_err());
        assert!(expand(quote! { enum E {} }).is_err());
    }
}
