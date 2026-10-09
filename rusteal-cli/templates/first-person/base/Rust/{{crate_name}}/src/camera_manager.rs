use bindings::engine::{PlayerCameraManager, PlayerCameraManagerExt};
use rusteal_runtime::runtime::RustealResult;
use rusteal_runtime::{uclass, uclass_impl};

const VIEW_PITCH_MIN: f32 = -70.0;
const VIEW_PITCH_MAX: f32 = 80.0;

#[uclass(parent = PlayerCameraManager)]
pub struct FirstPersonCameraManager {}

#[uclass_impl]
impl FirstPersonCameraManager {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        let me = self.as_ref().checked()?;
        me.set_view_pitch_min(VIEW_PITCH_MIN);
        me.set_view_pitch_max(VIEW_PITCH_MAX);

        Ok(())
    }
}
