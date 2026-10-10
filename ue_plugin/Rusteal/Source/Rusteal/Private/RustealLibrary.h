#pragma once

#include "CoreMinimal.h"
#include "RustealApiTable.h"

struct FRustealLibrary {
  FName Name;

  FString PackagePath;

  FString SourcePath;
  FString LoadedPath;
  int32 ReloadCount = 0;

  void *Handle = nullptr;
  const FRustealRustCallbacks *Callbacks = nullptr;

  FRustealApiTable Table;

  bool IsLoaded() const { return Callbacks != nullptr; }
};

FRustealLibrary *RustealCurrentLibrary();

UPackage *RustealCurrentPackage();

struct FRustealLibraryScope {
  explicit FRustealLibraryScope(FRustealLibrary *Library);
  ~FRustealLibraryScope();

private:
  FRustealLibrary *Previous;
};

template <typename FnType>
void RustealCallLibrary(FRustealLibrary *Library, FnType &&Fn) {
  if (Library && Library->Callbacks) {
    FRustealLibraryScope Scope(Library);
    Fn(*Library->Callbacks);
  }
}
