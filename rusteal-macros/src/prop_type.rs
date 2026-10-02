// Rust type → RustealReifyPropType mapping + PropertyApi accessor names.

use proc_macro2::TokenStream;
use quote::quote;
use syn::Type;

/// Info about a mapped UE property type.
pub struct PropTypeInfo {
    /// Token for the RustealReifyPropType variant (e.g. `RustealReifyPropType::Float`).
    pub prop_type_expr: TokenStream,
    /// The Rust type used for the getter's return value and the setter's parameter:
    /// the type as written, so the imports it names count as used.
    pub rust_type: TokenStream,
    pub kind: PropKind,
}

/// How a property is described to the plugin and read and written from Rust.
pub enum PropKind {
    /// `bool`/`i32`/`i64`/`u8`/`f32`/`f64`, read and written by value.
    Scalar {
        /// Identifier for the PropertyApi getter (e.g. `get_f32`).
        getter_fn: syn::Ident,
        /// Identifier for the PropertyApi setter (e.g. `set_f32`).
        setter_fn: syn::Ident,
        /// Default zero-value expression for the getter's out variable.
        zero_expr: TokenStream,
    },
    /// `UObjectRef<T>`: an object reference restricted to `T`.
    Object { class: Type },
    /// `SubclassOf<T>`: a class reference restricted to `T` and its subclasses.
    Class { meta_class: Type },
    /// `UeArray<E>`: a `TArray` of a scalar, object or class element.
    Array { element: Box<PropTypeInfo> },
    /// `UStructRef<T>`: a UE struct, by reference to its memory. Only as a
    /// `#[ufunction]` parameter, where it points into the call's parameters.
    Struct { strukt: Type },
}

impl PropTypeInfo {
    /// Field initializers for a `RustealReifyPropExtra` describing this type, or
    /// `None` when the plugin needs no more than the type discriminator.
    pub fn extra_fields(&self) -> Option<TokenStream> {
        match &self.kind {
            PropKind::Scalar { .. } => None,
            PropKind::Object { class } => Some(quote! {
                class_handle: <#class as ::rusteal_runtime::runtime::UeClass>::static_class(),
            }),
            PropKind::Class { meta_class } => Some(quote! {
                meta_class_handle: <#meta_class as ::rusteal_runtime::runtime::UeClass>::static_class(),
            }),
            PropKind::Struct { strukt } => Some(quote! {
                struct_handle: <#strukt as ::rusteal_runtime::runtime::UeStruct>::static_struct(),
            }),
            PropKind::Array { element } => {
                let inner_type = &element.prop_type_expr;
                let inner_fields = element.extra_fields().unwrap_or_default();
                Some(quote! {
                    #inner_fields
                    inner_prop_type: #inner_type as u32,
                })
            }
        }
    }
}

/// The single type argument of a path segment (`T` in `UObjectRef<T>`).
fn single_type_arg(seg: &syn::PathSegment) -> Option<Type> {
    let syn::PathArguments::AngleBracketed(args) = &seg.arguments else {
        return None;
    };
    let mut types = args.args.iter().filter_map(|a| match a {
        syn::GenericArgument::Type(t) => Some(t.clone()),
        _ => None,
    });
    let ty = types.next()?;
    types.next().is_none().then_some(ty)
}

/// Try to map a Rust type to UE property type info.
/// Returns None for unsupported types.
pub fn map_type(ty: &Type) -> Option<PropTypeInfo> {
    let seg = match ty {
        Type::Path(tp) => tp.path.segments.last()?,
        _ => return None,
    };

    match seg.ident.to_string().as_str() {
        "UObjectRef" => {
            let class = single_type_arg(seg)?;
            return Some(PropTypeInfo {
                prop_type_expr: quote! { ::rusteal_runtime::ffi::RustealReifyPropType::Object },
                rust_type: quote! { #ty },
                kind: PropKind::Object { class },
            });
        }
        "SubclassOf" => {
            let meta_class = single_type_arg(seg)?;
            return Some(PropTypeInfo {
                prop_type_expr: quote! { ::rusteal_runtime::ffi::RustealReifyPropType::Class },
                rust_type: quote! { #ty },
                kind: PropKind::Class { meta_class },
            });
        }
        "UStructRef" => {
            let strukt = single_type_arg(seg)?;
            return Some(PropTypeInfo {
                prop_type_expr: quote! { ::rusteal_runtime::ffi::RustealReifyPropType::Struct },
                rust_type: quote! { #ty },
                kind: PropKind::Struct { strukt },
            });
        }
        "UeArray" => {
            let element = map_type(&single_type_arg(seg)?)?;
            if matches!(element.kind, PropKind::Array { .. } | PropKind::Struct { .. }) {
                return None; // no arrays of arrays; struct elements are not supported
            }
            return Some(PropTypeInfo {
                prop_type_expr: quote! { ::rusteal_runtime::ffi::RustealReifyPropType::Array },
                rust_type: quote! { #ty },
                kind: PropKind::Array { element: Box::new(element) },
            });
        }
        _ => {}
    }

    let ident = |s: &str| syn::Ident::new(s, proc_macro2::Span::call_site());
    let scalar = |variant: &str, rust: TokenStream, get: &str, set: &str, zero: TokenStream| {
        let variant = ident(variant);
        PropTypeInfo {
            prop_type_expr: quote! { ::rusteal_runtime::ffi::RustealReifyPropType::#variant },
            rust_type: rust,
            kind: PropKind::Scalar {
                getter_fn: ident(get),
                setter_fn: ident(set),
                zero_expr: zero,
            },
        }
    };

    let info = match seg.ident.to_string().as_str() {
        "bool" => scalar("Bool", quote! { bool }, "get_bool", "set_bool", quote! { false }),
        "i32" => scalar("Int32", quote! { i32 }, "get_i32", "set_i32", quote! { 0i32 }),
        "i64" => scalar("Int64", quote! { i64 }, "get_i64", "set_i64", quote! { 0i64 }),
        "u8" => scalar("UInt8", quote! { u8 }, "get_u8", "set_u8", quote! { 0u8 }),
        "f32" => scalar("Float", quote! { f32 }, "get_f32", "set_f32", quote! { 0.0f32 }),
        "f64" => scalar("Double", quote! { f64 }, "get_f64", "set_f64", quote! { 0.0f64 }),
        _ => return None,
    };
    Some(info)
}

/// The UE name of a property or parameter: PascalCase, except that a `b_`
/// prefix is UE's bool `b` (`b_enabled` is `bEnabled`), the way the
/// generated bindings name UE's bools the other way round.
pub fn to_ue_name(s: &str) -> String {
    match s.strip_prefix("b_") {
        Some(rest) if !rest.is_empty() => format!("b{}", to_pascal_case(rest)),
        _ => to_pascal_case(s),
    }
}

/// Convert snake_case field name to PascalCase UE property name.
pub fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(c) => {
                    let upper: String = c.to_uppercase().collect();
                    upper + chars.as_str()
                }
                None => String::new(),
            }
        })
        .collect()
}

/// Compile-time FNV-1a hash of a byte string, producing u64.
pub fn fnv1a_hash(s: &str) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;
    let mut hash = FNV_OFFSET;
    for &b in s.as_bytes() {
        hash ^= b as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    fn kind(ty: Type) -> Option<&'static str> {
        map_type(&ty).map(|info| match info.kind {
            PropKind::Scalar { .. } => "scalar",
            PropKind::Object { .. } => "object",
            PropKind::Class { .. } => "class",
            PropKind::Array { .. } => "array",
            PropKind::Struct { .. } => "struct",
        })
    }

    #[test]
    fn maps_supported_types() {
        assert_eq!(kind(parse_quote!(f32)), Some("scalar"));
        assert_eq!(kind(parse_quote!(UObjectRef<InputAction>)), Some("object"));
        assert_eq!(kind(parse_quote!(rusteal_runtime::runtime::UObjectRef<InputAction>)), Some("object"));
        assert_eq!(kind(parse_quote!(SubclassOf<Pawn>)), Some("class"));
        assert_eq!(kind(parse_quote!(UeArray<UObjectRef<InputMappingContext>>)), Some("array"));
        assert_eq!(kind(parse_quote!(UeArray<SubclassOf<Actor>>)), Some("array"));
        assert_eq!(kind(parse_quote!(UeArray<f32>)), Some("array"));
        assert_eq!(kind(parse_quote!(UStructRef<FInputActionValue>)), Some("struct"));
    }

    #[test]
    fn rejects_unsupported_types() {
        assert_eq!(kind(parse_quote!(String)), None);
        assert_eq!(kind(parse_quote!(UObjectRef)), None);
        assert_eq!(kind(parse_quote!(UeArray<UeArray<f32>>)), None);
        assert_eq!(kind(parse_quote!(UeArray<String>)), None);
        assert_eq!(kind(parse_quote!(UeArray<UStructRef<FVector>>)), None);
        assert_eq!(kind(parse_quote!(Vec<UObjectRef<InputAction>>)), None);
    }

    #[test]
    fn ue_names() {
        assert_eq!(to_ue_name("move_action"), "MoveAction");
        assert_eq!(to_ue_name("b_enabled"), "bEnabled");
        assert_eq!(to_ue_name("b_force_touch_controls"), "bForceTouchControls");
        assert_eq!(to_ue_name("b"), "B");
        assert_eq!(to_ue_name("bounce"), "Bounce");
    }

    #[test]
    fn array_extra_describes_the_element() {
        let info = map_type(&parse_quote!(UeArray<UObjectRef<InputAction>>)).unwrap();
        let extra = info.extra_fields().unwrap().to_string();
        assert!(extra.contains("class_handle"), "{extra}");
        assert!(extra.contains("inner_prop_type"), "{extra}");
        assert!(extra.contains("RustealReifyPropType :: Object"), "{extra}");
    }
}
