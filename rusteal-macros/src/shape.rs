// The shape of a type Rusteal reifies: what its UE type is built from, as a
// hash a hot reload compares to tell whether the type changed. It covers the
// declarations the UE type is made of (the macro's arguments, the UE fields,
// the UE methods' signatures), never method bodies or Rust-private fields, so
// a change to code alone keeps the shape.

use quote::ToTokens;

use crate::prop_type;

/// The hash of these declarations, in any order.
pub(crate) fn hash<I: IntoIterator<Item = String>>(declarations: I) -> u64 {
    let mut declarations: Vec<String> = declarations.into_iter().collect();
    declarations.sort();
    prop_type::fnv1a_hash(&declarations.join("\u{1f}"))
}

/// A declaration with the attributes among `attrs` it carries: a field's name
/// and type, a method's signature. Doc comments are not part of it.
pub(crate) fn declaration(attributes: &[syn::Attribute], attrs: &[&str], item: &impl ToTokens) -> String {
    let mut out = String::new();
    for attr in attributes.iter().filter(|a| attrs.iter().any(|name| a.path().is_ident(name))) {
        out.push_str(&attr.to_token_stream().to_string());
        out.push(' ');
    }
    out.push_str(&item.to_token_stream().to_string());
    out
}

/// A field's declaration when it carries one of `attrs`.
pub(crate) fn field(field: &syn::Field, attrs: &[&str]) -> Option<String> {
    if !field.attrs.iter().any(|a| attrs.iter().any(|name| a.path().is_ident(name))) {
        return None;
    }
    let ident = field.ident.as_ref()?;
    let ty = &field.ty;
    Some(declaration(&field.attrs, attrs, &quote::quote! { #ident: #ty }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    fn fields(item: syn::ItemStruct) -> Vec<String> {
        item.fields.iter().filter_map(|f| field(f, &["uproperty", "component"])).collect()
    }

    #[test]
    fn ue_fields_make_the_shape() {
        let before = fields(parse_quote! {
            struct Platform {
                #[uproperty(EditAnywhere)]
                speed: f32,
                timer: f64,
            }
        });
        let same = fields(parse_quote! {
            struct Platform {
                /// A comment changes nothing.
                #[uproperty(EditAnywhere)]
                speed:   f32,
                timer: f32,
                elapsed: f64,
            }
        });
        let added = fields(parse_quote! {
            struct Platform {
                #[uproperty(EditAnywhere)]
                speed: f32,
                #[uproperty(EditAnywhere)]
                distance: f32,
            }
        });
        let flags = fields(parse_quote! {
            struct Platform {
                #[uproperty(VisibleAnywhere)]
                speed: f32,
            }
        });
        assert_eq!(hash(before.clone()), hash(same));
        assert_ne!(hash(before.clone()), hash(added));
        assert_ne!(hash(before), hash(flags));
    }

    #[test]
    fn order_does_not_matter() {
        let a = vec!["a".to_string(), "b".to_string()];
        let b = vec!["b".to_string(), "a".to_string()];
        assert_eq!(hash(a), hash(b));
    }

    #[test]
    fn a_method_body_is_not_part_of_it() {
        let shape = |method: syn::ImplItemFn| declaration(&method.attrs, &["ufunction"], &method.sig);
        let before = shape(parse_quote! {
            #[ufunction(BlueprintCallable)]
            fn jump(&mut self, height: f32) { let _ = height; }
        });
        let body = shape(parse_quote! {
            #[ufunction(BlueprintCallable)]
            fn jump(&mut self, height: f32) { println!("{height}"); }
        });
        let params = shape(parse_quote! {
            #[ufunction(BlueprintCallable)]
            fn jump(&mut self, height: f64) {}
        });
        assert_eq!(before, body);
        assert_ne!(before, params);
    }
}
