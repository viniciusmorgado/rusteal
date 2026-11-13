#include "Engine/AssetManager.h"
#include "Engine/StreamableManager.h"
#include "RustealApiTable.h"
#include "RustealErrors.h"
#include "RustealLibrary.h"
#include "RustealStrings.h"

namespace {
struct FRustealLoad {
  FRustealLibrary *Library = nullptr;
  TArray<FSoftObjectPath> Paths;
  TArray<FPrimaryAssetId> PrimaryAssets;
  TSharedPtr<FStreamableHandle> Handle;
};

TMap<uint64, FRustealLoad> GLoads;

FStreamableManager &Streamable() {
  static FStreamableManager Fallback;

  return UAssetManager::IsInitialized() ? UAssetManager::GetStreamableManager()
                                        : Fallback;
}

TArray<FString> SplitLines(const uint8 *Text, uint32 Len) {
  TArray<FString> Lines;
  RustealUtf8ToFString(Text, Len).ParseIntoArray(Lines, TEXT("\n"), true);
  return Lines;
}

FStreamableDelegate Wake(FRustealLibrary *Library, uint64 CallbackId) {
  return FStreamableDelegate::CreateLambda([Library, CallbackId]() {
    RustealCallLibrary(Library, [CallbackId](const FRustealRustCallbacks &Cb) {
      Cb.invoke_delegate_callback(CallbackId, nullptr);
    });
  });
}
} // namespace

ERustealErrorCode RustealLoading_LoadAsync(const uint8 *Paths, uint32 PathsLen,
                                           uint64 CallbackId) {
  FRustealLoad Load;
  Load.Library = RustealCurrentLibrary();

  for (const FString &Line : SplitLines(Paths, PathsLen)) {
    Load.Paths.Emplace(Line);
  }

  if (Load.Paths.IsEmpty()) {
    return RustealFail(ERustealErrorCode::NullArgument,
                       TEXT("load_async: no path to load"));
  }

  Load.Handle = Streamable().RequestAsyncLoad(
      Load.Paths, Wake(Load.Library, CallbackId), FStreamableManager::DefaultAsyncLoadPriority,
      false, false, TEXT("Rusteal load_async"));

  GLoads.Add(CallbackId, MoveTemp(Load));
  return ERustealErrorCode::Ok;
}

ERustealErrorCode RustealLoading_LoadPrimaryAssets(const uint8 *Ids,
                                                   uint32 IdsLen,
                                                   const uint8 *Bundles,
                                                   uint32 BundlesLen,
                                                   uint64 CallbackId) {
  if (!UAssetManager::IsInitialized()) {
    return RustealFail(ERustealErrorCode::InvalidOperation,
                       TEXT("load_primary_asset: the asset manager is not "
                            "initialized"));
  }

  FRustealLoad Load;
  Load.Library = RustealCurrentLibrary();

  for (const FString &Line : SplitLines(Ids, IdsLen)) {
    Load.PrimaryAssets.Emplace(FPrimaryAssetId::FromString(Line));
  }

  TArray<FName> BundleNames;

  for (const FString &Line : SplitLines(Bundles, BundlesLen)) {
    BundleNames.Emplace(*Line);
  }

  Load.Handle = UAssetManager::Get().LoadPrimaryAssets(
      Load.PrimaryAssets, BundleNames, Wake(Load.Library, CallbackId));

  const bool bQueued = Load.Handle.IsValid();
  GLoads.Add(CallbackId, MoveTemp(Load));

  if (!bQueued) {
    RustealCallLibrary(RustealCurrentLibrary(),
                       [CallbackId](const FRustealRustCallbacks &Cb) {
                         Cb.invoke_delegate_callback(CallbackId, nullptr);
                       });
  }

  return ERustealErrorCode::Ok;
}

ERustealErrorCode RustealLoading_Finish(uint64 CallbackId,
                                        RustealUObjectHandle *OutObjects,
                                        uint32 Capacity, uint32 *OutCount) {
  FRustealLoad Load;

  if (!GLoads.RemoveAndCopyValue(CallbackId, Load)) {
    return RustealFail(ERustealErrorCode::InvalidOperation,
                       TEXT("load_async: no load in progress"));
  }

  TArray<UObject *> Objects;

  for (const FSoftObjectPath &Path : Load.Paths) {
    Objects.Add(Path.ResolveObject());
  }

  for (const FPrimaryAssetId &Id : Load.PrimaryAssets) {
    Objects.Add(UAssetManager::Get().GetPrimaryAssetObject(Id));
  }

  if (OutCount) {
    *OutCount = static_cast<uint32>(Objects.Num());
  }

  for (int32 Index = 0; Index < Objects.Num() && Index < (int32)Capacity;
       ++Index) {
    OutObjects[Index] = RustealUObjectHandle{Objects[Index]};
  }

  if (Load.Handle.IsValid()) {
    Load.Handle->ReleaseHandle();
  }

  return ERustealErrorCode::Ok;
}

ERustealErrorCode RustealLoading_Cancel(uint64 CallbackId) {
  FRustealLoad Load;

  if (GLoads.RemoveAndCopyValue(CallbackId, Load) && Load.Handle.IsValid()) {
    Load.Handle->CancelHandle();
  }

  return ERustealErrorCode::Ok;
}

uint32 RustealLoading_InProgress() { return static_cast<uint32>(GLoads.Num()); }

void RustealLoadingForgetLibrary(FRustealLibrary *Library) {
  TArray<uint64> Owned;

  for (const TPair<uint64, FRustealLoad> &Entry : GLoads) {
    if (Entry.Value.Library == Library) {
      Owned.Add(Entry.Key);
    }
  }

  for (uint64 Id : Owned) {
    RustealLoading_Cancel(Id);
  }
}
