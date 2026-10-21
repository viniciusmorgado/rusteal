#include "RustealShims.h"

void ARustealActor::PreInitializeComponents() {
  Super::PreInitializeComponents();
  ReceivePreInitializeComponents();
}

void ARustealActor::PostInitializeComponents() {
  Super::PostInitializeComponents();
  ReceivePostInitializeComponents();
}

void ARustealActor::PostActorCreated() {
  Super::PostActorCreated();
  ReceivePostActorCreated();
}

void ARustealActor::PostRegisterAllComponents() {
  Super::PostRegisterAllComponents();
  ReceivePostRegisterAllComponents();
}

void ARustealActor::OnConstruction(const FTransform &Transform) {
  Super::OnConstruction(Transform);
  ReceiveOnConstruction(Transform);
}

void ARustealPawn::PreInitializeComponents() {
  Super::PreInitializeComponents();
  ReceivePreInitializeComponents();
}

void ARustealPawn::PostInitializeComponents() {
  Super::PostInitializeComponents();
  ReceivePostInitializeComponents();
}

void ARustealPawn::PostActorCreated() {
  Super::PostActorCreated();
  ReceivePostActorCreated();
}

void ARustealPawn::PostRegisterAllComponents() {
  Super::PostRegisterAllComponents();
  ReceivePostRegisterAllComponents();
}

void ARustealPawn::OnConstruction(const FTransform &Transform) {
  Super::OnConstruction(Transform);
  ReceiveOnConstruction(Transform);
}

void ARustealCharacter::PreInitializeComponents() {
  Super::PreInitializeComponents();
  ReceivePreInitializeComponents();
}

void ARustealCharacter::PostInitializeComponents() {
  Super::PostInitializeComponents();
  ReceivePostInitializeComponents();
}

void ARustealCharacter::PostActorCreated() {
  Super::PostActorCreated();
  ReceivePostActorCreated();
}

void ARustealCharacter::PostRegisterAllComponents() {
  Super::PostRegisterAllComponents();
  ReceivePostRegisterAllComponents();
}

void ARustealCharacter::OnConstruction(const FTransform &Transform) {
  Super::OnConstruction(Transform);
  ReceiveOnConstruction(Transform);
}

void ARustealPlayerController::PreInitializeComponents() {
  Super::PreInitializeComponents();
  ReceivePreInitializeComponents();
}

void ARustealPlayerController::PostInitializeComponents() {
  Super::PostInitializeComponents();
  ReceivePostInitializeComponents();
}

void ARustealPlayerController::PostActorCreated() {
  Super::PostActorCreated();
  ReceivePostActorCreated();
}

void ARustealPlayerController::PostRegisterAllComponents() {
  Super::PostRegisterAllComponents();
  ReceivePostRegisterAllComponents();
}

void ARustealPlayerController::OnConstruction(const FTransform &Transform) {
  Super::OnConstruction(Transform);
  ReceiveOnConstruction(Transform);
}

void ARustealGameModeBase::PreInitializeComponents() {
  Super::PreInitializeComponents();
  ReceivePreInitializeComponents();
}

void ARustealGameModeBase::PostInitializeComponents() {
  Super::PostInitializeComponents();
  ReceivePostInitializeComponents();
}

void ARustealGameModeBase::PostActorCreated() {
  Super::PostActorCreated();
  ReceivePostActorCreated();
}

void ARustealGameModeBase::PostRegisterAllComponents() {
  Super::PostRegisterAllComponents();
  ReceivePostRegisterAllComponents();
}

void ARustealGameModeBase::OnConstruction(const FTransform &Transform) {
  Super::OnConstruction(Transform);
  ReceiveOnConstruction(Transform);
}

URustealActorComponent::URustealActorComponent(const FObjectInitializer &ObjectInitializer)
    : Super(ObjectInitializer) {
  bWantsInitializeComponent = true;
  PrimaryComponentTick.bCanEverTick = GetClass()->IsFunctionImplementedInScript(FName(TEXT("ReceiveTickComponent")));
}

void URustealActorComponent::OnRegister() {
  Super::OnRegister();
  ReceiveOnRegister();
}

void URustealActorComponent::OnUnregister() {
  ReceiveOnUnregister();
  Super::OnUnregister();
}

void URustealActorComponent::InitializeComponent() {
  Super::InitializeComponent();
  ReceiveInitializeComponent();
}

void URustealActorComponent::UninitializeComponent() {
  ReceiveUninitializeComponent();
  Super::UninitializeComponent();
}

void URustealActorComponent::TickComponent(float DeltaTime, ELevelTick TickType, FActorComponentTickFunction *ThisTickFunction) {
  Super::TickComponent(DeltaTime, TickType, ThisTickFunction);
  ReceiveTickComponent(DeltaTime);
}

URustealSceneComponent::URustealSceneComponent(const FObjectInitializer &ObjectInitializer)
    : Super(ObjectInitializer) {
  bWantsInitializeComponent = true;
  PrimaryComponentTick.bCanEverTick = GetClass()->IsFunctionImplementedInScript(FName(TEXT("ReceiveTickComponent")));
}

void URustealSceneComponent::OnRegister() {
  Super::OnRegister();
  ReceiveOnRegister();
}

void URustealSceneComponent::OnUnregister() {
  ReceiveOnUnregister();
  Super::OnUnregister();
}

void URustealSceneComponent::InitializeComponent() {
  Super::InitializeComponent();
  ReceiveInitializeComponent();
}

void URustealSceneComponent::UninitializeComponent() {
  ReceiveUninitializeComponent();
  Super::UninitializeComponent();
}

void URustealSceneComponent::TickComponent(float DeltaTime, ELevelTick TickType, FActorComponentTickFunction *ThisTickFunction) {
  Super::TickComponent(DeltaTime, TickType, ThisTickFunction);
  ReceiveTickComponent(DeltaTime);
}
