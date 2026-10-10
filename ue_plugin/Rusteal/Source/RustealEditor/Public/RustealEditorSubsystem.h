#pragma once

#include "CoreMinimal.h"
#include "EditorSubsystem.h"
#include "RustealEditorSubsystem.generated.h"

UCLASS(Abstract, Blueprintable)
class RUSTEALEDITOR_API URustealEditorSubsystem : public UEditorSubsystem {
  GENERATED_BODY()

public:
  virtual bool ShouldCreateSubsystem(UObject *Outer) const override;
  virtual void Initialize(FSubsystemCollectionBase &Collection) override;
  virtual void Deinitialize() override;

  UFUNCTION(BlueprintNativeEvent, Category = "Rusteal|Subsystem")
  bool ReceiveShouldCreateSubsystem(UObject *Outer) const;

  UFUNCTION(BlueprintImplementableEvent, Category = "Rusteal|Subsystem")
  void ReceiveInitialize();

  UFUNCTION(BlueprintImplementableEvent, Category = "Rusteal|Subsystem")
  void ReceiveDeinitialize();
};
