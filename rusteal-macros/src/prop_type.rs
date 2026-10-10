use proc_macro2::TokenStream;
use quote::quote;
use syn::Type;

pub struct PropTypeInfo {
    pub prop_type_expr: TokenStream,
    pub rust_type: TokenStream,
    pub kind: PropKind,
}

pub enum PropKind {
    Scalar {
        getter_fn: syn::Ident,
        setter_fn: syn::Ident,
        zero_expr: TokenStream,
    },
    Object {
        class: Type,
    },
    Class {
        meta_class: Type,
    },
    SoftObject {
        class: Type,
    },
    Array {
        element: Box<PropTypeInfo>,
    },
    Struct {
        strukt: Type,
    },
    OwnedStruct {
        strukt: Type,
    },
    Name,
    Str,
    Enum {
        ty: Type,
    },
}

impl PropKind {
    pub fn property_only(&self) -> bool {
        matches!(
            self,
            PropKind::OwnedStruct { .. }
                | PropKind::Name
                | PropKind::Str
                | PropKind::SoftObject { .. }
        )
    }
}

impl PropTypeInfo {
    pub fn extra_fields(&self) -> Option<TokenStream> {
        match &self.kind {
            PropKind::Scalar { .. } => None,
            PropKind::Object { class } | PropKind::SoftObject { class } => Some(quote! {
                class_handle: <#class as ::rusteal_runtime::runtime::UeClass>::static_class(),
            }),
            PropKind::Class { meta_class } => Some(quote! {
                meta_class_handle: <#meta_class as ::rusteal_runtime::runtime::UeClass>::static_class(),
            }),
            PropKind::Struct { strukt } | PropKind::OwnedStruct { strukt } => Some(quote! {
                struct_handle: <#strukt as ::rusteal_runtime::runtime::UeStruct>::static_struct(),
            }),
            PropKind::Name | PropKind::Str => None,
            PropKind::Enum { ty } => Some(quote! {
                enum_handle: <#ty as ::rusteal_runtime::runtime::UeEnum>::static_enum(),
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
        "SoftObjectRef" => {
            let class = single_type_arg(seg)?;

            return Some(PropTypeInfo {
                prop_type_expr: quote! { ::rusteal_runtime::ffi::RustealReifyPropType::SoftObject },
                rust_type: quote! { #ty },
                kind: PropKind::SoftObject { class },
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
        "OwnedStruct" => {
            let strukt = single_type_arg(seg)?;

            return Some(PropTypeInfo {
                prop_type_expr: quote! { ::rusteal_runtime::ffi::RustealReifyPropType::Struct },
                rust_type: quote! { #ty },
                kind: PropKind::OwnedStruct { strukt },
            });
        }
        "FName" if seg.arguments.is_none() => {
            return Some(PropTypeInfo {
                prop_type_expr: quote! { ::rusteal_runtime::ffi::RustealReifyPropType::Name },
                rust_type: quote! { #ty },
                kind: PropKind::Name,
            });
        }
        "String" if seg.arguments.is_none() => {
            return Some(PropTypeInfo {
                prop_type_expr: quote! { ::rusteal_runtime::ffi::RustealReifyPropType::String },
                rust_type: quote! { #ty },
                kind: PropKind::Str,
            });
        }
        "UeArray" => {
            let element = map_type(&single_type_arg(seg)?)?;

            if !matches!(
                element.kind,
                PropKind::Scalar { .. }
                    | PropKind::Object { .. }
                    | PropKind::Class { .. }
                    | PropKind::Name
            ) {
                return None;
            }

            return Some(PropTypeInfo {
                prop_type_expr: quote! { ::rusteal_runtime::ffi::RustealReifyPropType::Array },
                rust_type: quote! { #ty },
                kind: PropKind::Array {
                    element: Box::new(element),
                },
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
        "bool" => scalar(
            "Bool",
            quote! { bool },
            "get_bool",
            "set_bool",
            quote! { false },
        ),
        "i32" => scalar(
            "Int32",
            quote! { i32 },
            "get_i32",
            "set_i32",
            quote! { 0i32 },
        ),
        "i64" => scalar(
            "Int64",
            quote! { i64 },
            "get_i64",
            "set_i64",
            quote! { 0i64 },
        ),
        "u8" => scalar("UInt8", quote! { u8 }, "get_u8", "set_u8", quote! { 0u8 }),
        "f32" => scalar(
            "Float",
            quote! { f32 },
            "get_f32",
            "set_f32",
            quote! { 0.0f32 },
        ),
        "f64" => scalar(
            "Double",
            quote! { f64 },
            "get_f64",
            "set_f64",
            quote! { 0.0f64 },
        ),
        "UObjectRef" | "SubclassOf" | "SoftObjectRef" | "UStructRef" | "OwnedStruct"
        | "UeArray" => {
            return None;
        }
        name if seg.arguments.is_none()
            && name.chars().next().is_some_and(|c| c.is_ascii_uppercase()) =>
        {
            PropTypeInfo {
                prop_type_expr: quote! { ::rusteal_runtime::ffi::RustealReifyPropType::Enum },
                rust_type: quote! { #ty },
                kind: PropKind::Enum { ty: ty.clone() },
            }
        }
        _ => return None,
    };

    Some(info)
}

pub fn to_ue_name(s: &str) -> String {
    match s.strip_prefix("b_") {
        Some(rest) if !rest.is_empty() => format!("b{}", to_pascal_case(rest)),
        _ => to_pascal_case(s),
    }
}

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
            PropKind::SoftObject { .. } => "soft object",
            PropKind::Array { .. } => "array",
            PropKind::Struct { .. } => "struct",
            PropKind::OwnedStruct { .. } => "owned struct",
            PropKind::Name => "name",
            PropKind::Str => "string",
            PropKind::Enum { .. } => "enum",
        })
    }

    #[test]
    fn maps_supported_types() {
        assert_eq!(kind(parse_quote!(f32)), Some("scalar"));
        assert_eq!(kind(parse_quote!(UObjectRef<InputAction>)), Some("object"));

        assert_eq!(
            kind(parse_quote!(
                rusteal_runtime::runtime::UObjectRef<InputAction>
            )),
            Some("object")
        );

        assert_eq!(kind(parse_quote!(SubclassOf<Pawn>)), Some("class"));

        assert_eq!(
            kind(parse_quote!(SoftObjectRef<StaticMesh>)),
            Some("soft object")
        );

        assert_eq!(
            kind(parse_quote!(UeArray<UObjectRef<InputMappingContext>>)),
            Some("array")
        );

        assert_eq!(
            kind(parse_quote!(UeArray<SubclassOf<Actor>>)),
            Some("array")
        );

        assert_eq!(kind(parse_quote!(UeArray<f32>)), Some("array"));
        assert_eq!(kind(parse_quote!(UeArray<FName>)), Some("array"));

        assert_eq!(
            kind(parse_quote!(UStructRef<FInputActionValue>)),
            Some("struct")
        );

        assert_eq!(
            kind(parse_quote!(OwnedStruct<FVector>)),
            Some("owned struct")
        );

        assert_eq!(kind(parse_quote!(FName)), Some("name"));
        assert_eq!(kind(parse_quote!(String)), Some("string"));
        assert_eq!(kind(parse_quote!(ECollisionChannel)), Some("enum"));

        assert_eq!(
            kind(parse_quote!(bindings::engine::ECollisionChannel)),
            Some("enum")
        );
    }

    #[test]
    fn rejects_unsupported_types() {
        assert_eq!(kind(parse_quote!(UObjectRef)), None);
        assert_eq!(kind(parse_quote!(OwnedStruct)), None);
        assert_eq!(kind(parse_quote!(SoftObjectRef)), None);
        assert_eq!(kind(parse_quote!(UeArray<SoftObjectRef<StaticMesh>>)), None);
        assert_eq!(kind(parse_quote!(usize)), None);
        assert_eq!(kind(parse_quote!(UeArray<UeArray<f32>>)), None);
        assert_eq!(kind(parse_quote!(UeArray<String>)), None);
        assert_eq!(kind(parse_quote!(UeArray<OwnedStruct<FVector>>)), None);
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
