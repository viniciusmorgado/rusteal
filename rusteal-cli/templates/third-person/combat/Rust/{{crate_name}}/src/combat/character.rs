use bindings::engine::{
    Actor, ActorExt, AnimInstanceExt, AnimMontage, CameraComponent, CameraComponentExt,
    CapsuleComponentExt, Character, CharacterExt, CharacterMovementComponentExt, Controller,
    ControllerExt, ECollisionChannel, EDrawDebugTrace, GameplayStatics, KismetSystemLibrary,
    PawnExt, PrimitiveComponentExt, SceneComponentExt, SkeletalMeshComponentExt,
    SpringArmComponent, SpringArmComponentExt,
};
use bindings::engine::{ActorComponentExt, FHitResult, FHitResultExt};
use bindings::enhanced_input::{ETriggerEvent, FInputActionValue, InputAction};
use bindings::prelude::*;
use bindings::umg::{WidgetComponent, WidgetComponentExt};
use glam::DVec3;
use rusteal_runtime::runtime::{
    FName, LOG_DISPLAY, LOG_WARNING, OwnedStruct, RustealResult, UObjectRef, UStructRef, UeArray,
    ulog,
};
use rusteal_runtime::{uclass, uclass_impl};

use super::interfaces::{self, CombatAttacker, CombatDamageable};
use super::life_bar::CombatLifeBar;
use super::model::{self, AttackState};
use super::player_controller::CombatPlayerController;

#[uclass(parent = Character)]
pub struct CombatCharacter {
    #[component(attach = "root_component")]
    camera_boom: SpringArmComponent,

    #[component(attach = "camera_boom", socket = "SpringEndpoint")]
    follow_camera: CameraComponent,

    #[component(attach = "root_component")]
    life_bar: WidgetComponent,

    #[uproperty(EditAnywhere)]
    jump_action: UObjectRef<InputAction>,

    #[uproperty(EditAnywhere)]
    move_action: UObjectRef<InputAction>,

    #[uproperty(EditAnywhere)]
    look_action: UObjectRef<InputAction>,

    #[uproperty(EditAnywhere)]
    mouse_look_action: UObjectRef<InputAction>,

    #[uproperty(EditAnywhere)]
    combo_attack_action: UObjectRef<InputAction>,

    #[uproperty(EditAnywhere)]
    charged_attack_action: UObjectRef<InputAction>,

    #[uproperty(EditAnywhere)]
    toggle_camera_action: UObjectRef<InputAction>,

    #[uproperty(EditAnywhere, default = model::MAX_HP, name = "MaxHP")]
    max_hp: f32,

    #[uproperty(VisibleAnywhere, default = 0.0, name = "CurrentHP")]
    current_hp: f32,

    #[uproperty(EditAnywhere)]
    life_bar_color: OwnedStruct<FLinearColor>,

    #[uproperty(EditAnywhere)]
    pelvis_bone_name: FName,

    #[uproperty(EditAnywhere)]
    life_bar_widget: UObjectRef<CombatLifeBar>,

    #[uproperty(EditAnywhere, default = model::ATTACK_INPUT_CACHE_TIME_TOLERANCE)]
    attack_input_cache_time_tolerance: f32,

    #[uproperty(EditAnywhere, default = model::MELEE_TRACE_DISTANCE)]
    melee_trace_distance: f32,

    #[uproperty(EditAnywhere, default = model::MELEE_TRACE_RADIUS)]
    melee_trace_radius: f32,

    #[uproperty(EditAnywhere, default = model::DANGER_TRACE_DISTANCE)]
    danger_trace_distance: f32,

    #[uproperty(EditAnywhere, default = model::DANGER_TRACE_RADIUS)]
    danger_trace_radius: f32,

    #[uproperty(EditAnywhere, default = model::MELEE_DAMAGE)]
    melee_damage: f32,

    #[uproperty(EditAnywhere, default = model::MELEE_KNOCKBACK_IMPULSE)]
    melee_knockback_impulse: f32,

    #[uproperty(EditAnywhere, default = model::MELEE_LAUNCH_IMPULSE)]
    melee_launch_impulse: f32,

    #[uproperty(EditAnywhere)]
    combo_attack_montage: UObjectRef<AnimMontage>,

    #[uproperty(EditAnywhere)]
    combo_section_names: UeArray<FName>,

    #[uproperty(EditAnywhere, default = model::COMBO_INPUT_CACHE_TIME_TOLERANCE)]
    combo_input_cache_time_tolerance: f32,

    #[uproperty(EditAnywhere)]
    charged_attack_montage: UObjectRef<AnimMontage>,

    #[uproperty(EditAnywhere)]
    charge_loop_section: FName,

    #[uproperty(EditAnywhere)]
    charge_attack_section: FName,

    #[uproperty(EditAnywhere, default = model::DEATH_CAMERA_DISTANCE)]
    death_camera_distance: f32,

    #[uproperty(EditAnywhere, default = model::DEFAULT_CAMERA_DISTANCE)]
    default_camera_distance: f32,

    #[uproperty(EditAnywhere, default = model::RESPAWN_TIME)]
    respawn_time: f32,

    attack: AttackState,

    attack_end_bound: bool,
}

#[uclass_impl]
impl CombatCharacter {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        let me = self.as_ref().checked()?;

        me.get_capsule_component().checked()?.set_capsule_size(
            model::CAPSULE_RADIUS,
            model::CAPSULE_HALF_HEIGHT,
            Some(false),
        );

        me.get_character_movement()
            .checked()?
            .set_max_walk_speed(model::MAX_WALK_SPEED);

        let boom = self.camera_boom()?.checked()?;
        boom.set_target_arm_length(self.default_camera_distance());
        boom.set_use_pawn_control_rotation(true);
        boom.set_enable_camera_lag(true);
        boom.set_enable_camera_rotation_lag(true);

        self.follow_camera()?
            .checked()?
            .set_use_pawn_control_rotation(false);

        me.tags().push(&FName::new(model::PLAYER_TAG).handle())?;

        Ok(())
    }

    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        if let Err(e) = self.begin_play() {
            ulog!(
                LOG_WARNING,
                "[Combat] {} BeginPlay failed: {e}",
                self.name()
            );
        }
    }

    #[ufunction(Override)]
    fn receive_end_play(&mut self, _end_play_reason: u8) {
        KismetSystemLibrary::k2_clear_timer(self.as_ref().upcast_to(), "RespawnCharacter");
    }

    #[ufunction(Override)]
    fn receive_restarted(&mut self) {
        match self.bind_input() {
            Ok(true) => ulog!(LOG_DISPLAY, "[Combat] {} input bound", self.name()),
            Ok(false) => {}
            Err(e) => ulog!(
                LOG_WARNING,
                "[Combat] {} failed to bind its input: {e}",
                self.name()
            ),
        }
    }

    #[ufunction(Override)]
    fn receive_controller_changed(
        &mut self,
        _old_controller: UObjectRef<Controller>,
        new_controller: UObjectRef<Controller>,
    ) {
        if let (Ok(controller), Ok(me)) = (
            CombatPlayerController::from_obj(new_controller),
            self.as_ref().checked(),
        ) {
            controller.set_respawn_transform(&me.get_transform());
        }
    }

    #[ufunction(Override)]
    fn on_landed(&mut self, _hit: UStructRef<FHitResult>) {
        if self.current_hp() >= 0.0
            && let Ok(mesh) = self
                .as_ref()
                .checked()
                .and_then(|me| me.get_mesh().checked())
        {
            mesh.set_physics_blend_weight(0.0);
        }
    }

    #[ufunction(BlueprintCallable)]
    fn r#move(&mut self, value: UStructRef<FInputActionValue>) {
        let movement_vector = value.axis2d();

        self.do_move(movement_vector.x as f32, movement_vector.y as f32);
    }

    #[ufunction(BlueprintCallable)]
    fn look(&mut self, value: UStructRef<FInputActionValue>) {
        let look_axis_vector = value.axis2d();

        self.do_look(look_axis_vector.x as f32, look_axis_vector.y as f32);
    }

    #[ufunction(BlueprintCallable)]
    fn combo_attack_pressed(&mut self) {
        self.do_combo_attack_start();
    }

    #[ufunction(BlueprintCallable)]
    fn charged_attack_pressed(&mut self) {
        self.do_charged_attack_start();
    }

    #[ufunction(BlueprintCallable)]
    fn charged_attack_released(&mut self) {
        self.do_charged_attack_end();
    }

    #[ufunction(BlueprintCallable)]
    fn toggle_camera(&mut self) {
        self.bp_toggle_camera();
    }

    #[ufunction(BlueprintImplementableEvent, name = "BP_ToggleCamera")]
    fn bp_toggle_camera(&self) {}

    #[ufunction(BlueprintCallable)]
    fn do_move(&mut self, right: f32, forward: f32) {
        if let Err(e) = self.apply_move(right, forward) {
            ulog!(LOG_WARNING, "[Combat] DoMove failed: {e}");
        }
    }

    #[ufunction(BlueprintCallable)]
    fn do_look(&mut self, yaw: f32, pitch: f32) {
        let Ok(me) = self.as_ref().checked() else {
            return;
        };

        if me.get_controller().is_valid() {
            me.add_controller_yaw_input(yaw);
            me.add_controller_pitch_input(pitch);
        }
    }

    #[ufunction(BlueprintCallable)]
    fn do_combo_attack_start(&mut self) {
        let mut attack = self.attack();

        if attack.is_attacking {
            attack.cached_attack_input_time = self.now();
            self.set_attack(attack);
            return;
        }

        self.combo_attack();
    }

    #[ufunction(BlueprintCallable)]
    fn do_combo_attack_end(&mut self) {}

    #[ufunction(BlueprintCallable)]
    fn do_charged_attack_start(&mut self) {
        let mut attack = self.attack();

        attack.is_charging_attack = true;

        if attack.is_attacking {
            if !attack.has_looped_charged_attack {
                attack.has_released_charged_attack = false;
            }

            attack.cached_attack_input_time = self.now();
            self.set_attack(attack);
            return;
        }

        self.set_attack(attack);
        self.charged_attack();
    }

    #[ufunction(BlueprintCallable)]
    fn do_charged_attack_end(&mut self) {
        let mut attack = self.attack();

        attack.is_charging_attack = false;

        let resolve = attack.has_looped_charged_attack && !attack.has_released_charged_attack;

        if resolve {
            attack.has_released_charged_attack = true;
        }

        self.set_attack(attack);

        if resolve {
            self.loop_or_resolve_charged_attack();
        }
    }

    #[ufunction(BlueprintCallable)]
    fn respawn_character(&mut self) {
        if let Ok(me) = self.as_ref().checked() {
            me.k2_destroy_actor();
        }
    }

    #[ufunction(BlueprintImplementableEvent)]
    fn dealt_damage(&self, damage: f32, impact_point: &OwnedStruct<FVector>) {}

    #[ufunction(BlueprintImplementableEvent)]
    fn received_damage(
        &self,
        damage: f32,
        impact_point: &OwnedStruct<FVector>,
        damage_direction: &OwnedStruct<FVector>,
    ) {
    }
}

impl CombatCharacter {
    fn begin_play(&mut self) -> RustealResult<()> {
        let life_bar_widget = self
            .life_bar()?
            .checked()?
            .get_user_widget_object()
            .cast::<CombatLifeBar>()?;

        self.set_life_bar_widget(life_bar_widget);

        self.camera_boom()?
            .checked()?
            .set_target_arm_length(self.default_camera_distance());

        CombatLifeBar::from_obj(life_bar_widget)?.set_bar_color(&self.life_bar_color());

        self.reset_hp()
    }

    fn bind_input(&self) -> RustealResult<bool> {
        let me = self.as_ref();
        let pawn = me.checked()?;

        if !pawn.is_player_controlled() || !pawn.is_locally_controlled() {
            return Ok(false);
        }

        bind_action(&me, self.move_action(), ETriggerEvent::Triggered, "Move")?;

        bind_action(&me, self.look_action(), ETriggerEvent::Triggered, "Look")?;

        bind_action(
            &me,
            self.mouse_look_action(),
            ETriggerEvent::Triggered,
            "Look",
        )?;

        bind_action(
            &me,
            self.combo_attack_action(),
            ETriggerEvent::Started,
            "ComboAttackPressed",
        )?;

        bind_action(
            &me,
            self.charged_attack_action(),
            ETriggerEvent::Started,
            "ChargedAttackPressed",
        )?;

        bind_action(
            &me,
            self.charged_attack_action(),
            ETriggerEvent::Completed,
            "ChargedAttackReleased",
        )?;

        bind_action(
            &me,
            self.toggle_camera_action(),
            ETriggerEvent::Triggered,
            "ToggleCamera",
        )?;

        Ok(true)
    }

    fn apply_move(&self, right: f32, forward: f32) -> RustealResult<()> {
        let me = self.as_ref().checked()?;
        let controller = me.get_controller();

        if !controller.is_valid() {
            return Ok(());
        }

        let yaw = controller
            .checked()?
            .get_control_rotation()
            .to_rotator()
            .yaw;

        let (sin, cos) = yaw.to_radians().sin_cos();
        let forward_direction = DVec3::new(cos, sin, 0.0);
        let right_direction = DVec3::new(-sin, cos, 0.0);

        me.add_movement_input(
            &FVector::from_dvec3(forward_direction),
            Some(forward),
            Some(false),
        );

        me.add_movement_input(
            &FVector::from_dvec3(right_direction),
            Some(right),
            Some(false),
        );

        Ok(())
    }

    fn reset_hp(&mut self) -> RustealResult<()> {
        self.set_current_hp(self.max_hp());

        CombatLifeBar::from_obj(self.life_bar_widget())?.set_life_percentage(1.0);

        Ok(())
    }

    fn combo_attack(&mut self) {
        let mut attack = self.attack();

        attack.is_attacking = true;

        attack.combo_count = 0;
        self.set_attack(attack);

        self.notify_enemies_of_incoming_attack();

        self.play_attack_montage(self.combo_attack_montage());
    }

    fn charged_attack(&mut self) {
        let mut attack = self.attack();

        attack.is_attacking = true;

        attack.has_looped_charged_attack = false;

        attack.has_released_charged_attack = false;
        self.set_attack(attack);

        self.notify_enemies_of_incoming_attack();

        self.play_attack_montage(self.charged_attack_montage());
    }

    fn play_attack_montage(&mut self, montage: UObjectRef<AnimMontage>) {
        let Ok(anim_instance) = self.anim_instance() else {
            return;
        };

        if let Err(e) = self.bind_attack_end(anim_instance) {
            ulog!(LOG_WARNING, "[Combat] cannot watch the attack montage: {e}");
        }

        if let Ok(anim_instance) = anim_instance.checked() {
            anim_instance.montage_play(montage, Some(1.0), None, Some(0.0), Some(true));
        }
    }

    fn bind_attack_end(
        &mut self,
        anim_instance: UObjectRef<bindings::engine::AnimInstance>,
    ) -> RustealResult<()> {
        if self.attack_end_bound() {
            return Ok(());
        }

        let character: UObjectRef<Character> = self.as_ref();

        anim_instance
            .checked()?
            .on_montage_ended()
            .add(move |montage, interrupted| {
                if let Ok(mut me) = CombatCharacter::from_obj(character)
                    && (montage == me.combo_attack_montage()
                        || montage == me.charged_attack_montage())
                {
                    me.attack_montage_ended(interrupted);
                }
            })?
            .detach();

        self.set_attack_end_bound(true);

        Ok(())
    }

    fn attack_montage_ended(&mut self, _interrupted: bool) {
        let mut attack = self.attack();

        attack.is_attacking = false;
        self.set_attack(attack);

        if model::input_is_fresh(
            self.now(),
            attack.cached_attack_input_time,
            self.attack_input_cache_time_tolerance(),
        ) {
            if attack.is_charging_attack {
                self.charged_attack();
            } else {
                self.combo_attack();
            }
        }
    }

    fn loop_or_resolve_charged_attack(&self) {
        let section = if self.attack().has_released_charged_attack {
            self.charge_attack_section()
        } else {
            self.charge_loop_section()
        };

        if let Ok(anim_instance) = self.anim_instance().and_then(|a| a.checked()) {
            anim_instance
                .montage_jump_to_section(section.handle(), Some(self.charged_attack_montage()));
        }
    }

    fn notify_enemies_of_incoming_attack(&self) {
        let Ok(me) = self.as_ref().checked() else {
            return;
        };

        let start = me.k2_get_actor_location().to_dvec3();

        let end = start
            + me.get_actor_forward_vector().to_dvec3() * f64::from(self.danger_trace_distance());

        for hit in self.sweep(
            start,
            end,
            self.danger_trace_radius(),
            &[ECollisionChannel::ECC_Pawn],
        ) {
            let Ok(actor) = hit
                .as_ref()
                .get_component()
                .checked()
                .map(|c| c.get_owner())
            else {
                continue;
            };

            if let Some(mut damageable) = interfaces::damageable(actor) {
                damageable.notify_danger(start, me.as_ref().upcast_to());
            }
        }
    }

    fn take_damage(&mut self, damage: f32) -> RustealResult<f32> {
        let Some((hp, dead)) = model::take_damage(self.current_hp(), damage) else {
            return Ok(0.0);
        };

        self.set_current_hp(hp);

        if dead {
            self.handle_death();
        } else {
            CombatLifeBar::from_obj(self.life_bar_widget())?
                .set_life_percentage(hp / self.max_hp());

            let mesh = self.as_ref().checked()?.get_mesh().checked()?;
            mesh.set_physics_blend_weight(0.5);
            mesh.set_body_simulate_physics(self.pelvis_bone_name().handle(), false);
        }

        Ok(damage)
    }

    pub(super) fn sweep(
        &self,
        start: DVec3,
        end: DVec3,
        radius: f32,
        channels: &[ECollisionChannel],
    ) -> Vec<OwnedStruct<FHitResult>> {
        let object_types: Vec<_> = channels
            .iter()
            .copied()
            .filter_map(object_type_query)
            .collect();

        KismetSystemLibrary::sphere_trace_multi_for_objects(
            self.as_ref().upcast_to(),
            &FVector::from_dvec3(start),
            &FVector::from_dvec3(end),
            radius,
            &object_types,
            false,
            &[],
            EDrawDebugTrace::None,
            true,
            &OwnedStruct::new(),
            &OwnedStruct::new(),
            None,
        )
        .1
    }

    fn anim_instance(&self) -> RustealResult<UObjectRef<bindings::engine::AnimInstance>> {
        Ok(self
            .as_ref()
            .checked()?
            .get_mesh()
            .checked()?
            .get_anim_instance())
    }

    fn now(&self) -> f64 {
        GameplayStatics::get_time_seconds(self.as_ref().upcast_to())
    }

    fn name(&self) -> String {
        self.as_ref()
            .get_name()
            .unwrap_or_else(|_| "CombatCharacter".into())
    }
}

impl CombatAttacker for CombatCharacter {
    fn do_attack_trace(&mut self, damage_source_bone: &str) {
        let Ok(me) = self.as_ref().checked() else {
            return;
        };

        let Ok(mesh) = me.get_mesh().checked() else {
            return;
        };

        let start = mesh
            .get_socket_location(FName::new(damage_source_bone).handle())
            .to_dvec3();

        let end = start
            + me.get_actor_forward_vector().to_dvec3() * f64::from(self.melee_trace_distance());

        let channels = [
            ECollisionChannel::ECC_Pawn,
            ECollisionChannel::ECC_WorldDynamic,
        ];

        for hit in self.sweep(start, end, self.melee_trace_radius(), &channels) {
            let hit = hit.as_ref();

            let Ok(actor) = hit.get_component().checked().map(|c| c.get_owner()) else {
                continue;
            };

            let Some(mut damageable) = interfaces::damageable(actor) else {
                continue;
            };

            let impulse = model::knockback(
                hit.get_impact_normal().to_dvec3(),
                self.melee_knockback_impulse(),
                self.melee_launch_impulse(),
            );

            let impact_point = hit.get_impact_point().to_dvec3();

            damageable.apply_damage(
                self.melee_damage(),
                me.as_ref().upcast_to(),
                impact_point,
                impulse,
            );

            self.dealt_damage(self.melee_damage(), &FVector::from_dvec3(impact_point));
        }
    }

    fn check_combo(&mut self) {
        let mut attack = self.attack();

        if !attack.is_attacking || attack.is_charging_attack {
            return;
        }

        if !model::input_is_fresh(
            self.now(),
            attack.cached_attack_input_time,
            self.combo_input_cache_time_tolerance(),
        ) {
            return;
        }

        attack.cached_attack_input_time = 0.0;

        attack.combo_count += 1;
        self.set_attack(attack);

        let Ok(sections) = self.combo_section_names().to_vec() else {
            return;
        };

        let Some(section) = sections.get(attack.combo_count as usize).copied() else {
            return;
        };

        self.notify_enemies_of_incoming_attack();

        if let Ok(anim_instance) = self.anim_instance().and_then(|a| a.checked()) {
            anim_instance
                .montage_jump_to_section(section.handle(), Some(self.combo_attack_montage()));
        }
    }

    fn check_charged_attack(&mut self) {
        let mut attack = self.attack();

        attack.has_looped_charged_attack = true;

        attack.has_released_charged_attack = !attack.is_charging_attack;
        self.set_attack(attack);

        self.loop_or_resolve_charged_attack();
    }
}

impl CombatDamageable for CombatCharacter {
    fn apply_damage(
        &mut self,
        damage: f32,
        _damage_causer: UObjectRef<Actor>,
        damage_location: DVec3,
        damage_impulse: DVec3,
    ) {
        let Ok(actual_damage) = self.take_damage(damage) else {
            return;
        };

        if actual_damage <= 0.0 {
            return;
        }

        let Ok(me) = self.as_ref().checked() else {
            return;
        };

        if let Ok(movement) = me.get_character_movement().checked() {
            movement.add_impulse(&FVector::from_dvec3(damage_impulse), Some(true));
        }

        if let Ok(mesh) = me.get_mesh().checked()
            && mesh.is_simulating_physics(None)
        {
            let impulse = damage_impulse * f64::from(mesh.get_mass());

            mesh.add_impulse_at_location(
                &FVector::from_dvec3(impulse),
                &FVector::from_dvec3(damage_location),
                None,
            );
        }

        self.received_damage(
            actual_damage,
            &FVector::from_dvec3(damage_location),
            &FVector::from_dvec3(model::safe_normal(damage_impulse)),
        );
    }

    fn handle_death(&mut self) {
        let Ok(me) = self.as_ref().checked() else {
            return;
        };

        if let Ok(movement) = me.get_character_movement().checked() {
            movement.disable_movement();
        }

        if let Ok(mesh) = me.get_mesh().checked() {
            mesh.set_simulate_physics(true);
        }

        if let Ok(life_bar) = self.life_bar().and_then(|l| l.checked()) {
            life_bar.set_hidden_in_game(true, None);
        }

        if let Ok(boom) = self.camera_boom().and_then(|b| b.checked()) {
            boom.set_target_arm_length(self.death_camera_distance());
        }

        KismetSystemLibrary::k2_set_timer(
            self.as_ref().upcast_to(),
            "RespawnCharacter",
            self.respawn_time(),
            false,
            None,
            None,
            None,
        );
    }

    fn apply_healing(&mut self, _healing: f32, _healer: UObjectRef<Actor>) {}

    fn notify_danger(&mut self, _danger_location: DVec3, _danger_source: UObjectRef<Actor>) {}
}
