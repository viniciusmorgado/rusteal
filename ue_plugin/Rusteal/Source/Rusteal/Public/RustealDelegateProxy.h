#pragma once

#include "UObject/Object.h"
#include "RustealDelegateProxy.generated.h"

struct FRustealLibrary;

UCLASS()
class URustealDelegateProxy : public UObject {
  GENERATED_BODY()

public:
  uint64 CallbackId = 0;

  FRustealLibrary *Library = nullptr;

  UFunction *Signature = nullptr;

  TWeakObjectPtr<UObject> OwnerObject;

  static FName FakeFuncName;

  static void Keep(URustealDelegateProxy *Proxy);

  static void Release(URustealDelegateProxy *Proxy);

  UFUNCTION()
  void RustFakeCallable();

  virtual void ProcessEvent(UFunction *Function, void *Parms) override;

  virtual void BeginDestroy() override;

  virtual UWorld *GetWorld() const override;
};
