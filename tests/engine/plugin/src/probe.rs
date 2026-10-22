use bindings::core_ue::Object;
use bindings::engine::Actor;
use rusteal_runtime::{uclass, uclass_impl};

#[uclass(parent = Actor)]
pub struct ProbeTarget {
    #[uproperty(BlueprintReadWrite)]
    charge: i32,

    #[uproperty(BlueprintReadWrite)]
    label: String,

    secret: i32,
}

#[uclass_impl]
impl ProbeTarget {
    #[ufunction]
    pub fn add_charge(&mut self, amount: i32) -> i32 {
        *self.secret_mut() += 1;
        self.set_charge(self.charge() + amount);
        self.on_charged(self.charge());
        self.charge()
    }

    #[ufunction(BlueprintPure)]
    pub fn times_charged(&self) -> i32 {
        self.secret()
    }

    #[ufunction(BlueprintNativeEvent)]
    pub fn describe(&self) -> String {
        format!("probe at {}", self.charge())
    }

    #[ufunction(BlueprintImplementableEvent)]
    pub fn on_charged(&self, total: i32) {}

    #[udelegate]
    pub fn on_label_changed(&self, label: String) {}

    #[ufunction]
    pub fn rename(&mut self, label: String) {
        self.set_label(&label);
        self.on_label_changed(label);
    }
}

#[uclass(parent = Object)]
pub struct ProbeRecord {
    #[uproperty(BlueprintReadWrite)]
    charge: i32,

    secret: i32,
}

#[uclass_impl]
impl ProbeRecord {
    #[ufunction]
    pub fn add_charge(&mut self, amount: i32) -> i32 {
        *self.secret_mut() += 1;
        self.set_charge(self.charge() + amount);
        self.on_charged(self.charge());
        self.charge()
    }

    #[ufunction(BlueprintPure)]
    pub fn times_charged(&self) -> i32 {
        self.secret()
    }

    #[ufunction(BlueprintImplementableEvent)]
    pub fn on_charged(&self, total: i32) {}
}
