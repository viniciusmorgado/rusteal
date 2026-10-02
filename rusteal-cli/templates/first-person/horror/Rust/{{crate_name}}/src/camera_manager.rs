// FirstPersonCameraManager: the First Person template's
// `ATP_FirstPersonCameraManager` in Rust, a basic first person camera manager
// that limits the min/max look pitch. `FirstPersonPlayerController` makes it
// its controllers' camera manager.

use bindings::engine::{PlayerCameraManager, PlayerCameraManagerExt};
use rusteal_runtime::runtime::RustealResult;
use rusteal_runtime::{uclass, uclass_impl};

/// `ViewPitchMin = -70.0f`, `ViewPitchMax = 80.0f`.
const VIEW_PITCH_MIN: f32 = -70.0;
const VIEW_PITCH_MAX: f32 = 80.0;

#[uclass(parent = PlayerCameraManager)]
pub struct FirstPersonCameraManager {}

#[uclass_impl]
impl FirstPersonCameraManager {
    /// Everything `ATP_FirstPersonCameraManager::ATP_FirstPersonCameraManager()` sets.
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        // set the min/max pitch
        let me = self.as_ref().checked()?;
        me.set_view_pitch_min(VIEW_PITCH_MIN);
        me.set_view_pitch_max(VIEW_PITCH_MAX);
        Ok(())
    }
}
