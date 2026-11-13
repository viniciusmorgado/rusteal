#pragma once

#include "Engine/LatentActionManager.h"
#include "UObject/Object.h"
#include "RustealLatentProxy.generated.h"

struct FRustealLibrary;

UCLASS(Transient)
class RUSTEAL_API URustealLatentProxy : public UObject {
  GENERATED_BODY()

public:
  uint64 CallbackId = 0;

  FRustealLibrary *Library = nullptr;

  TWeakObjectPtr<UObject> Context;

  TFunction<void()> OnComplete;

  bool bFinished = false;

  static URustealLatentProxy *Create(UObject *Context, uint64 CallbackId);

  FLatentActionInfo MakeLatentInfo();

  UFUNCTION()
  void LatentDone(int32 Linkage);

  void Cancel();

  virtual void BeginDestroy() override;

  virtual UWorld *GetWorld() const override;
};
