use std::time::Duration;

use bindings::core_ue::Object;
use bindings::engine::{GameModeBase, GameplayStatics, KismetSystemLibrary, SoundBase};
use bindings::prelude::*;
use rusteal_runtime::runtime::{SoftObjectRef, UObjectRef, http, task, timer};
use rusteal_runtime::{uclass, uclass_impl};

#[uclass(parent = GameModeBase)]
pub struct FirstPersonGameMode {}

#[uclass(parent = GameModeBase)]
pub struct IntroGameMode {
    #[uproperty(EditAnywhere, category = "Welcome")]
    welcome_sound: SoftObjectRef<SoundBase>,

    #[uproperty(EditAnywhere, category = "Welcome", default = 1.5)]
    welcome_delay: f32,

    #[uproperty(EditAnywhere, category = "Welcome")]
    welcome_url: String,

    #[uproperty(EditAnywhere, category = "Welcome", default = "Welcome to Rusteal")]
    welcome_message: String,
}

#[uclass_impl]
impl IntroGameMode {
    #[class_defaults]
    fn class_defaults(&mut self) {
        self.set_welcome_sound(&SoftObjectRef::new(
            "/Engine/EngineSounds/1kSineTonePing.1kSineTonePing",
        ));
    }

    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        let me = self.as_ref();

        let welcome = Self::welcome(
            me,
            self.welcome_sound(),
            self.welcome_delay(),
            self.welcome_url(),
            self.welcome_message(),
        );

        let _task = task::spawn_local(&me, welcome);
    }
}

impl IntroGameMode {
    async fn welcome(
        me: UObjectRef<GameModeBase>,
        sound: SoftObjectRef<SoundBase>,
        delay: f32,
        url: String,
        fallback: String,
    ) {
        let sound = if sound.is_null() {
            None
        } else {
            match sound.load_async().await {
                Ok(sound) => Some(sound),
                Err(e) => {
                    log::warn!("[Intro] the welcome sound did not load: {e}");
                    None
                }
            }
        };

        timer::sleep(&me, Duration::from_secs_f32(delay.max(0.0))).await;

        let message = if url.is_empty() {
            fallback
        } else {
            match http::get(&url).timeout(Duration::from_secs(5)).send().await {
                Ok(response) if response.is_success() => response.text().trim().to_string(),
                Ok(response) => {
                    log::warn!("[Intro] {url} answered {}", response.status);
                    fallback
                }
                Err(e) => {
                    log::warn!("[Intro] no welcome message from {url}: {e}");
                    fallback
                }
            }
        };

        let world: UObjectRef<Object> = me.upcast_to();

        if let Some(sound) = sound {
            GameplayStatics::play_sound2_d(world, sound, None, None, None, None, None, None);
        }

        KismetSystemLibrary::print_string(
            world,
            Some(&message),
            Some(true),
            Some(true),
            &FLinearColor::from_linear_color(rusteal_runtime::runtime::LinearColor::new(
                0.9, 0.6, 0.2, 1.0,
            )),
            Some(8.0),
            None,
        );
    }
}
