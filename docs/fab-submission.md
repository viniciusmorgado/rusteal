# Fab submission

What publishing the launcher (`ue_launcher/RustealLauncher`) on Fab takes, what is ready, and what to
ask Fab first. Nothing has been submitted. The launcher itself is described in the developer's
handbook, [The launcher](https://viniciusmorgado.github.io/dev.rusteal.github.io/architecture/launcher.html).

Sources: [Fab technical requirements](https://www.fab.com/technical-requirements) (updated
2026-09-21), [Publishing listings](https://dev.epicgames.com/documentation/en-us/fab/publishing-assets-for-sale-or-free-download-in-fab),
[Licenses and pricing](https://dev.epicgames.com/documentation/en-us/fab/licenses-and-pricing-in-fab),
[Plugin compilation troubleshooting](https://support.fab.com/s/article/Fab-Plugin-Compilation-Troubleshooting?language=en_US).
Section numbers below are the technical requirements'.

## Technical requirements

| Requirement | State |
|---|---|
| At least one C++ module; editor functionality (4.3.6.1.c, d) | Ready: the `RustealLauncher` editor module. |
| Epic builds the binaries with `BuildPlugin`, with no errors or consequential warnings (4.3.6.2) | Ready on Linux: `BuildPlugin -Rocket` builds with no warnings. Windows and Mac are not built yet; Epic's Mac builds treat more warnings as errors. |
| `EngineVersion` in the descriptor (4.3.6.a) | Ready: `5.8.0`. |
| `PlatformAllowList` in every module (4.3.6.b) | Ready: Win64, Mac, Linux. |
| A Fab URL in the descriptor (4.3.6.c) | Missing until the listing has an id. `BuildPlugin` keeps only the fields it knows, `MarketplaceURL` among them, and drops a `FabURL`. |
| A copyright notice with the publisher's name in every source file (4.3.6.1.b) | Ready: one line in each `.cpp` and `.h`. |
| No `.exe` or `.msi` (4.3.6.1.e) | Ready: the CLI is installed by the user with `cargo install`. |
| No dependency on another user-made plugin (4.3.6.d) | Ready: the launcher depends on engine modules only. |
| Folders other than the defaults listed in `Config/FilterPlugin.ini` (4.3.7.3.b) | Ready: `/Config/...` and `/Templates/...`. |
| The zip: the `.uplugin` at its root, no `Binaries`, `Build`, `Intermediate` or `Saved` (4.3.7.3.a) | To do at submission: zip `ue_launcher/RustealLauncher` as it is in the repository. |
| Paths of 170 characters at most, English letters, digits and underscores (4.3.7) | Ready: the longest template path is well under it. |
| Works on a binary engine, without building UE from source (4.3.1.i) | Ready: built and run on an installed UE 5.8.3. |
| Thorough, free documentation in English, including how to install third-party software that cannot be redistributed (4.3.8) | Ready: [Creating a project from the editor](https://viniciusmorgado.github.io/game.rusteal.github.io/getting_started/from_the_editor.html) and the installation of Rust and the CLI. |
| Dependencies and prerequisites in Technical Information (1.8.4.b) | To write in the listing: the Rust toolchain and the Rusteal CLI. |

## The listing

| Field | Draft |
|---|---|
| Title | Rusteal: Rust for Unreal Engine |
| Category | Tools & Plugins, Engine Tools |
| Price and licence | Free. Fab offers the Standard licence (its Personal and Professional tiers may both be free) or CC BY 4.0; the code stays MIT or Apache-2.0 on GitHub. The choice is the maintainer's. |
| Engine versions | 5.8 |
| Platforms | Windows, macOS, Linux (the editor); the games build for what Rusteal supports |
| Links | The repository, both handbooks, the issue tracker as the support channel |
| Created with AI | A required field of the form; what it declares is the maintainer's decision. |
| Images | At least one of 1920 × 1080: the Rust category in the New Project dialog, the setup window, a template running. The current category and template images are placeholders made from the logo and the template screenshots. |

**Description draft.** Rusteal lets you write your game's gameplay in Rust, with the engine's
reflection behind it: classes, properties, components and events in Rust, Blueprint children of
Rust classes, hot reload in the editor. This plugin adds a Rust category to the New Project dialog,
with every Rusteal template: Blank, First Person, Third Person, Top Down, Vehicle, VR, AR and the
Intro, and their variants. Pick one, and the project sets itself up the first time it opens.
Requires the Rust toolchain (rustup.rs) and the Rusteal CLI, which the plugin installs for you.

## Questions for Fab support

To send before the submission, through [Fab support](https://support.fab.com/s/?ProductOrigin=FabSupport):

1. *Required software.* The plugin creates projects that build Rust code, so it needs the Rust
   toolchain and our command-line tool, which users install with `cargo install` (the plugin offers
   to run it). Nothing is downloaded by the plugin otherwise. Is a code plugin that requires such a
   toolchain, documented in Technical Information, acceptable?
2. *Project templates.* The plugin ships project templates in its own `Templates/` folder, listed
   in `Config/FilterPlugin.ini`, which the New Project dialog shows under a category of their own.
   Is that accepted?
3. *Linux.* Does Epic's build farm produce Linux binaries for code plugins today, for UE 5.8?
4. *Later plugins.* For editor plugins we plan to sell, part of the code is written in Rust and
   would ship as a prebuilt static library per platform in `Source/ThirdParty`, with its Rust
   sources. The library contains no Unreal Engine source code; it calls the engine through a table
   of C function pointers that the plugin's C++ module provides, and some of its code is generated
   from the engine's reflection data. Does that meet 4.3.6.1.a?
5. *Build plugins.* Does the build farm accept a C# UnrealBuildTool/UHT plugin
   (`*.ubtplugin.csproj`) inside a code plugin?

## Editor plugins made with Rusteal

An editor tool written with Rusteal goes to Fab as `<output>/<Name>-Source` from
`rusteal plugin package <Name> --standalone`: the plugin's C++ sources with a
renamed copy of the Rusteal runtime as modules of its own, no dependency on any
other plugin made by a user, no Rust sources, and its Rust library prebuilt in
`Source/ThirdParty/RustLibraries/<Platform>/` (no `Binaries/`, which Fab refuses
in a zip). Question 4 above is about that library. Each platform's library comes
from packaging on that platform; `rusteal plugin merge` joins the packages.

