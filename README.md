# Rusteal

**Rust bindings for Unreal Engine 5.8**

Rusteal lets you write Unreal Engine gameplay in Rust. Your Rust code compiles to a shared library that is loaded by a small UE C++ plugin. All UE API calls cross the FFI boundary through a function pointer table — no C++ compilation required during Rust iteration.

> **⚠️ Early Stage Project** — Rusteal is under active development and **not ready for production use**. APIs will change without notice, documentation is incomplete, and many UE features are not yet covered. Contributions and feedback are welcome, but please do not use this for shipping projects.

## Acknowledgments

Rusteal is a hard fork of [**uika**](https://github.com/VioletHelianthus/uika) by [**VioletHelianthus**](https://github.com/VioletHelianthus), who designed and wrote the foundation this project stands on: the reflection-driven code generator, the UHT exporter plugin, the reification of Rust structs as `UClass`es, and the runtime. Thank you.

Rusteal does not intend to stay compatible with the original uika, unless its author wants to port the improvements back. From here on it develops the Unreal bindings independently.

## Example

See [`example_game/src/game_demo.rs`](example_game/src/game_demo.rs) for the full working demo.

## Getting Started

### Prerequisites

- **Unreal Engine 5.8** (source or installed build)
- **Rust** (stable, latest recommended)
- **Linux**: the clang toolchain and .NET runtime bundled with the engine
- **Windows**: Visual Studio 2022 with the C++ workload

### Setup

1. **Create your UE project** (or use an existing one).

2. **Clone this repo**:
   ```bash
   git clone https://github.com/viniciusmorgado/rusteal.git
   cd rusteal
   ```

3. **Configure** — copy and edit the config file:
   ```bash
   cp rusteal.config.toml.example rusteal.config.toml
   ```
   Edit `rusteal.config.toml` to set your UE engine path, project path and game crate:
   ```toml
   [ue]
   engine_path = "/opt/UnrealEngine-5.8"

   [project]
   path = "/path/to/YourProject"

   [build]
   crate_name = "your-game"
   crate_path = "/path/to/YourProject/Rust"

   [codegen.paths]
   cpp_out = "/path/to/YourProject/Plugins/Rusteal/Source/Rusteal/Generated"
   ```

4. **Set up the UE plugins** in your project:
   ```bash
   cargo run -p rusteal -- setup /path/to/YourProject --config rusteal.config.toml
   ```

5. **Build everything** (UE build → codegen → UE rebuild → Rust compile → deploy):
   ```bash
   cargo run -p rusteal -- build --config rusteal.config.toml
   ```

### Create your game crate

```bash
cargo new --lib your-game
```

**`Cargo.toml`:**
```toml
[lib]
crate-type = ["cdylib"]

[dependencies]
rusteal-runtime = { path = "../rusteal/rusteal-runtime", features = ["engine"] }
glam = "0.33"
```

**`src/lib.rs`:**
```rust
rusteal_runtime::entry!();

mod my_game;
```

## Build Pipeline

The CLI orchestrates a 5-step build:

| Step | Command | What it does |
|------|---------|-------------|
| 1 | UE Build | Compiles UE project, triggers RustealGenerator → JSON reflection data |
| 2 | Codegen | Reads JSON → generates Rust bindings + C++ wrappers |
| 3 | UE Rebuild | Compiles the generated C++ wrappers into the UE module |
| 4 | Cargo Build | `cargo build --release` on your cdylib crate |
| 5 | Deploy | Copies the library to `Plugins/Rusteal/Binaries/<Platform>/` (`rusteal.dll`, `librusteal.so`) |

Common shortcuts:
```bash
# Full build
cargo run -p rusteal -- build

# Rust-only rebuild (skip UE steps)
cargo run -p rusteal -- build --from 4

# Codegen + everything after
cargo run -p rusteal -- build --from 2

# Just regenerate bindings
cargo run -p rusteal -- generate
```

## Key Concepts

### Object References

```rust
// Lightweight handle (8 bytes, Copy). May become invalid if UE GCs the object.
let actor: UObjectRef<Actor> = world.spawn_actor(&transform)?;

// RAII GC root. Prevents garbage collection until dropped.
let pinned: Pinned<Actor> = actor.pin()?;

// Checked access — verifies the object is still alive before use.
let checked = actor.checked()?;
checked.k2_get_actor_location();
```

### Defining UE Classes

```rust
#[uclass(parent = Actor)]
pub struct MyActor {
    #[component(root)]
    root: SceneComponent,

    #[component(attach = "root")]
    mesh: StaticMeshComponent,

    #[uproperty(BlueprintReadWrite, default = 100)]
    health: i32,

    // Rust-only field (not exposed to UE)
    internal_state: Vec<String>,
}

#[uclass_impl]
impl MyActor {
    #[ufunction(Override)]
    fn receive_begin_play(&mut self) { /* ... */ }

    #[ufunction(BlueprintCallable)]
    fn take_damage(&mut self, amount: i32) {
        self.set_health(self.health() - amount);
    }
}
```

### Dynamic Calls

For Blueprint-defined functions or APIs not covered by generated bindings:

```rust
let mut call = DynamicCall::new(&actor, "SetMobility")?;
call.set("NewMobility", 2u8)?;
call.call()?;
```

### Hot Reload

During development, rebuild your Rust library and reload without restarting the editor:

```bash
cargo run -p rusteal -- build --from 4
```

Then in the UE console:
```
Rusteal.Reload
```

Function implementations update immediately. Adding/removing `uproperty` or `ufunction` requires an editor restart.

## Platform Support

| Platform | Status |
|----------|--------|
| Linux (x64) | Supported (Unreal Engine 5.8.2) |
| Windows (x64) | Supported |
| macOS | Not yet tested |

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.
