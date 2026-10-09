use bindings::engine::{
    Actor, ActorComponentExt, ActorExt, Character, DamageType, ECanBeCharacterBase,
    ECollisionChannel, ECollisionEnabled, ECollisionResponse, FHitResult, FHitResultExt,
    GameplayStatics, KismetSystemLibrary, PawnExt, PrimitiveComponent, PrimitiveComponentExt,
    ProjectileMovementComponent, ProjectileMovementComponentExt, SceneComponentExt,
    SphereComponent, SphereComponentExt,
};
use bindings::prelude::*;
use glam::DVec3;
use rusteal_runtime::runtime::{
    FName, OwnedStruct, RustealResult, SubclassOf, UObjectRef, UStructRef,
};
use rusteal_runtime::{uclass, uclass_impl};

const COLLISION_RADIUS: f32 = 16.0;
const SPEED: f32 = 3000.0;

#[uclass(parent = Actor)]
pub struct ShooterProjectile {
    #[component(root, name = "Collision Component")]
    collision_component: SphereComponent,

    #[component(name = "Projectile Movement")]
    projectile_movement: ProjectileMovementComponent,

    #[uproperty(EditAnywhere, category = "Projectile|Noise", default = 3.0)]
    noise_loudness: f32,

    #[uproperty(EditAnywhere, category = "Projectile|Noise", default = 1000.0)]
    noise_range: f32,

    #[uproperty(EditAnywhere, category = "Noise")]
    noise_tag: FName,

    #[uproperty(EditAnywhere, category = "Projectile|Hit", default = 100.0)]
    physics_force: f32,

    #[uproperty(EditAnywhere, category = "Projectile|Hit", default = 25.0)]
    hit_damage: f32,

    #[uproperty(EditAnywhere, category = "Projectile|Hit")]
    hit_damage_type: SubclassOf<DamageType>,

    #[uproperty(EditAnywhere, category = "Projectile|Hit", default = false)]
    b_damage_owner: bool,

    #[uproperty(EditAnywhere, category = "Projectile|Explosion", default = false)]
    b_explode_on_hit: bool,

    #[uproperty(EditAnywhere, category = "Projectile|Explosion", default = 500.0)]
    explosion_radius: f32,

    #[uproperty(EditAnywhere, category = "Projectile|Destruction", default = 5.0)]
    deferred_destruction_time: f32,

    hit: bool,
}

#[uclass_impl]
impl ShooterProjectile {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        self.set_noise_tag(FName::new("Projectile"));

        let collision = self.collision_component()?.checked()?;
        collision.set_sphere_radius(COLLISION_RADIUS, Some(true));
        collision.set_collision_enabled(ECollisionEnabled::QueryAndPhysics);
        collision.set_collision_response_to_all_channels(ECollisionResponse::ECR_Block);
        collision.set_can_character_step_up_on(ECanBeCharacterBase::ECB_No);

        let movement = self.projectile_movement()?.checked()?;
        movement.set_initial_speed(SPEED);
        movement.set_max_speed(SPEED);
        movement.set_should_bounce(true);

        self.set_hit_damage_type(SubclassOf::<DamageType>::base());

        Ok(())
    }

    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        if let (Ok(collision), Ok(me)) = (
            self.collision_component().and_then(|c| c.checked()),
            self.as_ref().checked(),
        ) {
            collision.ignore_actor_when_moving(me.get_instigator().upcast_to(), true);
        }
    }

    #[ufunction(Override)]
    fn receive_end_play(&mut self, _end_play_reason: u8) {
        KismetSystemLibrary::k2_clear_timer(self.as_ref().upcast_to(), "OnDeferredDestruction");
    }

    #[allow(clippy::too_many_arguments)]
    #[ufunction(Override)]
    fn receive_hit(
        &mut self,
        _my_comp: UObjectRef<PrimitiveComponent>,
        other: UObjectRef<Actor>,
        other_comp: UObjectRef<PrimitiveComponent>,
        _b_self_moved: bool,
        _hit_location: UStructRef<FVector>,
        _hit_normal: UStructRef<FVector>,
        _normal_impulse: UStructRef<FVector>,
        hit: UStructRef<FHitResult>,
    ) {
        if self.hit() {
            return;
        }

        self.set_hit(true);

        let Ok(me) = self.as_ref().checked() else {
            return;
        };

        if let Ok(collision) = self.collision_component().and_then(|c| c.checked()) {
            collision.set_collision_enabled(ECollisionEnabled::NoCollision);
        }

        me.make_noise(
            Some(self.noise_loudness()),
            Some(me.get_instigator()),
            &me.k2_get_actor_location(),
            Some(self.noise_range()),
            Some(self.noise_tag().handle()),
        );

        if self.b_explode_on_hit() {
            self.explosion_check(me.k2_get_actor_location().to_dvec3());
        } else {
            self.process_hit(
                other,
                other_comp,
                hit.get_impact_point().to_dvec3(),
                -hit.get_impact_normal().to_dvec3(),
            );
        }

        self.bp_on_projectile_hit(&hit.to_owned());

        if self.deferred_destruction_time() > 0.0 {
            KismetSystemLibrary::k2_set_timer(
                me.as_ref().upcast_to(),
                "OnDeferredDestruction",
                self.deferred_destruction_time(),
                false,
                None,
                None,
                None,
            );
        } else {
            me.k2_destroy_actor();
        }
    }

    #[ufunction(BlueprintImplementableEvent, name = "BP_OnProjectileHit")]
    fn bp_on_projectile_hit(&self, hit: &OwnedStruct<FHitResult>) {}

    #[ufunction]
    fn on_deferred_destruction(&mut self) {
        if let Ok(me) = self.as_ref().checked() {
            me.k2_destroy_actor();
        }
    }
}

impl ShooterProjectile {
    fn explosion_check(&self, explosion_center: DVec3) {
        let Ok(me) = self.as_ref().checked() else {
            return;
        };

        let mut ignored = vec![me.as_ref().upcast_to::<Actor>()];

        if !self.b_damage_owner() {
            ignored.push(me.get_instigator().upcast_to());
        }

        let object_types: Vec<_> = [
            ECollisionChannel::ECC_Pawn,
            ECollisionChannel::ECC_WorldDynamic,
            ECollisionChannel::ECC_PhysicsBody,
        ]
        .into_iter()
        .filter_map(object_type_query)
        .collect();

        let (_, overlaps) = KismetSystemLibrary::sphere_overlap_components(
            me.as_ref().upcast_to(),
            &FVector::from_dvec3(explosion_center),
            self.explosion_radius(),
            &object_types,
            Default::default(),
            &ignored,
        );

        let mut damaged_actors: Vec<UObjectRef<Actor>> = Vec::new();
        let location = me.k2_get_actor_location().to_dvec3();

        for component in overlaps {
            let Ok(actor) = component.checked().map(|c| c.get_owner()) else {
                continue;
            };

            if damaged_actors.contains(&actor) {
                continue;
            }

            damaged_actors.push(actor);

            let Ok(target) = actor.checked() else {
                continue;
            };

            let explosion_dir =
                (target.k2_get_actor_location().to_dvec3() - location).normalize_or_zero();

            self.process_hit(actor, component, location, explosion_dir);
        }
    }

    fn process_hit(
        &self,
        hit_actor: UObjectRef<Actor>,
        hit_comp: UObjectRef<PrimitiveComponent>,
        hit_location: DVec3,
        hit_direction: DVec3,
    ) {
        let Ok(me) = self.as_ref().checked() else {
            return;
        };

        if let Ok(hit_character) = hit_actor.cast::<Character>()
            && (hit_actor != me.get_owner() || self.b_damage_owner())
        {
            let instigator_controller = me
                .get_instigator()
                .checked()
                .map(|pawn| pawn.get_controller())
                .unwrap_or_default();

            GameplayStatics::apply_damage(
                hit_character.upcast_to(),
                self.hit_damage(),
                instigator_controller,
                me.as_ref().upcast_to(),
                self.hit_damage_type(),
            );
        }

        if let Ok(component) = hit_comp.checked()
            && component.is_simulating_physics(None)
        {
            component.add_impulse_at_location(
                &FVector::from_dvec3(hit_direction * f64::from(self.physics_force())),
                &FVector::from_dvec3(hit_location),
                None,
            );
        }
    }
}
