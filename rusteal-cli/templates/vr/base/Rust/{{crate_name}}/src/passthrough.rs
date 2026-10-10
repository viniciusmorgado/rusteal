use bindings::engine::{Actor, ActorExt, GameplayStatics, SceneComponent};
use bindings::openxr::{EOpenXREnvironmentBlendMode, OpenXRBlueprintFunctionLibrary};
use rusteal_runtime::runtime::{FName, LOG_DISPLAY, ulog};
use rusteal_runtime::{uclass, uclass_impl};

#[uclass(parent = Actor)]
pub struct Passthrough {
    #[component(root)]
    root: SceneComponent,
}

#[uclass_impl]
impl Passthrough {
    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        if OpenXRBlueprintFunctionLibrary::is_composition_layer_inverted_alpha_enabled()
            && is_alpha_blend_supported()
            && OpenXRBlueprintFunctionLibrary::get_environment_blend_mode()
                == EOpenXREnvironmentBlendMode::AlphaBlend
        {
            self.hide_passthrough_actors(true);
        }
    }

    #[ufunction(BlueprintCallable)]
    pub fn toggle_passthrough(&mut self) {
        if !OpenXRBlueprintFunctionLibrary::is_composition_layer_inverted_alpha_enabled()
            || !is_alpha_blend_supported()
        {
            return;
        }

        if OpenXRBlueprintFunctionLibrary::get_environment_blend_mode()
            != EOpenXREnvironmentBlendMode::AlphaBlend
        {
            OpenXRBlueprintFunctionLibrary::set_environment_blend_mode(
                EOpenXREnvironmentBlendMode::AlphaBlend,
            );
            self.hide_passthrough_actors(true);
        } else {
            OpenXRBlueprintFunctionLibrary::set_environment_blend_mode(
                EOpenXREnvironmentBlendMode::Opaque,
            );
            self.hide_passthrough_actors(false);
        }
    }

    #[ufunction(BlueprintCallable)]
    pub fn hide_passthrough_actors(&self, hide: bool) {
        let me = self.as_ref();

        for actor in GameplayStatics::get_all_actors_with_tag(
            me.upcast_to(),
            FName::new("Passthrough").handle(),
        ) {
            let Ok(actor) = actor.checked() else {
                continue;
            };

            actor.set_actor_hidden_in_game(hide);

            if !actor.actor_has_tag(FName::new("Floor").handle()) {
                actor.set_actor_enable_collision(!hide);
            }
        }
    }
}

fn is_alpha_blend_supported() -> bool {
    let supported = OpenXRBlueprintFunctionLibrary::get_supported_environment_blend_modes()
        .contains(&EOpenXREnvironmentBlendMode::AlphaBlend);

    if supported {
        ulog!(
            LOG_DISPLAY,
            "Environment Blend Mode Alpha Blend Supported: true"
        );
    } else {
        ulog!(
            LOG_DISPLAY,
            "The Active OpenXR runtime does NOT support the Environment Blend Mode Alpha Blend. Passthrough support may be available through the use of an OpenXR Vendor Plugin."
        );
    }

    supported
}
