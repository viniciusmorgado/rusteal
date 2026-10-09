#include "RustealSubsystems.h"
#include "Engine/World.h"

bool URustealGameInstanceSubsystem::ShouldCreateSubsystem(
    UObject *Outer) const {
  return Super::ShouldCreateSubsystem(Outer) &&
         ReceiveShouldCreateSubsystem(Outer);
}

bool URustealGameInstanceSubsystem::ReceiveShouldCreateSubsystem_Implementation(
    UObject *Outer) const {
  return true;
}

void URustealGameInstanceSubsystem::Initialize(
    FSubsystemCollectionBase &Collection) {
  Super::Initialize(Collection);
  ReceiveInitialize();
}

void URustealGameInstanceSubsystem::Deinitialize() {
  ReceiveDeinitialize();
  Super::Deinitialize();
}

bool URustealWorldSubsystem::ShouldCreateSubsystem(UObject *Outer) const {
  return Super::ShouldCreateSubsystem(Outer) &&
         ReceiveShouldCreateSubsystem(Outer);
}

bool URustealWorldSubsystem::ReceiveShouldCreateSubsystem_Implementation(
    UObject *Outer) const {
  return true;
}

bool URustealWorldSubsystem::DoesSupportWorldType(
    const EWorldType::Type WorldType) const {
  if (WorldType == EWorldType::Game || WorldType == EWorldType::PIE) {
    return true;
  }

  return bCreateInEditorWorlds && WorldType == EWorldType::Editor;
}

void URustealWorldSubsystem::Initialize(FSubsystemCollectionBase &Collection) {
  Super::Initialize(Collection);
  ReceiveInitialize();
}

void URustealWorldSubsystem::PostInitialize() {
  Super::PostInitialize();
  ReceivePostInitialize();
}

void URustealWorldSubsystem::OnWorldBeginPlay(UWorld &InWorld) {
  Super::OnWorldBeginPlay(InWorld);
  ReceiveWorldBeginPlay();
}

void URustealWorldSubsystem::Deinitialize() {
  ReceiveDeinitialize();
  Super::Deinitialize();
}

void URustealWorldSubsystem::Tick(float DeltaTime) {
  Super::Tick(DeltaTime);
  ReceiveTick(DeltaTime);
}

bool URustealWorldSubsystem::IsTickable() const { return bWantsTick; }

TStatId URustealWorldSubsystem::GetStatId() const {
  RETURN_QUICK_DECLARE_CYCLE_STAT(URustealWorldSubsystem, STATGROUP_Tickables);
}

bool URustealEngineSubsystem::ShouldCreateSubsystem(UObject *Outer) const {
  return Super::ShouldCreateSubsystem(Outer) &&
         ReceiveShouldCreateSubsystem(Outer);
}

bool URustealEngineSubsystem::ReceiveShouldCreateSubsystem_Implementation(
    UObject *Outer) const {
  return true;
}

void URustealEngineSubsystem::Initialize(FSubsystemCollectionBase &Collection) {
  Super::Initialize(Collection);
  ReceiveInitialize();
}

void URustealEngineSubsystem::Deinitialize() {
  ReceiveDeinitialize();
  Super::Deinitialize();
}

bool URustealLocalPlayerSubsystem::ShouldCreateSubsystem(UObject *Outer) const {
  return Super::ShouldCreateSubsystem(Outer) &&
         ReceiveShouldCreateSubsystem(Outer);
}

bool URustealLocalPlayerSubsystem::ReceiveShouldCreateSubsystem_Implementation(
    UObject *Outer) const {
  return true;
}

void URustealLocalPlayerSubsystem::Initialize(
    FSubsystemCollectionBase &Collection) {
  Super::Initialize(Collection);
  ReceiveInitialize();
}

void URustealLocalPlayerSubsystem::Deinitialize() {
  ReceiveDeinitialize();
  Super::Deinitialize();
}

void URustealLocalPlayerSubsystem::PlayerControllerChanged(
    APlayerController *NewPlayerController) {
  Super::PlayerControllerChanged(NewPlayerController);
  ReceivePlayerControllerChanged(NewPlayerController);
}
