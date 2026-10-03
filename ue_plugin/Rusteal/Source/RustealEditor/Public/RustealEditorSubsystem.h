#pragma once

#include "CoreMinimal.h"
#include "EditorSubsystem.h"
#include "RustealEditorSubsystem.generated.h"

/** A subsystem living as long as the editor, for editor tools written in Rust:
 * the engine creates one per Rust class deriving from it when the editor
 * starts, and its lifetime comes as events (see RustealSubsystems.h). */
UCLASS(Abstract, Blueprintable)
class RUSTEALEDITOR_API URustealEditorSubsystem : public UEditorSubsystem {
  GENERATED_BODY()

public:
  virtual bool ShouldCreateSubsystem(UObject *Outer) const override;
  virtual void Initialize(FSubsystemCollectionBase &Collection) override;
  virtual void Deinitialize() override;

  /** Whether to create the subsystem; true unless overridden. Called on the
   * class default object. */
  UFUNCTION(BlueprintNativeEvent, Category = "Rusteal|Subsystem")
  bool ReceiveShouldCreateSubsystem(UObject *Outer) const;

  /** The subsystem was created: the editor is starting. */
  UFUNCTION(BlueprintImplementableEvent, Category = "Rusteal|Subsystem")
  void ReceiveInitialize();

  /** The subsystem is about to be destroyed: the editor is closing. */
  UFUNCTION(BlueprintImplementableEvent, Category = "Rusteal|Subsystem")
  void ReceiveDeinitialize();
};
