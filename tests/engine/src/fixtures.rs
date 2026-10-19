use bindings::engine::{Material, SceneComponent};
use bindings::prelude::*;
use rusteal_runtime::runtime::{
    FName, ScriptInterface, SoftClassRef, SoftObjectRef, TWeakObjectPtr, Text, UeArray, UeMap,
    UeSet,
};
use rusteal_runtime::{uclass, uclass_impl, uinterface};

#[uinterface]
pub trait Pingable {
    fn ping_value(&self) -> i32;
}

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

    #[uproperty(BlueprintReadWrite)]
    small: i8,

    #[uproperty(BlueprintReadWrite)]
    short: i16,

    #[uproperty(BlueprintReadWrite, default = 40000)]
    wide: u16,

    #[uproperty(BlueprintReadWrite, default = 3_000_000_000)]
    big: u32,

    #[uproperty(BlueprintReadWrite)]
    huge: u64,

    /// The text shown over the actor.
    #[uproperty(
        BlueprintReadWrite,
        EditAnywhere,
        category = "Display",
        meta(MultiLine),
        default = "Hello"
    )]
    caption: Text,

    #[uproperty(BlueprintReadWrite, default = "Hero")]
    title: String,

    #[uproperty(BlueprintReadWrite)]
    target: TWeakObjectPtr<Actor>,

    #[uproperty(BlueprintReadWrite)]
    spawn_class: SoftClassRef<Actor>,

    #[uproperty(BlueprintReadWrite)]
    partner: ScriptInterface<dyn Pingable>,

    #[uproperty(BlueprintReadWrite)]
    scores: UeMap<String, i32>,

    #[uproperty(BlueprintReadWrite)]
    seen: UeSet<FName>,

    #[uproperty(BlueprintReadWrite)]
    names: UeArray<String>,

    #[uproperty(BlueprintReadWrite)]
    assets: UeArray<SoftObjectRef<Material>>,

    #[uproperty(BlueprintReadWrite)]
    captions: UeArray<Text>,

    #[uproperty(EditAnywhere, meta(ClampMin = 0, ClampMax = 100, Units = "cm"))]
    range: f32,
}

#[uclass_impl]
impl Pingable for TestActor {
    fn ping_value(&self) -> i32 {
        99
    }
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
