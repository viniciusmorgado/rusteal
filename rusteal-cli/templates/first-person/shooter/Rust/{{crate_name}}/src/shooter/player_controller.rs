use bindings::engine::{
    Actor, ActorExt, ControllerExt, GameplayStatics, KismetMathLibrary, Pawn, PlayerController,
    PlayerStart,
};
use bindings::enhanced_input::{EnhancedInputLocalPlayerSubsystemExt, InputMappingContext};
use bindings::prelude::*;
use bindings::umg::{UserWidget, UserWidgetExt};
use rusteal_runtime::runtime::input::should_display_touch_interface;
use rusteal_runtime::runtime::{
    FName, LOG_ERROR, LOG_WARNING, OwnedStruct, RustealResult, SubclassOf, UObjectRef, UeArray,
    ulog,
};
use rusteal_runtime::{uclass, uclass_impl};

use super::character::ShooterCharacter;
use super::ui::ShooterBulletCounterUI;

#[uclass(parent = PlayerController)]
pub struct ShooterPlayerController {
    #[uproperty(EditAnywhere, category = "Input|Input Mappings")]
    default_mapping_contexts: UeArray<UObjectRef<InputMappingContext>>,

    #[uproperty(EditAnywhere, category = "Input|Input Mappings")]
    mobile_excluded_mapping_contexts: UeArray<UObjectRef<InputMappingContext>>,

    #[uproperty(EditAnywhere, category = "Input|Touch Controls")]
    mobile_controls_widget_class: SubclassOf<UserWidget>,

    #[uproperty]
    mobile_controls_widget: UObjectRef<UserWidget>,

    #[uproperty(EditAnywhere, category = "Input|Touch Controls", default = false)]
    b_force_touch_controls: bool,

    #[uproperty(EditAnywhere, category = "Shooter|Respawn")]
    character_class: SubclassOf<ShooterCharacter>,

    #[uproperty(EditAnywhere, name = "BulletCounterUIClass", category = "Shooter|UI")]
    bullet_counter_ui_class: SubclassOf<ShooterBulletCounterUI>,

    #[uproperty(EditAnywhere, category = "Shooter|Player")]
    player_pawn_tag: FName,

    #[uproperty(name = "BulletCounterUI")]
    bullet_counter_ui: UObjectRef<ShooterBulletCounterUI>,

    #[uproperty(EditAnywhere, category = "Shooter|Team")]
    team_tags: UeArray<FName>,

    team_byte: u8,
}

#[uclass_impl]
impl ShooterPlayerController {
    #[class_defaults]
    fn class_defaults(&mut self) {
        self.set_player_pawn_tag(FName::new("Player"));
    }

    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        if let Err(e) = self.setup_input() {
            ulog!(LOG_WARNING, "[Shooter] input mapping contexts failed: {e}");
        }
    }

    #[ufunction(Override)]
    fn receive_possess(&mut self, possessed_pawn: UObjectRef<Pawn>) {
        if let Err(e) = self.possess_pawn(possessed_pawn) {
            ulog!(LOG_WARNING, "[Shooter] possession failed: {e}");
        }
    }
}

impl ShooterPlayerController {
    pub fn set_team(&mut self, team: u8) {
        self.set_team_byte(team);

        if let Ok(me) = self.as_ref().checked()
            && let Ok(character) = ShooterCharacter::from_obj(me.k2_get_pawn())
        {
            character.set_team(team);
        }
    }

    pub fn on_bullet_count_updated(&mut self, magazine_size: i32, bullets: i32) {
        if let Some(ui) = self.bullet_counter() {
            ui.bp_update_bullet_counter(magazine_size, bullets);
        }
    }

    pub fn on_pawn_damaged(&mut self, life_percent: f32) {
        if let Some(ui) = self.bullet_counter() {
            ui.bp_damaged(life_percent);
        }
    }

    fn bullet_counter(&mut self) -> Option<ShooterBulletCounterUI> {
        if let Ok(ui) = ShooterBulletCounterUI::from_obj(self.bullet_counter_ui()) {
            return Some(ui);
        }

        let me: UObjectRef<PlayerController> = self.as_ref();

        if !me.checked().ok()?.is_local_player_controller() {
            return None;
        }

        match create_widget_of_class(&me, self.bullet_counter_ui_class()) {
            Ok(ui) => {
                ui.checked().ok()?.add_to_player_screen(Some(0));
                self.set_bullet_counter_ui(ui);

                ShooterBulletCounterUI::from_obj(ui).ok()
            }
            Err(_) => {
                ulog!(LOG_ERROR, "Could not spawn bullet counter widget.");

                None
            }
        }
    }

    fn setup_input(&mut self) -> RustealResult<()> {
        let me: UObjectRef<PlayerController> = self.as_ref();

        if !me.checked()?.is_local_player_controller() {
            return Ok(());
        }

        let subsystem = enhanced_input_subsystem(me)?.checked()?;
        let mut contexts = self.default_mapping_contexts().to_vec()?;

        if !self.should_use_touch_controls() {
            contexts.extend(self.mobile_excluded_mapping_contexts().to_vec()?);
        }

        for context in contexts {
            subsystem.add_mapping_context(context, 0, &OwnedStruct::new());
        }

        if self.should_use_touch_controls() {
            match create_widget_of_class(&me, self.mobile_controls_widget_class()) {
                Ok(widget) => {
                    widget.checked()?.add_to_player_screen(Some(0));
                    self.set_mobile_controls_widget(widget);
                }
                Err(_) => ulog!(LOG_ERROR, "Could not spawn mobile controls widget."),
            }
        }

        self.bullet_counter();

        Ok(())
    }

    fn possess_pawn(&mut self, possessed_pawn: UObjectRef<Pawn>) -> RustealResult<()> {
        let controller: UObjectRef<PlayerController> = self.as_ref();

        possessed_pawn
            .checked()?
            .on_destroyed()
            .add(move |destroyed_actor| {
                if let Ok(mut me) = ShooterPlayerController::from_obj(controller) {
                    me.on_pawn_destroyed(destroyed_actor);
                }
            })?
            .detach();

        if let Ok(character) = ShooterCharacter::from_obj(possessed_pawn) {
            possessed_pawn
                .checked()?
                .tags()
                .push(&self.player_pawn_tag().handle())?;

            character.set_team(self.team_byte());

            self.on_pawn_damaged(1.0);
        }

        Ok(())
    }

    fn on_pawn_destroyed(&mut self, _destroyed_actor: UObjectRef<Actor>) {
        if let Ok(ui) = ShooterBulletCounterUI::from_obj(self.bullet_counter_ui()) {
            ui.bp_update_bullet_counter(0, 0);
        }

        if let Err(e) = self.respawn() {
            ulog!(LOG_WARNING, "[Shooter] respawn failed: {e}");
        }
    }

    fn respawn(&mut self) -> RustealResult<()> {
        let team_tags = self.team_tags().to_vec()?;

        let Some(team_tag) = team_tags.get(usize::from(self.team_byte())) else {
            return Ok(());
        };

        let me = self.as_ref();

        let player_starts = GameplayStatics::get_all_actors_of_class_with_tag(
            me.upcast_to(),
            SubclassOf::<PlayerStart>::base().upcast_to(),
            team_tag.handle(),
        );

        if player_starts.is_empty() {
            return Ok(());
        }

        let last = player_starts.len() as i32 - 1;

        let player_start =
            player_starts[KismetMathLibrary::random_integer_in_range(0, last) as usize];

        let transform = player_start.checked()?.get_transform();
        let world = me.get_world()?;
        let respawned_character = world.spawn_actor_of_class(self.character_class(), &transform)?;

        me.checked()?.possess(respawned_character.upcast_to());

        Ok(())
    }

    fn should_use_touch_controls(&self) -> bool {
        should_display_touch_interface() || self.b_force_touch_controls()
    }
}
