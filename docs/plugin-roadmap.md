# Plugin roadmap

What Rusteal does not do yet for writing Unreal Engine plugins (editor tools and
runtime gameplay plugins) in Rust, and why. What it does is in the README,
[Making a plugin](../README.md#making-a-plugin).

The plugin features Rusteal covers were picked as the ones plugins use most:
a plugin's own Rust library next to the game's, runtime and editor plugins,
subsystems, project settings, console commands and variables, work off the game
thread, editor menus, asset validation, editor scripting (assets, actors,
undo), data assets, and packaging plugins and the games that use them.
Everything below is less used, or needs work outside UObject reflection,
which is what Rusteal is built on.

Each entry says what is missing, what it takes, and what to do meanwhile.

## Editor UI

| Feature | What it takes | Meanwhile |
|---|---|---|
| **Slate** (custom windows, tabs, panels, any non-UMG editor UI) | Slate widgets are not UObjects: the Unreal Header Tool never sees them, so no binding is generated. Needs hand-written bindings for `SNew`/`SAssignNew`, slots, attributes and delegates, which is a project of its own. | UMG widgets, which are reflected, and editor menus (`ToolMenus`). |
| **Details panel customization** (`IDetailCustomization`, `IPropertyTypeCustomization`) | Slate, and the PropertyEditor module's C++ interfaces. | Property metadata (`EditAnywhere`, categories, `EditCondition`) on Rust properties. |
| **Editor Utility Widgets with a Rust parent** | The editor spawns them from a `UEditorUtilityWidgetBlueprint` asset or a `UWidgetBlueprintGeneratedClass`; a Rust class is neither and has no widget tree to show. Needs a native base class that builds its tree, or an asset made with the designer. | An Editor Utility Widget Blueprint whose graph calls Rust functions. |
| **Scripted asset and actor actions** (`UAssetActionUtility`, `UActorActionUtility`) | The editor finds them only as Blueprint assets on disk, through the asset registry; there is no registration call. Needs a native shim that registers Rust classes. | A `ToolMenus` entry in the content browser's or the level's context menu (`ContentBrowser.AssetContextMenu`, `LevelEditor.ActorContextMenu`). |
| **Custom asset editors and graph editors** | Slate and the asset editor toolkit, C++ only. | A data asset edited in the default details view. |
| **Editor modes and interactive tools** (`UEdMode`, gizmos) | C++ virtuals of the tools framework. The Scriptable Tools Framework (`UScriptableInteractiveTool`, Beta in 5.8, disabled by default) is Blueprintable and should work with Rust classes; not tested yet. | — |

## Assets and import

| Feature | What it takes | Meanwhile |
|---|---|---|
| **New asset types** (`UFactory`, asset type actions, thumbnails) | Factories and asset definitions are C++ virtuals. Needs native shims with events. | A Rust `PrimaryDataAsset` subclass: the editor creates it from *Miscellaneous > Data Asset*. |
| **Import pipelines and translators** (Interchange) | `UInterchangePipelineBase` is Blueprintable with native events (`ScriptedExecutePipeline`...) and should take a Rust class; the Interchange modules are not in the default bindings and nothing was tested. Translators (parsing a new file format) are C++ virtuals. A Rust strength (parsers from crates.io), worth a milestone of its own. | Parse the file in Rust and create the assets with the editor scripting API. |
| **Commandlets** (`UCommandlet::Main`) | A C++ virtual: needs a native shim. | A console command, run headless with `-ExecCmds`, or editor Python. |

## Blueprint integration

| Feature | What it takes | Meanwhile |
|---|---|---|
| **Async Blueprint nodes** (`UBlueprintAsyncActionBase`) | The node comes from a static factory function marked `BlueprintInternalUseOnly`; Rust functions are not static. | A delegate on a Rust object, broadcast when the work is done. |
| **Latent functions** (`FLatentActionInfo`) | Not tested from Rust. | Same as above. |
| **Custom Blueprint nodes** (`UK2Node`) | Editor C++ (graph schema, node expansion). | Rust functions with `BlueprintCallable`. |

## Libraries and distribution

| Feature | What it takes | Meanwhile |
|---|---|---|
| **A plugin with both a runtime and an editor library** | A plugin has one Rust library, runtime or editor. Two would need two C++ modules and two bindings crates in one plugin. | Two plugins, `Name` and `NameEditor`, the editor one depending on the runtime one. |
| **Typed calls into another library's Rust classes** | A game's Rust code calling a plugin's Rust class with its Rust type: the class lives in another library, whose data a Rust crate cannot share across the library boundary. Needs generated bindings for Rust classes, from their reflection. | `DynamicCall` by name, or interfaces (`implements = [...]`) the plugin's classes implement. |
| **Rust classes whose parent is another library's Rust class** | Same reason: the parent's Rust type is not visible to the child's crate. | A Blueprint child of the plugin's class. |
| **A plugin library built against another Rusteal version** | A library must match the Rusteal plugin's version exactly, as the game's does. Tolerating minor versions needs a stable API table layout. | Rebuild the plugin with the project's Rusteal version. |
| **Precompiled plugin packages** | `rusteal plugin package` copies the plugin with its sources, so the project that takes it compiles its C++ module. The engine's `BuildPlugin` (binaries for every configuration, no sources needed) builds the plugin in a project of its own, where the Rusteal plugin it depends on is missing. Needs Rusteal installed in the engine, or a host project of Rusteal's. | `rusteal plugin package`, built by the project that takes it. |
| **A plugin package for several platforms at once** | The Rust library is built for the platform the command runs on; Cargo cross-compilation to the engine's other platforms is not set up. | Package the plugin on each platform and merge the `Binaries/<Platform>/` directories. |
| **Fab and engine-wide installs** | A plugin's library is found from the plugin's directory, so a plugin in `Engine/Plugins/Marketplace` should load; not tested. Fab also wants builds for every platform and engine version it lists. | Distribute the plugin as a project plugin. |
| **macOS** | The whole of Rusteal is untested there. | — |

## Scripting

| Feature | What it takes | Meanwhile |
|---|---|---|
| **Python names for Rust properties** | Editor Python does not generate wrapper types for Blueprint-generated classes, which Rust classes are to the engine, so `get_editor_property` takes the UE name (`"MaxStack"`), not the snake case it takes for C++ classes (`"max_stack"`). | The UE name. Functions are reached by `call_method("UEName", (args,))`. |

## Runtime

| Feature | What it takes | Meanwhile |
|---|---|---|
| **Replication of plugin classes** | Rusteal does not cover multiplayer yet. | — |
| **Shaders and render passes** (global shaders, scene view extensions) | C++ and the render thread. | Materials and Niagara, which are assets. |
| **Changing a `#[ustruct]`'s fields during hot reload** | A hot reload replaces a class whose properties or functions changed and reinstances its objects ([hot-reload.md](hot-reload.md)); structs are not reinstanced, so a changed one keeps its old fields. | Restart the editor. |
