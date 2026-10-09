use bindings::engine::{
    Actor, ActorComponentExt, ActorExt, CapsuleComponentExt, Character, CharacterExt,
    CharacterMovementComponentExt, EAutoPossessAI, FHitResult, GameplayStatics, KismetMathLibrary,
    KismetSystemLibrary, MovementComponentExt, PawnExt, PrimitiveComponent, PrimitiveComponentExt,
};
use bindings::prelude::*;
use rusteal_runtime::runtime::{FName, Rotator, RustealResult, SubclassOf, UObjectRef, UStructRef};
use rusteal_runtime::{uclass, uclass_impl};

use super::character::TwinStickCharacter;
use super::game_mode::TwinStickGameMode;
use super::npc_destruction::TwinStickNPCDestruction;
use super::pickup::TwinStickPickup;

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
    #[uproperty(EditAnywhere, category = "Score", default = 1)]
    score: i32,

    #[uproperty(EditAnywhere, category = "Pickup", default = 10)]
    pickup_spawn_chance: i32,

    #[uproperty(EditAnywhere, category = "Pickup")]
    pickup_class: SubclassOf<TwinStickPickup>,

    #[uproperty(EditAnywhere, category = "Destruction")]
    destruction_proxy_class: SubclassOf<TwinStickNPCDestruction>,

    #[uproperty(EditAnywhere, category = "Pickup", default = 0.1)]
    deferred_destruction_time: f32,

    #[uproperty(VisibleAnywhere, BlueprintReadOnly, category = "NPC", default = false)]
    b_hit: bool,
}

#[uclass_impl]
impl TwinStickNPC {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        let me = self.as_ref().checked()?;

        me.set_auto_possess_ai(EAutoPossessAI::PlacedInWorldOrSpawned);

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

        movement.set_rotation_rate(&FRotator::from_rotator(Rotator::new(
            0.0,
            ROTATION_RATE_YAW,
            0.0,
        )));

        movement.set_orient_rotation_to_movement(true);
        movement.set_use_rvo_avoidance(true);
        movement.set_avoidance_consideration_radius(AVOIDANCE_CONSIDERATION_RADIUS);
        movement.set_avoidance_weight(AVOIDANCE_WEIGHT);
        movement.set_plane_constraint_enabled(true);
        movement.set_snap_to_plane_at_start(true);

        Ok(())
    }

    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        if let Some(mut game_mode) = self.game_mode() {
            game_mode.increase_npcs();
        }
    }

    #[ufunction(Override)]
    fn receive_end_play(&mut self, _end_play_reason: u8) {
        KismetSystemLibrary::k2_clear_timer(self.as_ref().upcast_to(), "DeferredDestroy");
    }

    #[ufunction(Override)]
    fn receive_destroyed(&mut self) {
        if let Some(mut game_mode) = self.game_mode() {
            game_mode.decrease_npcs();
        }
    }

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
        if let Ok(player_character) = TwinStickCharacter::from_obj(other)
            && let Ok(me) = self.as_ref().checked()
        {
            player_character.handle_damage(1.0, me.get_actor_forward_vector().to_dvec3());
        }
    }

    #[ufunction]
    fn deferred_destroy(&mut self) {
        if let Ok(me) = self.as_ref().checked() {
            me.k2_destroy_actor();
        }
    }
}

impl TwinStickNPC {
    fn game_mode(&self) -> Option<TwinStickGameMode> {
        TwinStickGameMode::from_obj(GameplayStatics::get_game_mode(self.as_ref().upcast_to())).ok()
    }

    pub fn projectile_impact(&self) {
        if self.b_hit() {
            return;
        }

        self.set_b_hit(true);

        let Ok(me) = self.as_ref().checked() else {
            return;
        };

        if let Ok(movement) = me.get_character_movement().checked() {
            movement.deactivate();
        }

        if let Some(mut game_mode) = self.game_mode() {
            game_mode.score_update(self.score());
        }

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

        me.set_actor_hidden_in_game(true);

        me.set_actor_enable_collision(false);

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
