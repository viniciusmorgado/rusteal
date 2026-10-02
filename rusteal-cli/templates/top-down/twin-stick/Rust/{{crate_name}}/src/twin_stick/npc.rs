// TwinStickNPC: the TwinStick variant's `ATwinStickNPC` in Rust, a simple
// enemy run by its AI controller's StateTree: it chases the player and
// knocks it back on contact, and a projectile or an area attack destroys it,
// awarding points and maybe dropping a pickup.
//
// `Destroyed` and `NotifyHit` are C++ virtuals; here they are the
// `ReceiveDestroyed` and `ReceiveHit` events.

use bindings::engine::{
    Actor, ActorComponentExt, ActorExt, CapsuleComponentExt, Character, CharacterExt,
    CharacterMovementComponentExt, EAutoPossessAI, FHitResult, GameplayStatics,
    KismetMathLibrary, KismetSystemLibrary, MovementComponentExt, PawnExt, PrimitiveComponent,
    PrimitiveComponentExt,
};
use bindings::prelude::*;
use rusteal_runtime::runtime::{FName, Rotator, RustealResult, SubclassOf, UObjectRef, UStructRef};
use rusteal_runtime::{uclass, uclass_impl};

use super::character::TwinStickCharacter;
use super::game_mode::TwinStickGameMode;
use super::npc_destruction::TwinStickNPCDestruction;
use super::pickup::TwinStickPickup;

/// The capsule and movement setup `ATwinStickNPC::ATwinStickNPC()` gives the NPC.
const CAPSULE_RADIUS: f32 = 45.0;
const GRAVITY_SCALE: f32 = 1.5;
const MAX_ACCELERATION: f32 = 1000.0;
const BRAKING_FRICTION: f32 = 1.0;
const MAX_WALK_SPEED: f32 = 200.0;
const MAX_WALK_SPEED_CROUCHED: f32 = 100.0;
const ROTATION_RATE_YAW: f64 = 640.0;
const AVOIDANCE_CONSIDERATION_RADIUS: f32 = 250.0;
const AVOIDANCE_WEIGHT: f32 = 1.0;

#[uclass(parent = Character)]
pub struct TwinStickNPC {
    /// Score to award when this NPC is destroyed
    #[uproperty(EditAnywhere, category = "Score", default = 1)]
    score: i32,

    /// Percentage chance of spawning a pickup
    #[uproperty(EditAnywhere, category = "Pickup", default = 10)]
    pickup_spawn_chance: i32,

    /// Type of pickup to spawn on death
    #[uproperty(EditAnywhere, category = "Pickup")]
    pickup_class: SubclassOf<TwinStickPickup>,

    /// Type of destruction proxy to spawn on death
    #[uproperty(EditAnywhere, category = "Destruction")]
    destruction_proxy_class: SubclassOf<TwinStickNPCDestruction>,

    /// Time to wait after this NPC is hit before destroying it
    #[uproperty(EditAnywhere, category = "Pickup", default = 0.1)]
    deferred_destruction_time: f32,

    /// If true, this NPC has already been hit by a projectile and is being destroyed. Exposed to BP so it can be read by StateTree
    #[uproperty(VisibleAnywhere, BlueprintReadOnly, category = "NPC", default = false)]
    b_hit: bool,
}

#[uclass_impl]
impl TwinStickNPC {
    /// Everything `ATwinStickNPC::ATwinStickNPC()` sets.
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        let me = self.as_ref().checked()?;

        // ensure we spawn an AI controller when we're spawned
        me.set_auto_possess_ai(EAutoPossessAI::PlacedInWorldOrSpawned);

        // configure the inherited components
        let capsule = me.get_capsule_component().checked()?;
        capsule.set_capsule_radius(CAPSULE_RADIUS, Some(true));
        capsule.set_notify_rigid_body_collision(true);
        me.get_mesh()
            .checked()?
            .set_collision_profile_name(FName::new("NoCollision").handle(), Some(true));

        let movement = me.get_character_movement().checked()?;
        movement.set_gravity_scale(GRAVITY_SCALE);
        movement.set_max_acceleration(MAX_ACCELERATION);
        movement.set_braking_friction(BRAKING_FRICTION);
        movement.set_max_walk_speed(MAX_WALK_SPEED);
        movement.set_max_walk_speed_crouched(MAX_WALK_SPEED_CROUCHED);
        movement.set_rotation_rate(&FRotator::from_rotator(Rotator::new(0.0, ROTATION_RATE_YAW, 0.0)));
        movement.set_orient_rotation_to_movement(true);
        movement.set_use_rvo_avoidance(true);
        movement.set_avoidance_consideration_radius(AVOIDANCE_CONSIDERATION_RADIUS);
        movement.set_avoidance_weight(AVOIDANCE_WEIGHT);
        movement.set_plane_constraint_enabled(true);
        movement.set_snap_to_plane_at_start(true);
        Ok(())
    }

    /// Gameplay Initialization
    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        // increment the NPC counter so we can cap spawning if necessary
        if let Some(mut game_mode) = self.game_mode() {
            game_mode.increase_npcs();
        }
    }

    /// Gameplay cleanup
    #[ufunction(Override)]
    fn receive_end_play(&mut self, _end_play_reason: u8) {
        // clear the destruction timer
        KismetSystemLibrary::k2_clear_timer(self.as_ref().upcast_to(), "DeferredDestroy");
    }

    /// Handle destruction
    #[ufunction(Override)]
    fn receive_destroyed(&mut self) {
        // decrease the NPC counter so we can cap spawning if necessary
        if let Some(mut game_mode) = self.game_mode() {
            game_mode.decrease_npcs();
        }
    }

    /// Collision handling
    #[allow(clippy::too_many_arguments)]
    #[ufunction(Override)]
    fn receive_hit(
        &mut self,
        _my_comp: UObjectRef<PrimitiveComponent>,
        other: UObjectRef<Actor>,
        _other_comp: UObjectRef<PrimitiveComponent>,
        _b_self_moved: bool,
        _hit_location: UStructRef<FVector>,
        _hit_normal: UStructRef<FVector>,
        _normal_impulse: UStructRef<FVector>,
        _hit: UStructRef<FHitResult>,
    ) {
        // have we collided against the player? apply damage to the character
        if let Ok(player_character) = TwinStickCharacter::from_obj(other)
            && let Ok(me) = self.as_ref().checked()
        {
            player_character.handle_damage(1.0, me.get_actor_forward_vector().to_dvec3());
        }
    }

    /// Called from timer to complete the destruction process for this NPC
    #[ufunction]
    fn deferred_destroy(&mut self) {
        // destroy this actor
        if let Ok(me) = self.as_ref().checked() {
            me.k2_destroy_actor();
        }
    }
}

impl TwinStickNPC {
    fn game_mode(&self) -> Option<TwinStickGameMode> {
        TwinStickGameMode::from_obj(GameplayStatics::get_game_mode(self.as_ref().upcast_to())).ok()
    }

    /// Tells the NPC to process a projectile impact
    pub fn projectile_impact(&self) {
        // only handle damage if we haven't been hit yet
        if self.b_hit() {
            return;
        }

        // raise the hit flag
        self.set_b_hit(true);
        let Ok(me) = self.as_ref().checked() else {
            return;
        };

        // deactivate character movement
        if let Ok(movement) = me.get_character_movement().checked() {
            movement.deactivate();
        }

        // award points
        if let Some(mut game_mode) = self.game_mode() {
            game_mode.score_update(self.score());
        }

        // randomly spawn a pickup, and the NPC destruction proxy
        let transform = me.get_transform();
        let world = me.as_ref().upcast_to();
        if KismetMathLibrary::random_integer_in_range(0, 100) < self.pickup_spawn_chance() {
            let pickup = GameplayStatics::begin_deferred_actor_spawn_from_class(
                world,
                self.pickup_class().upcast_to(),
                &transform,
                None,
                None,
                None,
            );
            GameplayStatics::finish_spawning_actor(pickup, &transform, None);
        }
        let proxy = GameplayStatics::begin_deferred_actor_spawn_from_class(
            world,
            self.destruction_proxy_class().upcast_to(),
            &transform,
            None,
            None,
            None,
        );
        GameplayStatics::finish_spawning_actor(proxy, &transform, None);

        // hide this actor
        me.set_actor_hidden_in_game(true);

        // disable collision
        me.set_actor_enable_collision(false);

        // defer destruction
        KismetSystemLibrary::k2_set_timer(
            me.as_ref().upcast_to(),
            "DeferredDestroy",
            self.deferred_destruction_time(),
            false,
            None,
            None,
            None,
        );
    }
}
