use bindings::engine::{
    Actor, ActorExt, AnimInstance, AnimMontage, EFirstPersonPrimitiveType,
    ESpawnActorCollisionHandlingMethod, ESpawnActorScaleMethod, GameplayStatics, KismetMathLibrary,
    KismetSystemLibrary, Pawn, PrimitiveComponentExt, SceneComponent, SceneComponentExt,
    SkeletalMeshComponent,
};
use bindings::prelude::*;
use glam::DVec3;
use rusteal_runtime::runtime::{
    FName, LOG_WARNING, OwnedStruct, RustealResult, SubclassOf, UObjectRef, ulog,
};
use rusteal_runtime::{uclass, uclass_impl};

use super::model::{self, TriggerPull};
use super::projectile::ShooterProjectile;
use super::weapon_holder::{ShooterWeaponHolder, weapon_holder};

#[uclass(parent = Actor)]
pub struct ShooterWeapon {
    #[component(root, name = "Root")]
    root: SceneComponent,

    #[component(attach = "root", name = "First Person Mesh")]
    first_person_mesh: SkeletalMeshComponent,

    #[component(attach = "root", name = "Third Person Mesh")]
    third_person_mesh: SkeletalMeshComponent,

    #[uproperty(EditAnywhere, category = "Ammo")]
    projectile_class: SubclassOf<ShooterProjectile>,

    #[uproperty(EditAnywhere, category = "Ammo", default = 10)]
    magazine_size: i32,

    #[uproperty]
    current_bullets: i32,

    #[uproperty(EditAnywhere, category = "Animation")]
    firing_montage: UObjectRef<AnimMontage>,

    #[uproperty(EditAnywhere, category = "Animation")]
    first_person_anim_instance_class: SubclassOf<AnimInstance>,

    #[uproperty(EditAnywhere, category = "Animation")]
    third_person_anim_instance_class: SubclassOf<AnimInstance>,

    #[uproperty(EditAnywhere, category = "Aim", default = 0.0)]
    aim_variance: f32,

    #[uproperty(EditAnywhere, category = "Aim", default = 0.0)]
    firing_recoil: f32,

    #[uproperty(EditAnywhere, category = "Aim")]
    muzzle_socket_name: FName,

    #[uproperty(EditAnywhere, category = "Aim", default = 10.0)]
    muzzle_offset: f32,

    #[uproperty(EditAnywhere, category = "Refire", default = false)]
    b_full_auto: bool,

    #[uproperty(EditAnywhere, category = "Refire", default = 0.5)]
    refire_rate: f32,

    #[uproperty(EditAnywhere, category = "Perception", default = 1.0)]
    shot_loudness: f32,

    #[uproperty(EditAnywhere, category = "Perception", default = 300.0)]
    shot_noise_range: f32,

    #[uproperty(EditAnywhere, category = "Perception")]
    noise_owner_tag: FName,

    time_of_last_shot: f32,

    is_firing: bool,
}

#[uclass_impl]
impl ShooterWeapon {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        self.set_noise_owner_tag(FName::new("Shot"));

        let first_person = self.first_person_mesh()?.checked()?;
        first_person.set_collision_profile_name(FName::new("NoCollision").handle(), Some(true));
        first_person.set_first_person_primitive_type(EFirstPersonPrimitiveType::FirstPerson);
        first_person.set_only_owner_see(true);

        let third_person = self.third_person_mesh()?.checked()?;
        third_person.set_collision_profile_name(FName::new("NoCollision").handle(), Some(true));

        third_person
            .set_first_person_primitive_type(EFirstPersonPrimitiveType::WorldSpaceRepresentation);

        third_person.set_owner_no_see(true);

        Ok(())
    }

    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        if let Err(e) = self.begin_play() {
            ulog!(LOG_WARNING, "[Shooter] weapon BeginPlay failed: {e}");
        }
    }

    #[ufunction(Override)]
    fn receive_end_play(&mut self, _end_play_reason: u8) {
        self.clear_refire_timer();
    }

    #[ufunction]
    fn fire(&mut self) {
        if !self.is_firing() {
            return;
        }

        let Some(mut holder) = self.holder() else {
            return;
        };

        let target = holder.get_weapon_target_location();

        if let Err(e) = self.fire_projectile(holder.as_mut(), target) {
            ulog!(LOG_WARNING, "[Shooter] firing failed: {e}");
        }

        let world = self.as_ref().upcast_to();

        let Ok(me) = self.as_ref().checked() else {
            return;
        };

        self.set_time_of_last_shot(GameplayStatics::get_time_seconds(world) as f32);

        let pawn_owner = self.pawn_owner();

        if let Ok(pawn) = pawn_owner.checked() {
            me.make_noise(
                Some(self.shot_loudness()),
                Some(pawn_owner),
                &pawn.k2_get_actor_location(),
                Some(self.shot_noise_range()),
                Some(self.noise_owner_tag().handle()),
            );
        }

        let next = if self.b_full_auto() {
            "Fire"
        } else {
            "FireCooldownExpired"
        };

        self.set_refire_timer(next, self.refire_rate());
    }

    #[ufunction]
    fn fire_cooldown_expired(&mut self) {
        if let Some(mut holder) = self.holder() {
            holder.on_semi_weapon_refire();
        }
    }

    #[ufunction(BlueprintPure, name = "GetFirstPersonMesh")]
    fn get_first_person_mesh(&self) -> UObjectRef<SkeletalMeshComponent> {
        self.first_person_mesh().unwrap_or_default()
    }

    #[ufunction(BlueprintPure, name = "GetThirdPersonMesh")]
    fn get_third_person_mesh(&self) -> UObjectRef<SkeletalMeshComponent> {
        self.third_person_mesh().unwrap_or_default()
    }
}

impl ShooterWeapon {
    pub fn spawn_for(
        holder: UObjectRef<Pawn>,
        class: SubclassOf<ShooterWeapon>,
    ) -> RustealResult<UObjectRef<ShooterWeapon>> {
        let transform = holder.checked()?.get_transform();

        let spawned = GameplayStatics::begin_deferred_actor_spawn_from_class(
            holder.upcast_to(),
            class.upcast_to(),
            &transform,
            Some(ESpawnActorCollisionHandlingMethod::AlwaysSpawn),
            Some(holder.upcast_to()),
            Some(ESpawnActorScaleMethod::MultiplyWithRoot),
        );

        spawned.checked()?.set_instigator(holder);

        let spawned = GameplayStatics::finish_spawning_actor(
            spawned,
            &transform,
            Some(ESpawnActorScaleMethod::MultiplyWithRoot),
        );

        spawned.cast()
    }

    fn holder(&self) -> Option<Box<dyn ShooterWeaponHolder>> {
        weapon_holder(self.as_ref().checked().ok()?.get_owner())
    }

    fn pawn_owner(&self) -> UObjectRef<Pawn> {
        self.as_ref()
            .checked()
            .ok()
            .and_then(|me| me.get_owner().cast().ok())
            .unwrap_or_default()
    }

    fn begin_play(&mut self) -> RustealResult<()> {
        let me = self.as_ref().checked()?;

        let weapon = self.as_ref();

        me.get_owner()
            .checked()?
            .on_destroyed()
            .add(move |_destroyed_actor| {
                if let Ok(weapon) = weapon.checked() {
                    weapon.k2_destroy_actor();
                }
            })?
            .detach();

        self.set_current_bullets(self.magazine_size());

        if let Some(mut holder) = self.holder() {
            holder.attach_weapon_meshes(self);
        }

        Ok(())
    }

    pub fn activate_weapon(&mut self, owner_tag: FName) {
        self.set_noise_owner_tag(owner_tag);

        if let Ok(me) = self.as_ref().checked() {
            me.set_actor_hidden_in_game(false);
        }

        if let Some(mut holder) = self.holder() {
            holder.on_weapon_activated(self);
        }
    }

    pub fn deactivate_weapon(&mut self) {
        self.stop_firing();

        if let Ok(me) = self.as_ref().checked() {
            me.set_actor_hidden_in_game(true);
        }

        if let Some(mut holder) = self.holder() {
            holder.on_weapon_deactivated(self);
        }
    }

    pub fn start_firing(&mut self) {
        self.set_is_firing(true);

        let now = GameplayStatics::get_time_seconds(self.as_ref().upcast_to()) as f32;

        match model::pull_trigger(
            now - self.time_of_last_shot(),
            self.refire_rate(),
            self.b_full_auto(),
        ) {
            TriggerPull::FireNow => self.fire(),
            TriggerPull::FireIn(delay) => self.set_refire_timer("Fire", delay),
            TriggerPull::Wait => {}
        }
    }

    pub fn stop_firing(&mut self) {
        self.set_is_firing(false);

        self.clear_refire_timer();
    }

    fn set_refire_timer(&self, function: &str, delay: f32) {
        self.clear_refire_timer();

        KismetSystemLibrary::k2_set_timer(
            self.as_ref().upcast_to(),
            function,
            delay,
            false,
            None,
            None,
            None,
        );
    }

    fn clear_refire_timer(&self) {
        let me = self.as_ref().upcast_to();
        KismetSystemLibrary::k2_clear_timer(me, "Fire");
        KismetSystemLibrary::k2_clear_timer(me, "FireCooldownExpired");
    }

    fn fire_projectile(
        &mut self,
        holder: &mut dyn ShooterWeaponHolder,
        target_location: DVec3,
    ) -> RustealResult<()> {
        let me = self.as_ref().checked()?;

        let transform = self.calculate_projectile_spawn_transform(target_location)?;

        let spawned = GameplayStatics::begin_deferred_actor_spawn_from_class(
            me.as_ref().upcast_to(),
            self.projectile_class().upcast_to(),
            &transform,
            Some(ESpawnActorCollisionHandlingMethod::AlwaysSpawn),
            Some(me.get_owner()),
            Some(ESpawnActorScaleMethod::OverrideRootScale),
        );

        if let Ok(projectile) = spawned.checked() {
            projectile.set_instigator(self.pawn_owner());
        }

        let spawned = GameplayStatics::finish_spawning_actor(
            spawned,
            &transform,
            Some(ESpawnActorScaleMethod::OverrideRootScale),
        );

        if let Ok(projectile) = ShooterProjectile::from_obj(spawned) {
            projectile.set_noise_tag(self.noise_owner_tag());
        }

        holder.play_firing_montage(self.firing_montage());

        holder.add_weapon_recoil(self.firing_recoil());

        self.set_current_bullets(model::bullets_after_shot(
            self.current_bullets(),
            self.magazine_size(),
        ));

        holder.update_weapon_hud(self.current_bullets(), self.magazine_size());

        Ok(())
    }

    fn calculate_projectile_spawn_transform(
        &self,
        target_location: DVec3,
    ) -> RustealResult<OwnedStruct<FTransform>> {
        let muzzle = self
            .first_person_mesh()?
            .checked()?
            .get_socket_location(self.muzzle_socket_name().handle())
            .to_dvec3();

        let spawn = muzzle
            + (target_location - muzzle).normalize_or_zero() * f64::from(self.muzzle_offset());

        let variance =
            KismetMathLibrary::random_unit_vector().to_dvec3() * f64::from(self.aim_variance());

        let aim = KismetMathLibrary::find_look_at_rotation(
            &FVector::from_dvec3(spawn),
            &FVector::from_dvec3(target_location + variance),
        );

        Ok(KismetMathLibrary::make_transform(
            &FVector::from_dvec3(spawn),
            &aim,
            &FVector::from_dvec3(DVec3::ONE),
        ))
    }

    pub fn bullet_count(&self) -> i32 {
        self.current_bullets()
    }
}
