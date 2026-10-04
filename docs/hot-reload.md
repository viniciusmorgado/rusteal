# Hot reload

How Rusteal swaps a Rust library while the editor runs, and why it is built the
way it is. The commands are in the README, [Hot Reload](../README.md#hot-reload).

## What a reload does

`Rusteal.Reload [Name]` reloads one library, or all of them:

1. The Rust data of every object of the library's classes is dropped. The
   UE properties (`#[uproperty]`) live in the objects' memory and stay;
   Rust-private fields start over from `Default`.
2. The library is unloaded and the deployed file is loaded again, from a
   numbered copy (`librusteal_hot_N.so`), so a build can always overwrite the
   deployed file.
3. `rusteal_init` registers the library's classes again. A class whose
   **shape** is unchanged is reused: its functions point to the new code. A
   class whose shape changed is replaced (below).
4. The objects of the replaced classes are reinstanced.
5. Every object of the library's classes gets its Rust data again.

## Reloading on deploy

The RustealEditor module reloads a library by itself when `rusteal build`
deploys it again (`RustealAutoReload.cpp`): it watches the directory each
library is deployed to with the engine's DirectoryWatcher, and once the
deployed file has not changed for half a second, reloads that library and
says so in an editor notification. `Rusteal.AutoReload 0` turns it off;
`Rusteal.Reload` still works either way.

The deploy step copies the library next to the deployed file
(`librusteal.so.partial`) and renames it over it, so the watcher never sees
a half-written library. The numbered copies a reload loads and the partial
file sit in the same directory and are ignored: only the deployed file
counts.

## Shapes

A class's shape is a hash of what its UClass is built from, computed by the
macros at compile time:

- `#[uclass(...)]`'s arguments: the parent, `implements`, `config`;
- each `#[uproperty]` and `#[component]` field: its attribute, name and type;
- each `#[ufunction]` and `#[udelegate]` method of its `#[uclass_impl]`
  blocks: its attribute and signature.

Method bodies, doc comments and Rust-private fields are not part of it, so
changing code alone keeps the class. The order of fields and methods does not
count either. A class whose Rust parent was replaced is replaced too, since
its layout starts with its parent's.

A `#[ustruct]` has a shape as well, but structs are not reinstanced: a changed
struct is logged, and keeps its old fields until the editor restarts.

## Replacing a class

When a class's shape changed, `CreateClassImpl`
(`ue_plugin/Rusteal/Source/Rusteal/Private/RustealReifyApiImpl.cpp`) retires
the old class and creates a new one with the same name:

- the old class moves to the transient package as `RUSTEAL_<Name>`, with its
  default object, as the engine's reload moves a changed native class
  (`ReloadProcessObject` in `CoreUObject/Private/UObject/DeferredRegistry.cpp`);
- it is cut from Rust: its objects get no Rust data, its functions do nothing;
- its stub Blueprint moves with it, and the class stops pointing to it, so
  the engine does not try to recompile a Rust subclass's stub.

Once the library is loaded, the module hands the (old, new) pairs to the
class reinstancer, which the RustealEditor module registers.

## Why FReload, not Live Coding

Live Coding is the editor's own hot reload for C++, so it was the first thing
to look at. It does not fit, for three reasons:

- **It is Windows only.** Its sources are in
  `Engine/Source/Developer/Windows/LiveCoding`; on Linux and macOS it is not
  built at all.
- **It patches machine code.** It recompiles the changed C++ files with the
  engine's compiler and patches the functions inside the running process (it
  is Live++ underneath). A Rust library is built by Cargo, not by UBT: there
  is nothing for it to patch.
- **Rusteal already swaps code, on every platform.** Every call between the
  engine and Rust goes through function tables, so loading the new library
  and pointing the functions at it is the whole code swap.

What Rusteal lacked was the other half: moving existing objects to a class
whose structure changed. **Live Coding does not do that part itself either.**
With *Enable Reinstancing* on (`bEnableReinstancing`), it hands the changed
classes to `FReload`, an editor class of the engine:

- `Engine/Source/Editor/UnrealEd/Public/Kismet2/ReloadUtilities.h`, exported
  with `UNREALED_API`, so any editor module can use it;
- `NotifyChange(New, Old)` for each changed class, then `Reinstance()`: the
  engine reinstances the objects (actors in the level included), the default
  object, and the Blueprint children of the old class, which it reparents to
  the new one and recompiles; `Finalize()` replaces the remaining references
  and collects the old objects.

The RustealEditor module does exactly that
(`ue_plugin/Rusteal/Source/RustealEditor/Private/RustealEditorModule.cpp`),
with `EActiveReloadType::Reinstancing`. Rusteal only has to detect the change
and create the new class; the migration is the engine's, the same code path
the editor uses for Live Coding and Blueprint compiles.

## Limits

What FReload does not support for Live Coding, Rusteal does not support
either; and classes created at runtime are not what it was written for. What
is known:

- **Without the editor** (a game, `-game`), there is no reinstancer: a changed
  class is kept as it was, with a warning to restart.
- **Structs** (`#[ustruct]`) are not reinstanced (above).
- **Rust-private fields** start over on every reload, changed or not.
- Removing a property or a function that a Blueprint uses breaks that
  Blueprint's nodes, as it would for a C++ class; the Blueprint shows the
  errors after its recompile.

What is tested, and what is still to be tested, is in the test list of the
branch that brought this (m1/structural-reload onwards).
