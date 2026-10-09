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

  UPROPERTY()
  TObjectPtr<UObject> OwnerObject;

  static FName FakeFuncName;

  UFUNCTION()
  void RustFakeCallable();

  virtual void ProcessEvent(UFunction *Function, void *Parms) override;
};
