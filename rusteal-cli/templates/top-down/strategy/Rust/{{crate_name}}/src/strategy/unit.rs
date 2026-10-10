use bindings::ai::{
    AIController, AIControllerExt, EEnvQueryRunMode, EPathFollowingRequestResult, EnvQuery,
    EnvQueryInstanceBlueprintWrapper, EnvQueryInstanceBlueprintWrapperExt, EnvQueryManager,
};
use bindings::engine::{
    Actor, ActorExt, Character, CharacterExt, CharacterMovementComponentExt, Controller,
    EAutoPossessAI, EObjectTypeQuery, FNavMovementPropertiesExt, KismetMathLibrary,
    KismetSystemLibrary, MovementComponentExt, NavMovementComponentExt, PawnExt,
    PrimitiveComponentExt, SphereComponent, SphereComponentExt,
};
use bindings::prelude::*;
use glam::DVec3;
use rusteal_runtime::runtime::{
    FName, LOG_WARNING, Rotator, RustealResult, SubclassOf, UObjectRef, ulog,
};
use rusteal_runtime::{uclass, uclass_impl};

const INTERACTION_RANGE_RADIUS: f32 = 100.0;
const GRAVITY_SCALE: f32 = 1.5;
const MAX_ACCELERATION: f32 = 1000.0;
const BRAKING_FRICTION_FACTOR: f32 = 1.0;
const BRAKING_DECELERATION_WALKING: f32 = 1000.0;
const PERCH_RADIUS_THRESHOLD: f32 = 20.0;
const ROTATION_RATE_YAW: f64 = 640.0;
const AVOIDANCE_CONSIDERATION_RADIUS: f32 = 150.0;
const AVOIDANCE_WEIGHT: f32 = 1.0;
const FIXED_BRAKING_DISTANCE: f32 = 1.0;

#[uclass(parent = Character)]
pub struct StrategyUnit {
    #[component(attach = "root_component", name = "Interaction Range")]
    interaction_range: SphereComponent,

    #[uproperty(EditAnywhere, category = "NPC")]
    interaction_query: UObjectRef<EnvQuery>,

    #[uproperty(EditAnywhere, category = "NPC")]
    no_interaction_query: UObjectRef<EnvQuery>,

    #[uproperty(EditAnywhere, category = "NPC", default = 100.0)]
    movement_acceptance_radius: f32,

    #[uproperty(EditAnywhere, category = "Input", default = 250.0)]
    interaction_radius: f32,

    #[uproperty]
    env_query_instance: UObjectRef<EnvQueryInstanceBlueprintWrapper>,

    ai_controller: UObjectRef<AIController>,

    current_movement_goal: DVec3,

    b_interact_on_arrival: bool,

    interact_ignore_list: Vec<UObjectRef<StrategyUnit>>,
}

#[uclass_impl]
impl StrategyUnit {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        let me = self.as_ref().checked()?;

        me.set_auto_possess_ai(EAutoPossessAI::PlacedInWorldOrSpawned);

        let interaction_range = self.interaction_range()?.checked()?;
        interaction_range.set_sphere_radius(INTERACTION_RANGE_RADIUS, Some(true));

        interaction_range
            .set_collision_profile_name(FName::new("OverlapAllDynamic").handle(), Some(true));

        let movement = me.get_character_movement().checked()?;
        movement.set_gravity_scale(GRAVITY_SCALE);
        movement.set_max_acceleration(MAX_ACCELERATION);
        movement.set_braking_friction_factor(BRAKING_FRICTION_FACTOR);
        movement.set_braking_deceleration_walking(BRAKING_DECELERATION_WALKING);
        movement.set_perch_radius_threshold(PERCH_RADIUS_THRESHOLD);
        movement.set_use_flat_base_for_floor_checks(true);

        movement.set_rotation_rate(&FRotator::from_rotator(Rotator::new(
            0.0,
            ROTATION_RATE_YAW,
            0.0,
        )));

        movement.set_orient_rotation_to_movement(true);
        movement.set_avoidance_consideration_radius(AVOIDANCE_CONSIDERATION_RADIUS);
        movement.set_avoidance_weight(AVOIDANCE_WEIGHT);
        movement.set_plane_constraint_enabled(true);
        movement.set_snap_to_plane_at_start(true);
        let nav_movement = movement.get_nav_movement_properties();

        nav_movement
            .as_ref()
            .set_use_fixed_braking_distance_for_paths(true);

        nav_movement
            .as_ref()
            .set_fixed_path_braking_distance(FIXED_BRAKING_DISTANCE);

        movement.set_nav_movement_properties(&nav_movement);

        Ok(())
    }

    #[ufunction(Override)]
    fn receive_controller_changed(
        &mut self,
        _old_controller: UObjectRef<Controller>,
        new_controller: UObjectRef<Controller>,
    ) {
        let ai_controller: UObjectRef<AIController> = new_controller.cast().unwrap_or_default();
        self.set_ai_controller(ai_controller);

        let Ok(controller) = ai_controller.checked() else {
            return;
        };

        let me: UObjectRef<StrategyUnit> = self.as_ref().cast().unwrap_or_default();

        let bound = controller
            .receive_move_completed()
            .add(move |_request_id, _result| {
                if let Ok(mut unit) = StrategyUnit::from_obj(me) {
                    unit.handle_move_finished();
                }
            });

        match bound {
            Ok(binding) => binding.detach(),
            Err(e) => ulog!(LOG_WARNING, "[Strategy] cannot watch the unit's moves: {e}"),
        }
    }

    #[ufunction(BlueprintImplementableEvent, name = "BP_UnitSelected")]
    fn bp_unit_selected(&self) {}

    #[ufunction(BlueprintImplementableEvent, name = "BP_UnitDeselected")]
    fn bp_unit_deselected(&self) {}

    #[ufunction(BlueprintImplementableEvent, name = "BP_StopAnimation")]
    fn bp_stop_animation(&self) {}

    #[ufunction(BlueprintImplementableEvent, name = "BP_InteractionBehavior")]
    fn bp_interaction_behavior(&self, interactor: UObjectRef<StrategyUnit>) {}
}

impl StrategyUnit {
    pub fn stop_moving(&self) {
        if let Ok(movement) = self
            .as_ref()
            .checked()
            .and_then(|me| me.get_character_movement().checked())
        {
            movement.stop_movement_immediately();
        }

        self.bp_stop_animation();
    }

    pub fn unit_selected(&self) {
        self.bp_unit_selected();
    }

    pub fn unit_deselected(&self) {
        self.bp_unit_deselected();
    }

    pub fn interact(&self, interactor: UObjectRef<StrategyUnit>) {
        let (Ok(me), Ok(other)) = (self.as_ref().checked(), StrategyUnit::from_obj(interactor))
        else {
            return;
        };

        let Ok(other_actor) = interactor.checked() else {
            return;
        };

        let look_at = KismetMathLibrary::find_look_at_rotation(
            &me.k2_get_actor_location(),
            &other_actor.k2_get_actor_location(),
        );

        me.k2_set_actor_rotation(&look_at, false);

        other.bp_interaction_behavior(self.as_ref().cast().unwrap_or_default());

        self.bp_interaction_behavior(interactor);
    }

    pub fn move_to_location(
        &mut self,
        location: DVec3,
        b_interact: bool,
        ignore_list: Vec<UObjectRef<StrategyUnit>>,
    ) {
        self.set_current_movement_goal(location);
        self.set_b_interact_on_arrival(b_interact);
        self.set_interact_ignore_list(ignore_list);

        self.stop_moving();

        let move_query = if b_interact {
            self.interaction_query()
        } else {
            self.no_interaction_query()
        };

        let run_mode = if b_interact {
            EEnvQueryRunMode::SingleResult
        } else {
            EEnvQueryRunMode::RandomBest25Pct
        };

        let me: UObjectRef<StrategyUnit> = self.as_ref().cast().unwrap_or_default();

        let query_instance = EnvQueryManager::run_eqs_query(
            me.upcast_to(),
            move_query,
            me.upcast_to(),
            run_mode,
            SubclassOf::<EnvQueryInstanceBlueprintWrapper>::base(),
        );

        self.set_env_query_instance(query_instance);

        if let Ok(query) = query_instance.checked() {
            let bound =
                query
                    .on_query_finished_event()
                    .add(move |finished_query, _query_status| {
                        if let Ok(mut unit) = StrategyUnit::from_obj(me) {
                            unit.on_eqs_finished(finished_query);
                        }
                    });

            match bound {
                Ok(binding) => binding.detach(),
                Err(e) => ulog!(LOG_WARNING, "[Strategy] cannot watch the move query: {e}"),
            }
        }
    }

    pub fn get_movement_goal(&self) -> DVec3 {
        self.current_movement_goal()
    }

    fn on_eqs_finished(&mut self, query_instance: UObjectRef<EnvQueryInstanceBlueprintWrapper>) {
        let Ok(query) = query_instance.checked() else {
            return;
        };

        let (found, result_locations) = query.get_query_results_as_locations();

        let Some(top_result) = result_locations.first().filter(|_| found) else {
            return;
        };

        self.set_current_movement_goal(top_result.to_dvec3());

        let Ok(ai_controller) = self.ai_controller().checked() else {
            return;
        };

        let result = ai_controller.move_to_location(
            &FVector::from_dvec3(self.current_movement_goal()),
            Some(self.movement_acceptance_radius()),
            Some(true),
            Some(true),
            Some(true),
            Some(false),
            None,
            Some(true),
        );

        if result == EPathFollowingRequestResult::AlreadyAtGoal {
            self.handle_move_finished();
        }
    }

    fn handle_move_finished(&mut self) {
        if !self.b_interact_on_arrival() {
            return;
        }

        let Ok(me) = self.as_ref().checked() else {
            return;
        };

        let mut ignored: Vec<UObjectRef<Actor>> = vec![me.as_ref().upcast_to()];

        ignored.extend(
            self.interact_ignore_list()
                .iter()
                .map(|unit| unit.upcast_to()),
        );

        let (_, overlaps) = KismetSystemLibrary::sphere_overlap_actors(
            me.as_ref().upcast_to(),
            &me.k2_get_actor_location(),
            self.interaction_radius(),
            &[EObjectTypeQuery::ObjectTypeQuery2],
            SubclassOf::<StrategyUnit>::base().upcast_to(),
            &ignored,
        );

        if let Some(unit) = overlaps
            .into_iter()
            .find_map(|actor| StrategyUnit::from_obj(actor).ok())
        {
            unit.interact(self.as_ref().cast().unwrap_or_default());
        }
    }
}
