use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Fields, ItemStruct, parse2};

use crate::prop_type;
use crate::shape;
use crate::uclass::{
    UPropertyField, add_property_statements, parse_uproperty_args, property_accessors,
    to_screaming_snake, to_snake_case,
};

pub fn expand_ustruct(attr: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    if !attr.is_empty() {
        return Err(syn::Error::new_spanned(
            attr,
            "#[ustruct] takes no arguments",
        ));
    }

    let input: ItemStruct = parse2(item)?;
    let name = &input.ident;
    let vis = &input.vis;
    let name_str = name.to_string();
    let attrs = &input.attrs;

    let Fields::Named(fields) = &input.fields else {
        return Err(syn::Error::new_spanned(
            &input,
            "#[ustruct] requires a struct with named fields, each a #[uproperty]",
        ));
    };

    let mut uprops: Vec<UPropertyField> = Vec::new();
    let mut field_docs = Vec::new();

    for field in &fields.named {
        let ident = field.ident.clone().unwrap();

        let Some(attr) = field.attrs.iter().find(|a| a.path().is_ident("uproperty")) else {
            return Err(syn::Error::new_spanned(
                field,
                "every #[ustruct] field is a #[uproperty]: the struct's memory is UE's",
            ));
        };

        let args = parse_uproperty_args(attr)?;

        let Some(info) = prop_type::map_type(&field.ty) else {
            return Err(syn::Error::new_spanned(
                &field.ty,
                "unsupported uproperty type: supported are bool/i32/i64/u8/f32/f64, \
                 UObjectRef<T>, SubclassOf<T>, SoftObjectRef<T>, UeArray of those, \
                 OwnedStruct<T>, FName, String and UE enums",
            ));
        };

        if matches!(info.kind, prop_type::PropKind::Struct { .. }) {
            return Err(syn::Error::new_spanned(
                &field.ty,
                "a struct field is an OwnedStruct<T>",
            ));
        }

        if args.default_expr.is_some() {
            return Err(syn::Error::new_spanned(
                attr,
                "a #[ustruct] field has no `default = ...`: it starts zeroed (null, empty, 0)",
            ));
        }

        let docs: Vec<_> = field
            .attrs
            .iter()
            .filter(|a| a.path().is_ident("doc"))
            .cloned()
            .collect();

        field_docs.push((ident.clone(), docs));

        uprops.push(UPropertyField {
            ident,
            ty: field.ty.clone(),
            args,
        });
    }

    let shape_value = shape::hash(
        fields
            .named
            .iter()
            .filter_map(|f| shape::field(f, &["uproperty"])),
    );

    let handle_name = format_ident!("__RUSTEAL_STRUCT_HANDLE_{}", to_screaming_snake(&name_str));
    let create_fn = format_ident!("__rusteal_create_struct_{}", to_snake_case(&name_str));
    let members_fn = format_ident!("__rusteal_struct_members_{}", to_snake_case(&name_str));
    let finalize_fn = format_ident!("__rusteal_finalize_struct_{}", to_snake_case(&name_str));
    let ext_name = format_ident!("{}Ext", name);
    let name_bytes = name_str.as_bytes();
    let name_len = name_str.len() as u32;

    let accessors = property_accessors(
        &uprops,
        &quote! { self.as_ptr() },
        &format_ident!("reflection_find_struct_property"),
        &quote! { <#name as ::rusteal_runtime::runtime::UeStruct>::static_struct() },
    );

    let docs_of = |accessor: &str| {
        field_docs
            .iter()
            .find(|(ident, _)| *ident == accessor || format!("set_{ident}") == accessor)
            .map(|(_, docs)| docs.clone())
            .unwrap_or_default()
    };

    let mut trait_items = Vec::new();
    let mut ref_impls = Vec::new();
    let mut owned_impls = Vec::new();

    for accessor in &accessors {
        let signature = accessor.signature();
        let body = &accessor.body;
        let method = &accessor.name;
        let docs = docs_of(&method.to_string());

        let forward = if accessor.param.is_some() {
            quote! { self.as_ref().#method(val) }
        } else {
            quote! { self.as_ref().#method() }
        };

        trait_items.push(quote! { #(#docs)* #signature; });
        ref_impls.push(quote! { #signature { #body } });
        owned_impls.push(quote! { #signature { #forward } });
    }

    let add_prop_stmts = add_property_statements(&uprops);
    let ext_doc = format!("The fields of a `{name_str}`, wherever it is.");

    Ok(quote! {
        #(#attrs)*
        #vis struct #name;

        #[doc(hidden)]
        pub static #handle_name: std::sync::OnceLock<::rusteal_runtime::ffi::UStructHandle> =
            std::sync::OnceLock::new();

        impl ::rusteal_runtime::runtime::UeStruct for #name {
            fn static_struct() -> ::rusteal_runtime::ffi::UStructHandle {
                *#handle_name.get().expect(concat!(
                    "Failed to find the UScriptStruct for ", stringify!(#name), " — was it registered?"
                ))
            }
        }

        #[doc = #ext_doc]
        #vis trait #ext_name {
            #(#trait_items)*
        }

        impl #ext_name for ::rusteal_runtime::runtime::UStructRef<#name> {
            #(#ref_impls)*
        }

        impl #ext_name for ::rusteal_runtime::runtime::OwnedStruct<#name> {
            #(#owned_impls)*
        }

        #[doc(hidden)]
        pub fn #create_fn() {
            let handle = unsafe {
                ::rusteal_runtime::runtime::ffi_dispatch::reify_create_struct(
                    [#(#name_bytes),*].as_ptr(),
                    #name_len,
                    #shape_value,
                )
            };

            if handle.is_null() {
                let msg = concat!("[Rusteal] ", stringify!(#name), ": create_struct failed");
                let bytes = msg.as_bytes();
                unsafe { ::rusteal_runtime::runtime::ffi_dispatch::logging_log(2, bytes.as_ptr(), bytes.len() as u32); }
                return;
            }

            #handle_name.set(handle).ok();
        }

        #[doc(hidden)]
        pub fn #members_fn() {
            let Some(&handle) = #handle_name.get() else {
                return;
            };

            let class = ::rusteal_runtime::ffi::UClassHandle(handle.0);

            #(#add_prop_stmts)*
        }

        #[doc(hidden)]
        pub fn #finalize_fn() {
            let Some(&handle) = #handle_name.get() else {
                return;
            };

            let result = unsafe { ::rusteal_runtime::runtime::ffi_dispatch::reify_finalize_struct(handle) };

            if result != ::rusteal_runtime::ffi::RustealErrorCode::Ok {
                let msg = concat!("[Rusteal] ", stringify!(#name), ": finalize_struct failed");
                let bytes = msg.as_bytes();
                unsafe { ::rusteal_runtime::runtime::ffi_dispatch::logging_log(2, bytes.as_ptr(), bytes.len() as u32); }
            }
        }

        ::rusteal_runtime::__inventory::submit! {
            ::rusteal_runtime::runtime::reify_registry::StructRegistration {
                create: #create_fn,
                register: #members_fn,
                finalize: #finalize_fn,
            }
        }
    })
}
