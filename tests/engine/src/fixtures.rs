use std::collections::{HashMap, HashSet};

use bindings::engine::{
    BlueprintFunctionLibrary, ECollisionChannel, Material, SaveGame, SceneComponent,
};
use bindings::enhanced_input::FInputActionValue;
use bindings::prelude::*;
use rusteal_runtime::runtime::{
    FName, OwnedStruct, ScriptInterface, SoftClassRef, SoftObjectRef, SubclassOf, TWeakObjectPtr,
    Text, UObjectRef, UStructRef, UeArray, UeMap, UeSet,
};
use rusteal_runtime::{uclass, uclass_impl, uenum, uinterface, ustruct};

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

#[uenum]
pub enum Mood {
    Calm = 0,
    Wary = 1,
    Angry = 3,
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

    #[uproperty(BlueprintReadWrite)]
    buddy: UObjectRef<Actor>,

    #[uproperty(BlueprintReadWrite)]
    buddy_class: SubclassOf<Actor>,

    #[uproperty(BlueprintReadWrite)]
    tag: FName,

    #[uproperty(BlueprintReadWrite)]
    mood: Mood,

    #[uproperty(BlueprintReadWrite)]
    channel: ECollisionChannel,

    #[uproperty(BlueprintReadWrite)]
    skin: SoftObjectRef<Material>,

    #[uproperty(BlueprintReadWrite)]
    spot: OwnedStruct<FVector>,

    #[uproperty(BlueprintReadWrite)]
    gear: OwnedStruct<Loadout>,

    #[uproperty(BlueprintReadWrite)]
    level: u8,

    #[uproperty(BlueprintReadWrite)]
    elapsed: f64,

    #[uproperty(BlueprintReadWrite)]
    serial: i64,

    #[uproperty(BlueprintReadWrite, default = -1)]
    finished_link: i32,
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

    #[udelegate]
    pub fn on_named(&self, name: String, tags: Vec<FName>, title: Text) {}

    #[udelegate(single)]
    pub fn on_asked(&self, question: String, weights: HashMap<String, i32>) {}

    #[udelegate(sparse)]
    pub fn on_scored(&self, points: i32, by: String) {}

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
    pub async fn count_up(self, steps: i32) {
        for _ in 0..steps {
            rusteal_runtime::runtime::task::next_frame().await;
            self.set_count(self.count() + 1);
        }
    }

    #[ufunction]
    pub fn latent_finished(&self, linkage: i32) {
        self.set_finished_link(linkage);
    }

    #[ufunction]
    pub fn mirror(
        &self,
        buddy: UObjectRef<Actor>,
        class: SubclassOf<Actor>,
        mood: Mood,
        channel: ECollisionChannel,
        skin: SoftObjectRef<Material>,
        gear: OwnedStruct<Loadout>,
        level: u8,
        serial: i64,
    ) -> String {
        self.set_buddy(buddy);
        self.set_buddy_class(class);
        self.set_mood(mood);
        self.set_channel(channel);
        self.set_skin(&skin);
        self.set_gear(&gear);
        self.set_level(level);
        self.set_serial(serial);
        format!("{mood:?} {channel:?} {} {level} {serial}", skin.path())
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

    #[ufunction(CallInEditor, category = "Debug")]
    pub fn reset_count(&self) {
        self.set_count(0);
    }

    #[ufunction(BlueprintNativeEvent)]
    pub fn reaction(&self, amount: i32) -> i32 {
        amount * 2
    }

    #[ufunction(BlueprintNativeEvent)]
    pub fn summary(
        &self,
        items: Vec<String>,
        owner: SoftObjectRef<Actor>,
        counts: HashMap<FName, i32>,
    ) -> String {
        format!("{} items", items.len() + counts.len() + owner.path().len())
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

impl TestActor {
    pub fn pinged(&mut self, value: i32) {
        self.set_serial(i64::from(value));
    }

    pub fn jumped(&self, value: UStructRef<FInputActionValue>) {
        if value.get_bool() {
            self.set_level(self.level().saturating_add(1));
        }
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

    #[ufunction(Override)]
    fn summary(
        &self,
        items: Vec<String>,
        owner: SoftObjectRef<Actor>,
        counts: HashMap<FName, i32>,
    ) -> String {
        format!(
            "{} of {} counting {}",
            items.join("+"),
            owner.path(),
            counts.values().sum::<i32>()
        )
    }
}

#[uclass(parent = BlueprintFunctionLibrary)]
pub struct TestLibrary {}

#[uclass_impl]
impl TestLibrary {
    /// Adds three numbers.
    #[ufunction(BlueprintPure, category = "Rusteal Tests")]
    pub fn add_three(a: i32, b: i32, c: i32) -> i32 {
        a + b + c
    }

    #[ufunction]
    pub fn shout(words: String) -> String {
        words.to_uppercase()
    }
}

#[uclass(parent = SaveGame)]
pub struct TestSave {
    #[uproperty(SaveGame)]
    score: i32,
}
