#pragma once

#include "CoreMinimal.h"
#include "GameFramework/Actor.h"
#include "RustealShims.generated.h"

UCLASS(Abstract, Blueprintable)
class RUSTEAL_API ARustealActor : public AActor {
  GENERATED_BODY()

public:
  virtual void PreInitializeComponents() override;
  virtual void PostInitializeComponents() override;

  UFUNCTION(BlueprintImplementableEvent, Category = "Rusteal|Actor")
  void ReceivePreInitializeComponents();

  UFUNCTION(BlueprintImplementableEvent, Category = "Rusteal|Actor")
  void ReceivePostInitializeComponents();
};
