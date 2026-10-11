use bindings::enhanced_input::{
    EnhancedInputLibrary, EnhancedPlayerInput, FInputActionValue, InputModifier,
};
use rusteal_runtime::runtime::{OwnedStruct, UObjectRef, UStructRef};
use rusteal_runtime::{uclass, uclass_impl};

use crate::model;

#[uclass(parent = InputModifier)]
pub struct InputModifierXAxisPositiveOnly {}

#[uclass_impl]
impl InputModifierXAxisPositiveOnly {
    #[ufunction(Override)]
    fn modify_raw(
        &self,
        _player_input: UObjectRef<EnhancedPlayerInput>,
        current_value: UStructRef<FInputActionValue>,
        _delta_time: f32,
    ) -> OwnedStruct<FInputActionValue> {
        let (x, y, z, value_type) =
            EnhancedInputLibrary::break_input_action_value(&current_value.to_owned());
        EnhancedInputLibrary::make_input_action_value_of_type(
            model::x_axis_positive_only(x),
            y,
            z,
            value_type,
        )
    }
}
