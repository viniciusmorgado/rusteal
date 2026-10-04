#pragma once

#include "CoreMinimal.h"
#include "RustealApiTable.h"

// One hosted Rust library (see RustealLibraries.h). Entries are never freed:
// classes, functions and delegate bindings keep a pointer to the library that
// made them, which outlives an unload as a library without callbacks.
struct FRustealLibrary {
  FName Name;

  /** The package its classes and structs live in: /Script/Rusteal for the
   * game's, the registering module's own (/Script/<Plugin>) for a plugin's,
   * as a C++ module's classes live in its package. */
  FString PackagePath;

  /** The deployed library; each (re)load uses a numbered copy of it. */
  FString SourcePath;
  FString LoadedPath;
  int32 ReloadCount = 0;

  void *Handle = nullptr;
  const FRustealRustCallbacks *Callbacks = nullptr;

  /** This library's API table: the shared sub-tables and its own generated
   * function table. Rust keeps a pointer to it while loaded. */
  FRustealApiTable Table;

  bool IsLoaded() const { return Callbacks != nullptr; }
};

/** The library Rust is running for right now: Rust code only runs inside a
 * call from C++ into one library (init, a function, a delegate, construct,
 * drop, shutdown), so an API call that registers something (a class, a
 * delegate binding, a pinned object) belongs to this one. */
FRustealLibrary *RustealCurrentLibrary();

/** The package the current library's classes and structs go in (created on
 * first use); /Script/Rusteal when no library is current. */
UPackage *RustealCurrentPackage();

/** Makes Library the current one for the scope. */
struct FRustealLibraryScope {
  explicit FRustealLibraryScope(FRustealLibrary *Library);
  ~FRustealLibraryScope();

private:
  FRustealLibrary *Previous;
};

/** Call into Library's callbacks with it current; nothing when it is not
 * loaded. */
template <typename FnType>
void RustealCallLibrary(FRustealLibrary *Library, FnType &&Fn) {
  if (Library && Library->Callbacks) {
    FRustealLibraryScope Scope(Library);
    Fn(*Library->Callbacks);
  }
}
