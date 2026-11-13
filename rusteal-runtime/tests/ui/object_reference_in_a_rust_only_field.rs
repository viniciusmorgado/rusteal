// A Rust-only field is invisible to the garbage collector: keeping an object there is refused.

#[rusteal_runtime::uclass(parent = Actor)]
pub struct Guard {
    target: Option<rusteal_runtime::runtime::UObjectRef<Actor>>,
}

fn main() {}
