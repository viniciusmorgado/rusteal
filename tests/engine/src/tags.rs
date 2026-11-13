use bindings::gameplay_tags::BlueprintGameplayTagLibrary;
use bindings::prelude::*;

use crate::runner::{Context, Outcome};

const CHILD: &str = "Rusteal.Test.Declared.Child";
const PARENT: &str = "Rusteal.Test.Declared";

pub fn register() {
    if let Err(error) = declare_gameplay_tag(CHILD, "Declared by the Rusteal tests") {
        log::error!("[RustealTests] cannot declare {CHILD}: {error:?}");
    }
}

pub fn declared_tag_matches_its_parent(_context: &Context) -> Outcome {
    Outcome::Done((|| {
        let e = |e| format!("{e:?}");
        let child = gameplay_tag(CHILD).map_err(e)?;
        let parent = gameplay_tag(PARENT).map_err(e)?;

        if gameplay_tag("Rusteal.Test.Never.Declared").is_ok() {
            return Err("a tag nobody declared was found".to_string());
        }

        if declare_gameplay_tag("Rusteal..Bad", "").is_ok() {
            return Err("a tag with an empty part was declared".to_string());
        }

        let container = BlueprintGameplayTagLibrary::make_gameplay_tag_container_from_tag(&child);
        let parent_match = BlueprintGameplayTagLibrary::has_tag(&container, &parent, false);
        let parent_exact = BlueprintGameplayTagLibrary::has_tag(&container, &parent, true);
        let child_exact = BlueprintGameplayTagLibrary::has_tag(&container, &child, true);

        if parent_match && !parent_exact && child_exact {
            Ok(())
        } else {
            Err(format!(
                "the parent matches: {parent_match}, exactly: {parent_exact}; the child exactly: {child_exact}"
            ))
        }
    })())
}
