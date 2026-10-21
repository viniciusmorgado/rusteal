use bindings::engine::ActorExt;
use bindings::gameplay_tags::{
    BlueprintGameplayTagLibrary, FGameplayTag, FGameplayTagContainer, FGameplayTagExt,
    GameplayTagAssetInterface,
};
use bindings::prelude::*;
use bindings::rusteal::RustealTaggedActor;
use rusteal_runtime::runtime::{FName, OwnedStruct, Transform, UStructRef};
use rusteal_runtime::{uclass, uclass_impl};

use crate::runner::{Context, Outcome};

const TAG: &str = "Rusteal.Test.Tagged";

fn tag(name: &str) -> OwnedStruct<FGameplayTag> {
    let tag = OwnedStruct::<FGameplayTag>::new();
    tag.as_ref().set_tag_name(FName::new(name).handle());
    tag
}

#[uclass(parent = RustealTaggedActor)]
pub struct TaggedActor {}

#[uclass_impl]
impl TaggedActor {
    #[ufunction(Override)]
    fn receive_get_owned_gameplay_tags(&self, tag_container: UStructRef<FGameplayTagContainer>) {
        tag_container.assign(&BlueprintGameplayTagLibrary::make_gameplay_tag_container_from_tag(
            &tag(TAG),
        ));
    }
}

pub fn cpp_only_interface_reaches_rust(context: &Context) -> Outcome {
    Outcome::Done((|| {
        let e = |e| format!("{e:?}");
        let transform = FTransform::from_transform(Transform::IDENTITY);
        let actor = context
            .world
            .spawn_actor::<TaggedActor>(&transform)
            .map_err(e)?;

        let tagged = actor.cast::<GameplayTagAssetInterface>().map_err(e)?;
        let owned = BlueprintGameplayTagLibrary::get_owned_gameplay_tags(tagged);

        let has = BlueprintGameplayTagLibrary::has_tag(&owned, &tag(TAG), true);
        let has_other =
            BlueprintGameplayTagLibrary::has_tag(&owned, &tag("Rusteal.Test.Other"), true);

        actor.checked().map_err(e)?.k2_destroy_actor();

        if has && !has_other {
            Ok(())
        } else {
            Err(format!(
                "the owned tags have {TAG}: {has}, another tag: {has_other}"
            ))
        }
    })())
}
