// ShooterNPC: the Shooter variant's `AShooterNPC` in Rust, an AI-controlled
// shooter. It is a `FirstPersonCharacter`, as the C++ class is an
// `ATP_FirstPersonCharacter`; its AI controller's StateTree decides what it
// does, it holds one weapon (a `ShooterWeaponHolder`), and dies in ragdoll.
//
// The C++ class tells its controller and its spawner it died through a
// delegate both subscribe to; a Rust class declares no delegates, so it calls
// them itself, its controller first (it subscribes first).

use bindings::engine::{
    Actor, ActorExt, AnimMontage, CharacterExt, Controller, DamageType, EAttachmentRule,
    ECollisionEnabled, EDrawDebugTrace, ETraceTypeQuery, FHitResultExt, GameplayStatics,
    KismetMathLibrary, KismetSystemLibrary, MovementComponentExt, NavMovementComponentExt,
    PawnExt, PrimitiveComponentExt, SceneComponentExt, SkeletalMeshComponentExt,
};
use bindings::prelude::*;
use glam::DVec3;
use rusteal_runtime::runtime::{FName, LOG_WARNING, RustealResult, SubclassOf, UObjectRef, ulog};
use rusteal_runtime::{uclass, uclass_impl};

use super::ai_controller::ShooterAIController;
use super::game_mode::ShooterGameMode;
use super::npc_spawner::ShooterNPCSpawner;
use super::weapon::ShooterWeapon;
use super::weapon_holder::ShooterWeaponHolder;
use crate::character::FirstPersonCharacter;

#[uclass(parent = FirstPersonCharacter)]
pub struct ShooterNPC {
    /// Current HP for this character. It dies if it reaches zero through damage
    #[uproperty(EditAnywhere, BlueprintReadOnly, name = "CurrentHP", category = "Damage", default = 100.0)]
    current_hp: f32,

    /// Name of the collision profile to use during ragdoll death
    #[uproperty(EditAnywhere, category = "Damage")]
    ragdoll_collision_profile: FName,

    /// Time to wait after death before destroying this actor
    #[uproperty(EditAnywhere, category = "Damage", default = 5.0)]
    deferred_destruction_time: f32,

    /// Team byte for this character
    #[uproperty(EditAnywhere, category = "Team", default = 1)]
    team_byte: u8,

    /// Actor tag to grant this character when it dies
    #[uproperty(EditAnywhere, category = "Team")]
    death_tag: FName,

    /// Pointer to the equipped weapon
    #[uproperty]
    weapon: UObjectRef<ShooterWeapon>,

    /// Type of weapon to spawn for this character
    #[uproperty(EditAnywhere, category = "Weapon")]
    weapon_class: SubclassOf<ShooterWeapon>,

    /// Name of the first person mesh weapon socket
    #[uproperty(EditAnywhere, BlueprintReadOnly, category = "Weapons")]
    first_person_weapon_socket: FName,

    /// Name of the third person mesh weapon socket
    #[uproperty(EditAnywhere, BlueprintReadOnly, category = "Weapons")]
    third_person_weapon_socket: FName,

    /// Max range for aiming calculations
    #[uproperty(EditAnywhere, category = "Aim", default = 10000.0)]
    aim_range: f32,

    /// Cone variance to apply while aiming
    #[uproperty(EditAnywhere, category = "Aim", default = 10.0)]
    aim_variance_half_angle: f32,

    /// Minimum vertical offset from the target center to apply when aiming
    #[uproperty(EditAnywhere, category = "Aim", default = -35.0)]
    min_aim_offset_z: f32,

    /// Maximum vertical offset from the target center to apply when aiming
    #[uproperty(EditAnywhere, category = "Aim", default = -60.0)]
    max_aim_offset_z: f32,

    /// Actor currently being targeted
    #[uproperty]
    current_aim_target: UObjectRef<Actor>,

    /// The spawner told about this NPC's death (an `OnPawnDeath` listener)
    #[uproperty]
    spawner: UObjectRef<ShooterNPCSpawner>,

    /// If true, this character is currently shooting its weapon
    is_shooting: bool,

    /// If true, this character has already died
    is_dead: bool,
}

#[uclass_impl]
impl ShooterNPC {
    /// The defaults `AShooterNPC` gives its names.
    #[class_defaults]
    fn class_defaults(&mut self) {
        self.set_ragdoll_collision_profile(FName::new("Ragdoll"));
        self.set_death_tag(FName::new("Dead"));
        self.set_first_person_weapon_socket(FName::new("HandGrip_R"));
        self.set_third_person_weapon_socket(FName::new("HandGrip_R"));
    }

    /// Gameplay initialization
    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        if let Ok(mut parent) = FirstPersonCharacter::from_obj(self.as_ref()) {
            parent.receive_begin_play();
        }

        // spawn the weapon
        match ShooterWeapon::spawn_for(self.as_ref().upcast_to(), self.weapon_class()) {
            Ok(weapon) => self.set_weapon(weapon),
            Err(e) => ulog!(LOG_WARNING, "[Shooter] NPC weapon: {e}"),
        }
    }

    /// Gameplay cleanup
    #[ufunction(Override)]
    fn receive_end_play(&mut self, _end_play_reason: u8) {
        // clear the death timer
        KismetSystemLibrary::k2_clear_timer(self.as_ref().upcast_to(), "DeferredDestruction");
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
        if self.is_dead() {
            return;
        }

        // Reduce HP
        self.set_current_hp(self.current_hp() - damage);

        // Have we depleted HP?
        if self.current_hp() <= 0.0 {
            self.die();
        }
    }

    /// Called after death to destroy the actor
    #[ufunction]
    fn deferred_destruction(&mut self) {
        if let Ok(me) = self.as_ref().checked() {
            me.k2_destroy_actor();
        }
    }
}

impl ShooterNPC {
    /// This character as its parent class, whose components it has.
    fn base(&self) -> RustealResult<FirstPersonCharacter> {
        FirstPersonCharacter::from_obj(self.as_ref())
    }

    /// Called when HP is depleted and the character should die
    fn die(&mut self) {
        // ignore if already dead
        if self.is_dead() {
            return;
        }

        // raise the dead flag
        self.set_is_dead(true);
        let Ok(me) = self.as_ref().checked() else {
            return;
        };

        // grant the death tag to the character
        let _ = me.tags().push(&self.death_tag().handle());

        // call the delegate
        if let Ok(mut controller) = ShooterAIController::from_obj(me.get_controller()) {
            controller.on_pawn_death();
        }
        if let Ok(mut spawner) = ShooterNPCSpawner::from_obj(self.spawner()) {
            spawner.on_npc_died();
        }

        // increment the team score
        if let Ok(mut game_mode) = ShooterGameMode::from_obj(GameplayStatics::get_game_mode(me.as_ref().upcast_to())) {
            game_mode.increment_team_score(self.team_byte());
        }

        // disable capsule collision
        if let Ok(capsule) = me.get_capsule_component().checked() {
            capsule.set_collision_enabled(ECollisionEnabled::NoCollision);
        }

        // stop movement
        if let Ok(movement) = me.get_character_movement().checked() {
            movement.stop_movement_immediately();
            movement.stop_active_movement();
        }

        // enable ragdoll physics on the third person mesh
        if let Ok(mesh) = me.get_mesh().checked() {
            mesh.set_collision_profile_name(self.ragdoll_collision_profile().handle(), Some(true));
            mesh.set_simulate_physics(true);
            mesh.set_physics_blend_weight(1.0);
        }

        // schedule actor destruction
        KismetSystemLibrary::k2_set_timer(
            me.as_ref().upcast_to(),
            "DeferredDestruction",
            self.deferred_destruction_time(),
            false,
            None,
            None,
            None,
        );
    }

    /// Signals this character to start shooting at the passed actor
    pub fn start_shooting(&mut self, actor_to_shoot: UObjectRef<Actor>) {
        // save the aim target
        self.set_current_aim_target(actor_to_shoot);

        // raise the flag
        self.set_is_shooting(true);

        // signal the weapon
        if let Ok(mut weapon) = ShooterWeapon::from_obj(self.weapon()) {
            weapon.start_firing();
        }
    }

    /// Signals this character to stop shooting
    pub fn stop_shooting(&mut self) {
        // lower the flag
        self.set_is_shooting(false);

        // signal the weapon
        if let Ok(mut weapon) = ShooterWeapon::from_obj(self.weapon()) {
            weapon.stop_firing();
        }
    }
}

impl ShooterWeaponHolder for ShooterNPC {
    fn attach_weapon_meshes(&mut self, weapon_to_attach: &ShooterWeapon) {
        let Ok(me) = self.as_ref().checked() else {
            return;
        };
        let snap = EAttachmentRule::SnapToTarget;

        // attach the weapon actor
        if let Ok(weapon_actor) = weapon_to_attach.as_ref().checked() {
            weapon_actor.k2_attach_to_actor(me.as_ref().upcast_to(), FName::NONE.handle(), snap, snap, snap, false);
        }

        // attach the weapon meshes
        if let (Ok(first_person), Ok(arms)) = (
            weapon_to_attach.first_person_mesh().and_then(|m| m.checked()),
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
            weapon_to_attach.third_person_mesh().and_then(|m| m.checked()),
            me.get_mesh().checked(),
        ) {
            third_person.k2_attach_to_component(
                mesh.as_ref().upcast_to(),
                self.third_person_weapon_socket().handle(),
                snap,
                snap,
                snap,
                false,
            );
        }
    }

    fn play_firing_montage(&mut self, _montage: UObjectRef<AnimMontage>) {
        // unused
    }

    fn add_weapon_recoil(&mut self, _recoil: f32) {
        // unused
    }

    fn update_weapon_hud(&mut self, _current_ammo: i32, _magazine_size: i32) {
        // unused
    }

    fn get_weapon_target_location(&mut self) -> DVec3 {
        // start aiming from the camera location
        let Ok(camera) = self.base().and_then(|base| base.first_person_camera_component()?.checked()) else {
            return DVec3::ZERO;
        };
        let aim_source = camera.k2_get_component_location().to_dvec3();

        // do we have an aim target?
        let aim_dir = match self.current_aim_target().checked() {
            Ok(target) => {
                // target the actor location
                let mut aim_target = target.k2_get_actor_location().to_dvec3();

                // apply a vertical offset to target head/feet
                aim_target.z += f64::from(KismetMathLibrary::random_float_in_range(
                    f64::from(self.min_aim_offset_z()),
                    f64::from(self.max_aim_offset_z()),
                ));

                // get the aim direction and apply randomness in a cone
                let aim_dir = (aim_target - aim_source).normalize_or_zero();
                KismetMathLibrary::random_unit_vector_in_cone_in_degrees(
                    &FVector::from_dvec3(aim_dir),
                    self.aim_variance_half_angle(),
                )
            }
            // no aim target, so just use the camera facing
            Err(_) => KismetMathLibrary::random_unit_vector_in_cone_in_degrees(
                &camera.get_forward_vector(),
                self.aim_variance_half_angle(),
            ),
        }
        .to_dvec3();

        // calculate the unobstructed aim target location
        let aim_target = aim_source + aim_dir * f64::from(self.aim_range());

        // run a visibility trace to see if there's obstructions
        let me = self.as_ref().upcast_to::<Actor>();
        let (hit, out_hit) = KismetSystemLibrary::line_trace_single(
            me.upcast_to(),
            &FVector::from_dvec3(aim_source),
            &FVector::from_dvec3(aim_target),
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
            aim_target
        }
    }

    fn add_weapon_class(&mut self, _weapon_class: SubclassOf<ShooterWeapon>) {
        // unused
    }

    fn on_weapon_activated(&mut self, _weapon: &ShooterWeapon) {
        // unused
    }

    fn on_weapon_deactivated(&mut self, _weapon: &ShooterWeapon) {
        // unused
    }

    fn on_semi_weapon_refire(&mut self) {
        // are we still shooting?
        if self.is_shooting()
            && let Ok(mut weapon) = ShooterWeapon::from_obj(self.weapon())
        {
            // fire the weapon
            weapon.start_firing();
        }
    }
}
