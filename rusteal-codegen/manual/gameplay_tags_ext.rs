use rusteal_core::{FName, OwnedStruct, RustealError, RustealResult};

use crate::gameplay_tags::{FGameplayTag, FGameplayTagExt};

fn tag_named(name: &str) -> OwnedStruct<FGameplayTag> {
    let tag = OwnedStruct::<FGameplayTag>::new();
    tag.as_ref().set_tag_name(FName::new(name).handle());
    tag
}

/// Declares the gameplay tag `name` (see `rusteal_core::gameplay_tags::declare`) and gives it;
/// call it from `rusteal_runtime::on_load!`, so the tag exists before anything asks for it.
#[track_caller]
pub fn declare_gameplay_tag(name: &str, comment: &str) -> RustealResult<OwnedStruct<FGameplayTag>> {
    rusteal_core::gameplay_tags::declare(name, comment)?;
    Ok(tag_named(name))
}

/// The gameplay tag `name`, or an error when the project has no such tag.
#[track_caller]
pub fn gameplay_tag(name: &str) -> RustealResult<OwnedStruct<FGameplayTag>> {
    if rusteal_core::gameplay_tags::is_registered(name) {
        Ok(tag_named(name))
    } else {
        Err(RustealError::invalid_operation(format!(
            "gameplay tags: {name} is not a gameplay tag of the project"
        )))
    }
}
