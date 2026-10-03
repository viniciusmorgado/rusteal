#pragma once

#include "CoreMinimal.h"

// The Rust libraries the Rusteal plugin hosts.
//
// A project has its game's library, which the Rusteal module loads itself, and
// one library per Rusteal plugin, which the plugin's own module hands over from
// its StartupModule. Each library comes with the function table generated for
// it (its bindings and C++ wrappers), and is loaded, hot reloaded
// (`Rusteal.Reload [Name]`) and unloaded on its own.

/** Load the Rust library at LibraryPath and initialize it under Name, with the
 * generated function table of the module that registers it; its classes and
 * structs go in the package /Script/<Name>, the module's own. Returns false
 * when the library is missing or refused (another Rusteal version, a failed
 * init); the reason is in the Output Log. */
RUSTEAL_API bool RustealRegisterLibrary(FName Name, const FString &LibraryPath,
                                        void *const *FuncTable,
                                        uint32 FuncCount);

/** RustealRegisterLibrary for the library of the plugin PluginName, deployed by
 * `rusteal build` as <Plugin>/Binaries/<Platform>/rusteal_<PluginName>
 * (librusteal_<PluginName>.so on Linux, rusteal_<PluginName>.dll on Windows). */
RUSTEAL_API bool RustealRegisterPluginLibrary(const FString &PluginName,
                                              void *const *FuncTable,
                                              uint32 FuncCount);

/** Shut the library down and unload it. Objects of its classes stay; they lose
 * their Rust data. Called from the registering module's ShutdownModule. */
RUSTEAL_API void RustealUnregisterLibrary(FName Name);

/** The file name `rusteal build` deploys a library as: Stem with the platform's
 * prefix and extension (librusteal.so, rusteal.dll, librusteal.dylib). */
RUSTEAL_API FString RustealLibraryFileName(const FString &Stem);
