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
TOptional<FString> ReadVersion(const FString &Executable);
void AddCargoBinToPath();
FString NewProjectArguments(const FRustealLauncherStub &Stub);
FString ProjectParentDir();
} // namespace RustealLauncherCli
