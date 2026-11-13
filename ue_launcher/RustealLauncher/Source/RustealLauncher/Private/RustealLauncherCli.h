// Copyright 2026 Vinicius Donatto Morgado.

#pragma once

#include "CoreMinimal.h"

struct FRustealLauncherStub {
  FString Template;
  FString Variant;

  static TOptional<FRustealLauncherStub> Read(const FString &ProjectDir);
};

namespace RustealLauncherCli {
FString CargoBinDir();
FString FindRusteal();
FString FindCargo();
TOptional<FString> ReadVersion(const FString &Executable);
bool CreatesFromStubs(const FString &Rusteal);
bool IsValidProjectName(const FString &Name);
void AddCargoBinToPath();
FString NewProjectArguments(const FRustealLauncherStub &Stub);
FString ProjectParentDir();
} // namespace RustealLauncherCli
