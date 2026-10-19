use bindings::engine::SceneComponent;
use bindings::prelude::*;
use rusteal_runtime::{uclass, uclass_impl};

#[uclass(parent = Actor)]
pub struct TestActor {
    #[component(root)]
    root: SceneComponent,

    #[uproperty(BlueprintReadWrite)]
    count: i32,

    #[uproperty(BlueprintReadWrite)]
    speed: f32,

    #[uproperty(BlueprintReadWrite)]
    b_enabled: bool,

    #[uproperty(BlueprintReadWrite)]
    label: String,
}

#[uclass_impl]
impl TestActor {
    #[ufunction]
    pub fn add(&self, a: i32, b: i32) -> i32 {
        a + b
    }

    #[ufunction]
    pub fn read_count(&self) -> i32 {
        self.count()
    }

    #[ufunction]
    pub fn echo(&self, value: i32) -> i32 {
        value
    }

    #[ufunction]
    pub fn store(&self, value: i32) {
        self.set_count(value);
    }

    #[udelegate]
    pub fn on_ping(&self, value: i32) {}

    #[ufunction]
    pub fn ping(&self, value: i32) {
        self.on_ping(value);
    }
}
