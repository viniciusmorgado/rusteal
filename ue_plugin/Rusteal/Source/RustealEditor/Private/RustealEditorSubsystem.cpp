#include "RustealEditorSubsystem.h"

bool URustealEditorSubsystem::ShouldCreateSubsystem(UObject *Outer) const {
  return Super::ShouldCreateSubsystem(Outer) &&
         ReceiveShouldCreateSubsystem(Outer);
}

bool URustealEditorSubsystem::ReceiveShouldCreateSubsystem_Implementation(
    UObject *Outer) const {
  return true;
}

void URustealEditorSubsystem::Initialize(FSubsystemCollectionBase &Collection) {
  Super::Initialize(Collection);
  ReceiveInitialize();
}

void URustealEditorSubsystem::Deinitialize() {
  ReceiveDeinitialize();
  Super::Deinitialize();
}
