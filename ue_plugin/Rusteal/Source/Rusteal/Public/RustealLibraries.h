#pragma once

#include "CoreMinimal.h"

RUSTEAL_API bool RustealRegisterLibrary(FName Name, const FString &LibraryPath,
                                        void *const *FuncTable,
                                        uint32 FuncCount);

RUSTEAL_API bool RustealRegisterPluginLibrary(const FString &PluginName,
                                              void *const *FuncTable,
                                              uint32 FuncCount);

RUSTEAL_API void RustealUnregisterLibrary(FName Name);

RUSTEAL_API FString RustealLibraryFileName(const FString &Stem);

RUSTEAL_API void RustealForEachLibrary(
    TFunctionRef<void(FName Name, const FString &DeployedPath)> Callback);

DECLARE_MULTICAST_DELEGATE_TwoParams(FRustealLibraryRegistered, FName,
                                     const FString &);
RUSTEAL_API FRustealLibraryRegistered &RustealOnLibraryRegistered();

RUSTEAL_API bool RustealReloadLibrary(FName Name);

using FRustealClassReinstancer =
    TFunction<void(const TArray<TPair<UClass *, UClass *>> &Classes)>;

RUSTEAL_API void
RustealSetClassReinstancer(FRustealClassReinstancer Reinstancer);
