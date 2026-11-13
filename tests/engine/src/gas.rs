use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};

use bindings::engine::{ActorExt, SceneComponent};
use bindings::gameplay_abilities::{
    AbilitySystemComponent, AbilitySystemComponentExt, AttributeSet,
    EGameplayEffectDurationType, EGameplayEffectMagnitudeCalculation, EGameplayModOp,
    FGameplayAttribute, FGameplayAttributeData, FGameplayAttributeDataExt, FGameplayAttributeExt,
    FGameplayEffectModifierMagnitudeExt, FGameplayEffectSpec, FGameplayModifierEvaluatedData,
    FGameplayModifierEvaluatedDataExt, FGameplayModifierInfo, FGameplayModifierInfoExt,
    FScalableFloatExt, GameplayAbility, GameplayAbilityExt, GameplayEffect, GameplayEffectExt,
};
use bindings::prelude::*;
use bindings::rusteal::RustealAttributeSet;
use rusteal_runtime::runtime::{
    FName, OutRef, OwnedStruct, RustealResult, SubclassOf, Transform, UObjectRef, UStructRef,
};
use rusteal_runtime::{uclass, uclass_impl};

use crate::runner::{Context, Outcome};

const FULL_HEALTH: f32 = 100.0;
const DAMAGE: f32 = -150.0;

static EXECUTED_MAGNITUDE: Mutex<Option<f32>> = Mutex::new(None);
static ACTIVATIONS: AtomicU32 = AtomicU32::new(0);
static ENDS: AtomicU32 = AtomicU32::new(0);

fn attribute_data(value: f32) -> OwnedStruct<FGameplayAttributeData> {
    let data = OwnedStruct::<FGameplayAttributeData>::new();
    data.as_ref().set_base_value(value);
    data.as_ref().set_current_value(value);
    data
}

fn health_attribute() -> OwnedStruct<FGameplayAttribute> {
    RustealAttributeSet::attribute_of(
        SubclassOf::<HealthSet>::base().upcast_to::<AttributeSet>(),
        FName::new("Health").handle(),
    )
}

#[uclass(parent = RustealAttributeSet)]
pub struct HealthSet {
    #[uproperty(BlueprintReadOnly)]
    health: OwnedStruct<FGameplayAttributeData>,

    #[uproperty(BlueprintReadOnly)]
    max_health: OwnedStruct<FGameplayAttributeData>,
}

impl HealthSet {
    fn clamped(&self, attribute: &UStructRef<FGameplayAttribute>, value: f32) -> f32 {
        if attribute.get_attribute_name() == "Health" {
            value.clamp(0.0, self.max_health().as_ref().get_current_value())
        } else {
            value
        }
    }
}

#[uclass_impl]
impl HealthSet {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        self.set_health(&attribute_data(FULL_HEALTH));
        self.set_max_health(&attribute_data(FULL_HEALTH));
        Ok(())
    }

    #[ufunction(Override)]
    fn receive_pre_attribute_change(
        &mut self,
        attribute: UStructRef<FGameplayAttribute>,
        new_value: OutRef<f32>,
    ) {
        new_value.set(self.clamped(&attribute, new_value.get()));
    }

    #[ufunction(Override)]
    fn receive_pre_attribute_base_change(
        &self,
        attribute: UStructRef<FGameplayAttribute>,
        new_value: OutRef<f32>,
    ) {
        new_value.set(self.clamped(&attribute, new_value.get()));
    }

    #[ufunction(Override)]
    fn receive_post_gameplay_effect_execute(
        &mut self,
        _effect_spec: UStructRef<FGameplayEffectSpec>,
        evaluated_data: UStructRef<FGameplayModifierEvaluatedData>,
        _target: UObjectRef<AbilitySystemComponent>,
    ) {
        if let Ok(mut executed) = EXECUTED_MAGNITUDE.lock() {
            *executed = Some(evaluated_data.get_magnitude());
        }
    }
}

#[uclass(parent = GameplayEffect)]
pub struct DamageEffect {}

#[uclass_impl]
impl DamageEffect {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        let effect = self.as_ref().checked()?;
        effect.set_duration_policy(EGameplayEffectDurationType::Instant);

        let modifier = OwnedStruct::<FGameplayModifierInfo>::new();
        modifier.as_ref().set_attribute(&health_attribute());
        modifier.as_ref().set_modifier_op(EGameplayModOp::AddBase);

        let magnitude = modifier.as_ref().get_modifier_magnitude();
        magnitude
            .as_ref()
            .set_magnitude_calculation_type(EGameplayEffectMagnitudeCalculation::ScalableFloat);
        let amount = magnitude.as_ref().get_scalable_float_magnitude();
        amount.as_ref().set_value(DAMAGE);
        magnitude.as_ref().set_scalable_float_magnitude(&amount);
        modifier.as_ref().set_modifier_magnitude(&magnitude);

        effect.modifiers().push(&modifier)?;
        Ok(())
    }
}

#[uclass(parent = GameplayAbility)]
pub struct ProbeAbility {}

#[uclass_impl]
impl ProbeAbility {
    #[ufunction(Override, name = "K2_ActivateAbility")]
    fn k2_activate_ability(&mut self) {
        ACTIVATIONS.fetch_add(1, Ordering::SeqCst);

        if let Ok(ability) = self.as_ref().checked() {
            ability.k2_end_ability();
        }
    }

    #[ufunction(Override, name = "K2_OnEndAbility")]
    fn k2_on_end_ability(&mut self, _was_cancelled: bool) {
        ENDS.fetch_add(1, Ordering::SeqCst);
    }
}

#[uclass(parent = Actor)]
pub struct AbilityTarget {
    #[component(root)]
    root: SceneComponent,

    #[component]
    ability_system: AbilitySystemComponent,
}

fn spawn_target(context: &Context) -> Result<UObjectRef<AbilityTarget>, String> {
    let transform = FTransform::from_transform(Transform::from_translation(glam::DVec3::new(
        0.0, 0.0, -50_000.0,
    )));

    context
        .world
        .spawn_actor::<AbilityTarget>(&transform)
        .map_err(|e| format!("{e:?}"))
}

pub fn gameplay_effect_changes_a_rust_attribute(context: &Context) -> Outcome {
    Outcome::Done((|| {
        let e = |e| format!("{e:?}");
        let actor = spawn_target(context)?;
        let asc = AbilityTarget::from_obj(actor)
            .and_then(|target| target.ability_system())
            .map_err(e)?
            .checked()
            .map_err(e)?;

        asc.k2_init_stats(
            SubclassOf::<HealthSet>::base().upcast_to::<AttributeSet>(),
            UObjectRef::null(),
        );

        let health = health_attribute();
        let (before, found) = asc.get_gameplay_attribute_value(&health);

        if let Ok(mut executed) = EXECUTED_MAGNITUDE.lock() {
            *executed = None;
        }

        let effect_context = asc.make_effect_context();
        asc.bp_apply_gameplay_effect_to_self(
            SubclassOf::<DamageEffect>::base().upcast_to::<GameplayEffect>(),
            1.0,
            &effect_context,
        );

        let (after, _) = asc.get_gameplay_attribute_value(&health);
        let executed = EXECUTED_MAGNITUDE.lock().ok().and_then(|executed| *executed);

        actor.checked().map_err(e)?.k2_destroy_actor();

        if found && before == FULL_HEALTH && after == 0.0 && executed == Some(DAMAGE) {
            Ok(())
        } else {
            Err(format!(
                "health found: {found}, before: {before}, after: {after}; the executed magnitude: {executed:?}"
            ))
        }
    })())
}

pub fn rust_gameplay_ability_activates_and_ends(context: &Context) -> Outcome {
    Outcome::Done((|| {
        let e = |e| format!("{e:?}");
        let actor = spawn_target(context)?;
        let asc = AbilityTarget::from_obj(actor)
            .and_then(|target| target.ability_system())
            .map_err(e)?
            .checked()
            .map_err(e)?;

        let activations = ACTIVATIONS.load(Ordering::SeqCst);
        let ends = ENDS.load(Ordering::SeqCst);

        asc.k2_give_ability_and_activate_once(
            SubclassOf::<ProbeAbility>::base().upcast_to::<GameplayAbility>(),
            None,
            None,
        );

        let activated = ACTIVATIONS.load(Ordering::SeqCst) - activations;
        let ended = ENDS.load(Ordering::SeqCst) - ends;

        actor.checked().map_err(e)?.k2_destroy_actor();

        if activated == 1 && ended == 1 {
            Ok(())
        } else {
            Err(format!("activated {activated} times, ended {ended} times"))
        }
    })())
}
