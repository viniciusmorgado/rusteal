use std::sync::atomic::{AtomicI32, Ordering};

use bindings::engine::{
    ActorExt, CharacterExt, CharacterMovementComponent, CharacterMovementComponentExt,
    EMovementMode, MovementComponentExt,
};
use bindings::prelude::*;
use bindings::rusteal::{RustealCharacter, RustealCharacterMovementComponent, RustealCommandlet};
use rusteal_runtime::runtime::{Rotator, Transform, UObjectRef};
use rusteal_runtime::{uclass, uclass_impl};

use crate::runner::{Context, Outcome};

static MODE_CHANGES: AtomicI32 = AtomicI32::new(0);

#[uclass(parent = RustealCharacterMovementComponent)]
pub struct VirtualsMovement {
    #[uproperty]
    custom_steps: i32,
}

#[uclass_impl]
impl VirtualsMovement {
    #[ufunction(Override)]
    fn receive_phys_custom(&mut self, _delta_time: f32, _iterations: i32) {
        self.set_custom_steps(self.custom_steps() + 1);

        let Ok(movement) = self.as_ref().checked() else {
            return;
        };

        movement.k2_move_updated_component(
            &FVector::from_dvec3(glam::DVec3::new(0.0, 0.0, 10.0)),
            &FRotator::from_rotator(Rotator::ZERO),
            Some(false),
            Some(false),
        );
    }

    #[ufunction(Override)]
    fn receive_get_max_speed(&self) -> f32 {
        1234.0
    }

    #[ufunction(Override)]
    fn receive_on_movement_mode_changed(
        &mut self,
        _previous_movement_mode: EMovementMode,
        _previous_custom_mode: u8,
    ) {
        MODE_CHANGES.fetch_add(1, Ordering::SeqCst);
    }
}

#[uclass(parent = RustealCharacter, subobject_class(CharMoveComp = VirtualsMovement))]
pub struct VirtualsMover {}

#[uclass(parent = RustealCommandlet)]
pub struct VirtualsCommandlet {}

#[uclass_impl]
impl VirtualsCommandlet {
    #[ufunction(Override)]
    fn receive_main(&mut self, params: String) -> i32 {
        log::info!("[RustealTests] VirtualsCommandlet ran with `{params}`");

        if params.contains("rusteal-probe") { 3 } else { 1 }
    }
}

fn ensure(condition: bool, what: impl Into<String>) -> Result<(), String> {
    if condition { Ok(()) } else { Err(what.into()) }
}

pub fn custom_movement_reaches_rust(context: &Context) -> Outcome {
    let started = (|| {
        let e = |e| format!("{e:?}");
        let transform = FTransform::from_transform(Transform::from_translation(
            glam::DVec3::new(40_000.0, -40_000.0, 40_000.0),
        ));
        let mover = context
            .world
            .spawn_actor::<VirtualsMover>(&transform)
            .map_err(e)?;
        let checked = mover.checked().map_err(e)?;
        let movement: UObjectRef<CharacterMovementComponent> = checked.get_character_movement();

        VirtualsMovement::from_obj(movement)
            .map_err(|e| format!("the character's movement is not the Rust class: {e:?}"))?;

        let checked_movement = movement.checked().map_err(e)?;
        checked_movement.set_run_physics_with_no_controller(true);
        let changes_before = MODE_CHANGES.load(Ordering::SeqCst);
        checked_movement.set_movement_mode(EMovementMode::MOVE_Custom, Some(1));

        let start_z = checked.k2_get_actor_location().to_dvec3().z;
        Ok::<_, String>((mover, movement, changes_before, start_z))
    })();

    let (mover, movement, changes_before, start_z) = match started {
        Ok(started) => started,
        Err(e) => return Outcome::Done(Err(e)),
    };

    let mut frames = 0;

    Outcome::Pending(Box::new(move || {
        frames += 1;

        if frames < 10 {
            return None;
        }

        let result = (|| {
            let e = |e| format!("{e:?}");
            let custom = VirtualsMovement::from_obj(movement).map_err(e)?;
            let steps = custom.custom_steps();
            let max_speed = movement.checked().map_err(e)?.get_max_speed();
            let changes = MODE_CHANGES.load(Ordering::SeqCst) - changes_before;
            let z = mover.checked().map_err(e)?.k2_get_actor_location().to_dvec3().z;

            ensure(
                steps > 0 && z > start_z,
                format!("{steps} custom steps moved the character from {start_z} to {z}"),
            )?;
            ensure(max_speed == 1234.0, format!("GetMaxSpeed gave {max_speed}"))?;
            ensure(changes > 0, "OnMovementModeChanged never reached Rust")
        })();

        if let Ok(mover) = mover.checked() {
            mover.k2_destroy_actor();
        }

        Some(result)
    }))
}
