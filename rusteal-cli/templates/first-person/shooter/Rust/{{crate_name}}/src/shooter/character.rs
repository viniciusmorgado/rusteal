// ShooterCharacter: the Shooter variant's `AShooterCharacter` in Rust, a
// player controllable first person shooter character. It is a
// `FirstPersonCharacter`, as the C++ class is an `ATP_FirstPersonCharacter`;
// it holds a weapon inventory (a `ShooterWeaponHolder`) and has health and a
// death.
//
// The C++ class tells its player controller about its bullet count and health
// through two delegates the controller subscribes to; a Rust class declares
// no delegates, so it calls its controller (`ShooterPlayerController`) itself.
// `TakeDamage` is a C++ virtual whose Blueprint event is `ReceiveAnyDamage`.

use bindings::engine::{
    Actor, ActorExt, AnimMontage, CharacterExt, CharacterMovementComponentExt, Controller,
    DamageType, EAttachmentRule, ECollisionEnabled, EDrawDebugTrace, ETraceTypeQuery,
    FHitResultExt, GameplayStatics, KismetMathLibrary, KismetSystemLibrary, MovementComponentExt,
    PawnExt, PawnNoiseEmitterComponent, PrimitiveComponentExt, SceneComponentExt,
    SkeletalMeshComponentExt,
};
use bindings::enhanced_input::{ETriggerEvent, InputAction};
use bindings::prelude::*;
use glam::DVec3;
use rusteal_runtime::runtime::{
    FName, LOG_WARNING, Rotator, RustealResult, SubclassOf, UObjectRef, UeArray, ulog,
};
use rusteal_runtime::{uclass, uclass_impl};

use super::game_mode::ShooterGameMode;
use super::model;
use super::player_controller::ShooterPlayerController;
use super::weapon::ShooterWeapon;
use super::weapon_holder::ShooterWeaponHolder;
use crate::character::FirstPersonCharacter;

/// `GetCharacterMovement()->RotationRate = FRotator(0.0f, 600.0f, 0.0f)`.
const ROTATION_RATE_YAW: f64 = 600.0;

#[uclass(parent = FirstPersonCharacter)]
pub struct ShooterCharacter {
    /// AI Noise emitter component
    #[component(name = "Pawn Noise Emitter")]
    pawn_noise_emitter: PawnNoiseEmitterComponent,

    /// Fire weapon input action
    #[uproperty(EditAnywhere, category = "Input")]
    fire_action: UObjectRef<InputAction>,

    /// Switch weapon input action
    #[uproperty(EditAnywhere, category = "Input")]
    switch_weapon_action: UObjectRef<InputAction>,

    /// Name of the first person mesh weapon socket
    #[uproperty(EditAnywhere, category = "Weapons")]
    first_person_weapon_socket: FName,

    /// Name of the third person mesh weapon socket
    #[uproperty(EditAnywhere, category = "Weapons")]
    third_person_weapon_socket: FName,

    /// Max distance to use for aim traces
    #[uproperty(EditAnywhere, category = "Aim", default = 10000.0)]
    max_aim_distance: f32,

    /// Max HP this character can have
    #[uproperty(EditAnywhere, name = "MaxHP", category = "Health", default = 500.0)]
    max_hp: f32,

    /// Current HP remaining to this character
    #[uproperty(name = "CurrentHP")]
    current_hp: f32,

    /// Team ID for this character
    #[uproperty(EditAnywhere, category = "Team", default = 0)]
    team_byte: u8,

    /// Actor tag to grant this character when it dies
    #[uproperty(EditAnywhere, category = "Team")]
    death_tag: FName,

    /// Tag to pass to weapons and projectiles to identify their AI perception noise as player-generated
    #[uproperty(EditAnywhere, category = "Tags")]
    player_tag: FName,

    /// List of weapons picked up by the character
    #[uproperty]
    owned_weapons: UeArray<UObjectRef<ShooterWeapon>>,

    /// Weapon currently equipped and ready to shoot with
    #[uproperty]
    current_weapon: UObjectRef<ShooterWeapon>,

    #[uproperty(EditAnywhere, category = "Destruction", default = 5.0)]
    respawn_time: f32,
}

#[uclass_impl]
impl ShooterCharacter {
    /// Everything `AShooterCharacter::AShooterCharacter()` sets.
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        self.set_first_person_weapon_socket(FName::new("HandGrip_R"));
        self.set_third_person_weapon_socket(FName::new("HandGrip_R"));
        self.set_death_tag(FName::new("Dead"));
        self.set_player_tag(FName::new("Player"));

        // configure movement
        let movement = self.as_ref().checked()?.get_character_movement().checked()?;
        movement.set_rotation_rate(&FRotator::from_rotator(Rotator::new(0.0, ROTATION_RATE_YAW, 0.0)));
        Ok(())
    }

    /// Gameplay initialization
    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        if let Ok(mut parent) = FirstPersonCharacter::from_obj(self.as_ref()) {
            parent.receive_begin_play();
        }

        // reset HP to max
        self.set_current_hp(self.max_hp());

        // update the HUD
        self.broadcast_damaged(1.0);
    }

    /// Gameplay cleanup
    #[ufunction(Override)]
    fn receive_end_play(&mut self, _end_play_reason: u8) {
        // clear the respawn timer
        KismetSystemLibrary::k2_clear_timer(self.as_ref().upcast_to(), "OnRespawn");
    }

    /// The pawn's player input component is set up: bind the input actions.
    #[ufunction(Override)]
    fn receive_restarted(&mut self) {
        // base class handles move, aim and jump inputs
        if let Ok(mut parent) = FirstPersonCharacter::from_obj(self.as_ref()) {
            parent.receive_restarted();
        }
        if let Err(e) = self.bind_weapon_input() {
            ulog!(LOG_WARNING, "[Shooter] failed to bind the weapon input: {e}");
        }
    }

    /// Handle incoming damage
    #[ufunction(Override)]
    fn receive_any_damage(
        &mut self,
        damage: f32,
        _damage_type: UObjectRef<DamageType>,
        _instigated_by: UObjectRef<Controller>,
        _damage_causer: UObjectRef<Actor>,
    ) {
        // ignore if already dead
        if self.current_hp() <= 0.0 {
            return;
        }

        // Reduce HP
        self.set_current_hp(self.current_hp() - damage);

        // Have we depleted HP?
        if self.current_hp() <= 0.0 {
            self.die();
        }

        // update the HUD
        self.broadcast_damaged((self.current_hp() / self.max_hp()).max(0.0));
    }

    /// Handles aim inputs from either controls or UI interfaces
    #[ufunction(BlueprintCallable)]
    fn do_aim(&mut self, yaw: f32, pitch: f32) {
        // only route inputs if the character is not dead
        if !self.is_dead()
            && let Ok(mut parent) = FirstPersonCharacter::from_obj(self.as_ref())
        {
            parent.do_aim(yaw, pitch);
        }
    }

    /// Handles move inputs from either controls or UI interfaces
    #[ufunction(BlueprintCallable)]
    fn do_move(&mut self, right: f32, forward: f32) {
        // only route inputs if the character is not dead
        if !self.is_dead()
            && let Ok(mut parent) = FirstPersonCharacter::from_obj(self.as_ref())
        {
            parent.do_move(right, forward);
        }
    }

    /// Handles jump start inputs from either controls or UI interfaces
    #[ufunction(BlueprintCallable)]
    fn do_jump_start(&mut self) {
        // only route inputs if the character is not dead
        if !self.is_dead()
            && let Ok(mut parent) = FirstPersonCharacter::from_obj(self.as_ref())
        {
            parent.do_jump_start();
        }
    }

    /// Handles jump end inputs from either controls or UI interfaces
    #[ufunction(BlueprintCallable)]
    fn do_jump_end(&mut self) {
        // only route inputs if the character is not dead
        if !self.is_dead()
            && let Ok(mut parent) = FirstPersonCharacter::from_obj(self.as_ref())
        {
            parent.do_jump_end();
        }
    }

    /// Handles start firing input
    #[ufunction(BlueprintCallable)]
    fn do_start_firing(&mut self) {
        // fire the current weapon
        if !self.is_dead()
            && let Ok(mut weapon) = ShooterWeapon::from_obj(self.current_weapon())
        {
            weapon.start_firing();
        }
    }

    /// Handles stop firing input
    #[ufunction(BlueprintCallable)]
    fn do_stop_firing(&mut self) {
        // stop firing the current weapon
        if !self.is_dead()
            && let Ok(mut weapon) = ShooterWeapon::from_obj(self.current_weapon())
        {
            weapon.stop_firing();
        }
    }

    /// Handles switch weapon input
    #[ufunction(BlueprintCallable)]
    fn do_switch_weapon(&mut self) {
        // ensure we have at least two weapons two switch between
        let Ok(owned_weapons) = self.owned_weapons().to_vec() else {
            return;
        };
        if owned_weapons.len() < 2 || self.is_dead() {
            return;
        }

        // deactivate the old weapon
        if let Ok(mut weapon) = ShooterWeapon::from_obj(self.current_weapon()) {
            weapon.deactivate_weapon();
        }

        // find the index of the current weapon in the owned list
        let current = self.current_weapon();
        let index = owned_weapons.iter().position(|w| *w == current).unwrap_or(0);

        // set the new weapon as current
        let next = owned_weapons[model::next_weapon_index(index, owned_weapons.len())];
        self.set_current_weapon(next);

        // activate the new weapon
        if let Ok(mut weapon) = ShooterWeapon::from_obj(next) {
            weapon.activate_weapon(self.player_tag());
        }
    }

    /// Called to allow Blueprint code to react to this character's death
    #[ufunction(BlueprintImplementableEvent, name = "BP_OnDeath")]
    fn bp_on_death(&self) {}

    /// Called from the respawn timer to destroy this character and force the PC to respawn
    #[ufunction]
    fn on_respawn(&mut self) {
        // destroy the character to force the PC to respawn
        if let Ok(me) = self.as_ref().checked() {
            me.k2_destroy_actor();
        }
    }
}

impl ShooterCharacter {
    /// This character as its parent class, whose components it has.
    fn base(&self) -> RustealResult<FirstPersonCharacter> {
        FirstPersonCharacter::from_obj(self.as_ref())
    }

    /// Returns true if the character is dead
    pub fn is_dead(&self) -> bool {
        // the character is dead if their current HP drops to zero
        self.current_hp() <= 0.0
    }

    /// Sets the team ID for this character
    pub fn set_team(&self, team: u8) {
        self.set_team_byte(team);
    }

    /// What `AShooterCharacter::SetupPlayerInputComponent` binds beyond the parent.
    fn bind_weapon_input(&self) -> RustealResult<()> {
        let me = self.as_ref();
        let pawn = me.checked()?;
        if !pawn.is_player_controlled() || !pawn.is_locally_controlled() {
            return Ok(());
        }

        // Firing
        bind_action(&me, self.fire_action(), ETriggerEvent::Started, "DoStartFiring")?;
        bind_action(&me, self.fire_action(), ETriggerEvent::Completed, "DoStopFiring")?;

        // Switch weapon
        bind_action(&me, self.switch_weapon_action(), ETriggerEvent::Triggered, "DoSwitchWeapon")?;
        Ok(())
    }

    /// The controller, the `OnBulletCountUpdated` and `OnDamaged` listener.
    fn listener(&self) -> Option<ShooterPlayerController> {
        let controller = self.as_ref().checked().ok()?.get_controller();
        ShooterPlayerController::from_obj(controller).ok()
    }

    /// `OnDamaged.Broadcast(LifePercent)`
    fn broadcast_damaged(&self, life_percent: f32) {
        if let Some(mut controller) = self.listener() {
            controller.on_pawn_damaged(life_percent);
        }
    }

    /// `OnBulletCountUpdated.Broadcast(MagazineSize, Bullets)`
    fn broadcast_bullet_count(&self, magazine_size: i32, bullets: i32) {
        if let Some(mut controller) = self.listener() {
            controller.on_bullet_count_updated(magazine_size, bullets);
        }
    }

    /// Returns the owned weapon of the given class, if any
    fn find_weapon_of_type(&self, weapon_class: SubclassOf<ShooterWeapon>) -> Option<UObjectRef<ShooterWeapon>> {
        // check each owned weapon
        self.owned_weapons()
            .to_vec()
            .ok()?
            .into_iter()
            .find(|weapon| {
                KismetMathLibrary::class_is_child_of(
                    GameplayStatics::get_object_class(weapon.upcast_to()),
                    weapon_class.upcast_to(),
                )
            })
    }

    /// Called when this character's HP is depleted
    fn die(&mut self) {
        // deactivate the weapon
        if let Ok(mut weapon) = ShooterWeapon::from_obj(self.current_weapon()) {
            weapon.deactivate_weapon();
        }

        // increment the team score
        let world = self.as_ref().upcast_to();
        if let Ok(mut game_mode) = ShooterGameMode::from_obj(GameplayStatics::get_game_mode(world)) {
            game_mode.increment_team_score(self.team_byte());
        }

        let Ok(me) = self.as_ref().checked() else {
            return;
        };

        // grant the death tag to the character
        let _ = me.tags().push(&self.death_tag().handle());

        // stop character movement
        if let Ok(movement) = me.get_character_movement().checked() {
            movement.stop_movement_immediately();
            movement.disable_movement();
        }

        // disable collision
        if let Ok(capsule) = me.get_capsule_component().checked() {
            capsule.set_collision_enabled(ECollisionEnabled::NoCollision);
        }

        // disable controls
        me.disable_input(UObjectRef::null());

        // reset the bullet counter UI
        self.broadcast_bullet_count(0, 0);

        // call the BP handler
        self.bp_on_death();

        // schedule character respawn
        KismetSystemLibrary::k2_set_timer(
            self.as_ref().upcast_to(),
            "OnRespawn",
            self.respawn_time(),
            false,
            None,
            None,
            None,
        );
    }
}

impl ShooterWeaponHolder for ShooterCharacter {
    fn attach_weapon_meshes(&mut self, weapon: &ShooterWeapon) {
        let Ok(me) = self.as_ref().checked() else {
            return;
        };
        let snap = EAttachmentRule::SnapToTarget;

        // attach the weapon actor
        if let Ok(weapon_actor) = weapon.as_ref().checked() {
            weapon_actor.k2_attach_to_actor(me.as_ref().upcast_to(), FName::NONE.handle(), snap, snap, snap, false);
        }

        // attach the weapon meshes
        if let (Ok(first_person), Ok(arms)) = (
            weapon.first_person_mesh().and_then(|m| m.checked()),
            self.base().and_then(|base| base.first_person_mesh()?.checked()),
        ) {
            first_person.k2_attach_to_component(
                arms.as_ref().upcast_to(),
                self.first_person_weapon_socket().handle(),
                snap,
                snap,
                snap,
                false,
            );
        }
        if let (Ok(third_person), Ok(mesh)) = (
            weapon.third_person_mesh().and_then(|m| m.checked()),
            me.get_mesh().checked(),
        ) {
            third_person.k2_attach_to_component(
                mesh.as_ref().upcast_to(),
                self.first_person_weapon_socket().handle(),
                snap,
                snap,
                snap,
                false,
            );
        }
    }

    fn play_firing_montage(&mut self, _montage: UObjectRef<AnimMontage>) {
        // stub
    }

    fn add_weapon_recoil(&mut self, recoil: f32) {
        // apply the recoil as pitch input
        if let Ok(me) = self.as_ref().checked() {
            me.add_controller_pitch_input(recoil);
        }
    }

    fn update_weapon_hud(&mut self, current_ammo: i32, magazine_size: i32) {
        self.broadcast_bullet_count(magazine_size, current_ammo);
    }

    fn get_weapon_target_location(&mut self) -> DVec3 {
        // trace ahead from the camera viewpoint
        let Ok(camera) = self.base().and_then(|base| base.first_person_camera_component()?.checked()) else {
            return DVec3::ZERO;
        };
        let start = camera.k2_get_component_location().to_dvec3();
        let end = start + camera.get_forward_vector().to_dvec3() * f64::from(self.max_aim_distance());
        let me = self.as_ref().upcast_to::<Actor>();

        let (hit, out_hit) = KismetSystemLibrary::line_trace_single(
            me.upcast_to(),
            &FVector::from_dvec3(start),
            &FVector::from_dvec3(end),
            ETraceTypeQuery::TraceTypeQuery1,
            false,
            &[me],
            EDrawDebugTrace::None,
            true,
            &Default::default(),
            &Default::default(),
            None,
        );

        // return either the impact point or the trace end
        if hit {
            out_hit.as_ref().get_impact_point().to_dvec3()
        } else {
            end
        }
    }

    fn add_weapon_class(&mut self, weapon_class: SubclassOf<ShooterWeapon>) {
        // do we already own this weapon?
        if self.find_weapon_of_type(weapon_class).is_some() {
            return;
        }
        let Ok(added_weapon) = ShooterWeapon::spawn_for(self.as_ref().upcast_to(), weapon_class) else {
            return;
        };

        // add the weapon to the owned list
        let _ = self.owned_weapons().push(&added_weapon);

        // if we have an existing weapon, deactivate it
        if let Ok(mut weapon) = ShooterWeapon::from_obj(self.current_weapon()) {
            weapon.deactivate_weapon();
        }

        // switch to the new weapon
        self.set_current_weapon(added_weapon);
        if let Ok(mut weapon) = ShooterWeapon::from_obj(added_weapon) {
            weapon.activate_weapon(self.player_tag());
        }
    }

    fn on_weapon_activated(&mut self, weapon: &ShooterWeapon) {
        // update the bullet counter
        self.broadcast_bullet_count(weapon.magazine_size(), weapon.bullet_count());

        // set the character mesh AnimInstances
        if let Ok(arms) = self.base().and_then(|base| base.first_person_mesh()?.checked()) {
            arms.set_anim_instance_class(weapon.first_person_anim_instance_class().upcast_to());
        }
        if let Ok(mesh) = self.as_ref().checked().and_then(|me| me.get_mesh().checked()) {
            mesh.set_anim_instance_class(weapon.third_person_anim_instance_class().upcast_to());
        }
    }

    fn on_weapon_deactivated(&mut self, _weapon: &ShooterWeapon) {
        // unused
    }

    fn on_semi_weapon_refire(&mut self) {
        // unused
    }
}
