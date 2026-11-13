use std::time::Duration;

use bindings::engine::{EQuitPreference, GameplayStatics, KismetSystemLibrary, Texture};
use bindings::head_mounted_display::EOrientPositionSelector;
use bindings::prelude::*;
use bindings::umg::{Button, ButtonExt, UserWidget};
use bindings::xr_base::{HeadMountedDisplayFunctionLibrary, XRLoadingScreenFunctionLibrary};
use glam::{DVec2, DVec3};
use rusteal_runtime::runtime::timer::set_timer;
use rusteal_runtime::runtime::{FName, RustealResult, UObjectRef, method};
use rusteal_runtime::{uclass, uclass_impl};

use crate::passthrough::Passthrough;

#[uclass(parent = UserWidget)]
pub struct MenuWidget {
    #[uproperty(BindWidget, BlueprintReadOnly)]
    restart_button: UObjectRef<Button>,

    #[uproperty(BindWidget, BlueprintReadOnly)]
    reset_orientation_button: UObjectRef<Button>,

    #[uproperty(BindWidget, BlueprintReadOnly)]
    passthrough_button: UObjectRef<Button>,

    #[uproperty(BindWidget, BlueprintReadOnly)]
    exit_button: UObjectRef<Button>,

    #[uproperty(EditAnywhere, category = "Restart")]
    loading_screen_texture: UObjectRef<Texture>,

    #[uproperty(EditAnywhere, category = "Restart", default = 3.0)]
    restart_delay: f32,

    #[uproperty(EditAnywhere, category = "Restart")]
    restart_level_name: String,
}

#[uclass_impl]
impl MenuWidget {
    #[ufunction(Override)]
    fn construct(&mut self) {
        if let Err(e) = self.bind_buttons() {
            log::warn!("[VR] the menu buttons are not bound: {e}");
        }
    }

    #[ufunction(BlueprintCallable)]
    pub fn restart(&mut self) {
        let (_, position) = HeadMountedDisplayFunctionLibrary::get_orientation_and_position();

        XRLoadingScreenFunctionLibrary::set_loading_screen(
            self.loading_screen_texture(),
            &FVector2D::from_dvec2(DVec2::new(4.0, 4.0)),
            &FVector::from_dvec3(DVec3::new(10.0, 0.0, position.to_dvec3().z / 100.0)),
            Some(false),
            Some(true),
        );

        let restarted = set_timer(
            &self.as_ref(),
            Duration::from_secs_f32(self.restart_delay()),
            false,
            method(self, Self::open_restart_level),
        );

        if let Err(e) = restarted {
            log::warn!("[VR] the restart timer was not set: {e}");
        }
    }

    #[ufunction]
    fn open_restart_level(&mut self) {
        let me = self.as_ref();
        let name = self.restart_level_name();

        let level = if name.is_empty() {
            GameplayStatics::get_current_level_name(me.upcast_to(), Some(true))
        } else {
            name
        };

        GameplayStatics::open_level(
            me.upcast_to(),
            FName::new(&level).handle(),
            Some(true),
            None,
        );
    }

    #[ufunction(BlueprintCallable)]
    pub fn exit(&mut self) {
        KismetSystemLibrary::quit_game(
            self.as_ref().upcast_to(),
            UObjectRef::null(),
            EQuitPreference::Quit,
            false,
        );
    }

    #[ufunction(BlueprintCallable)]
    pub fn reset_orientation(&mut self) {
        HeadMountedDisplayFunctionLibrary::reset_orientation_and_position(
            Some(0.0),
            Some(EOrientPositionSelector::OrientationAndPosition),
        );
    }

    #[ufunction(BlueprintCallable)]
    pub fn toggle_passthrough(&mut self) {
        let world = self.as_ref().get_world();

        let first = world
            .and_then(|world| world.get_all_actors_of_class::<Passthrough>())
            .ok()
            .and_then(|actors| actors.into_iter().next());

        if let Some(actor) = first
            && let Ok(mut passthrough) = Passthrough::from_obj(actor)
        {
            passthrough.toggle_passthrough();
        }
    }
}

impl MenuWidget {
    fn bind_buttons(&self) -> RustealResult<()> {
        for (button, handler) in [
            (self.restart_button(), method(self, Self::restart)),
            (self.exit_button(), method(self, Self::exit)),
            (
                self.reset_orientation_button(),
                method(self, Self::reset_orientation),
            ),
            (
                self.passthrough_button(),
                method(self, Self::toggle_passthrough),
            ),
        ] {
            button.checked()?.on_clicked().add(handler)?.detach();
        }

        Ok(())
    }
}
