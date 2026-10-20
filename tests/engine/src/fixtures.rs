use std::collections::{HashMap, HashSet};

use bindings::engine::{Material, SceneComponent};
use bindings::prelude::*;
use rusteal_runtime::runtime::{
    FName, OwnedStruct, ScriptInterface, SoftClassRef, SoftObjectRef, TWeakObjectPtr, Text,
    UeArray, UeMap, UeSet,
};
use rusteal_runtime::{uclass, uclass_impl, uinterface, ustruct};

#[ustruct]
pub struct Loadout {
    #[uproperty(EditAnywhere)]
    title: Text,

    #[uproperty(EditAnywhere)]
    counts: UeMap<FName, i32>,

    #[uproperty(EditAnywhere)]
    level: u16,
}

#[uinterface]
pub trait Pingable {
    fn ping_value(&self) -> i32;

    fn describe(&self, name: String, tags: Vec<FName>) -> Text;
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

    fn describe(&self, name: String, tags: Vec<FName>) -> Text {
        Text::from(format!("{name}: {}", tags.len()))
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

    #[ufunction]
    pub fn greet(&self, name: String, title: Text) -> String {
        format!("{title} {name}")
    }

    #[ufunction]
    pub fn tally(&self, words: Vec<String>) -> HashMap<String, i32> {
        let mut counts = HashMap::new();

        for word in words {
            *counts.entry(word).or_insert(0) += 1;
        }

        counts
    }

    #[ufunction]
    pub fn unique(&self, names: HashSet<FName>) -> u32 {
        names.len() as u32
    }

    #[ufunction]
    pub fn widen(&self, value: u16, delta: i8) -> u64 {
        (value as i64 + delta as i64) as u64
    }

    #[ufunction]
    pub fn caption_for(&self, key: FName) -> Text {
        Text::localized("RustealTests", &key.to_string(), "A caption")
    }

    #[ufunction]
    pub fn scale(&self, vector: OwnedStruct<FVector>, by: f64) -> OwnedStruct<FVector> {
        FVector::from_dvec3(vector.to_dvec3() * by)
    }

    #[ufunction]
    pub fn has_partner(&self, partner: Option<ScriptInterface<dyn Pingable>>) -> bool {
        partner.is_some()
    }

    #[ufunction]
    pub fn class_path(&self, class: SoftClassRef<Actor>) -> String {
        class.path().to_string()
    }

    #[ufunction(BlueprintNativeEvent)]
    pub fn reaction(&self, amount: i32) -> i32 {
        amount * 2
    }

    #[ufunction]
    pub fn split_name(
        &self,
        full: String,
        first: &mut String,
        rest: &mut Vec<String>,
        count: &mut i32,
    ) -> bool {
        let mut words = full.split(' ').map(String::from);
        *first = words.next().unwrap_or_default();
        *rest = words.collect();
        *count = 1 + rest.len() as i32;
        !first.is_empty()
    }
}

#[uclass(parent = TestActor)]
pub struct ChildTestActor {}

#[uclass_impl]
impl ChildTestActor {
    #[ufunction(Override)]
    fn reaction(&self, amount: i32) -> i32 {
        amount * 3
    }
}
