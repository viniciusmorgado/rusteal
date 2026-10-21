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

void URustealUserWidget::NativeOnInitialized() {
  Super::NativeOnInitialized();
  ReceiveOnInitialized();
}

void URustealUserWidget::NativePreConstruct() {
  Super::NativePreConstruct();
  ReceivePreConstruct();
}

void URustealUserWidget::NativeConstruct() {
  Super::NativeConstruct();
  ReceiveConstruct();
}

void URustealUserWidget::NativeDestruct() {
  ReceiveDestruct();
  Super::NativeDestruct();
}

void URustealUserWidget::NativeTick(const FGeometry &MyGeometry, float InDeltaTime) {
  Super::NativeTick(MyGeometry, InDeltaTime);
  ReceiveTick(MyGeometry, InDeltaTime);
}

void URustealAnimInstance::NativeInitializeAnimation() {
  Super::NativeInitializeAnimation();
  ReceiveInitializeAnimation();
}

void URustealAnimInstance::NativeBeginPlay() {
  Super::NativeBeginPlay();
  ReceiveBeginPlay();
}

void URustealAnimInstance::NativeUpdateAnimation(float DeltaSeconds) {
  Super::NativeUpdateAnimation(DeltaSeconds);
  ReceiveUpdateAnimation(DeltaSeconds);
}

void URustealAnimInstance::NativeThreadSafeUpdateAnimation(float DeltaSeconds) {
  Super::NativeThreadSafeUpdateAnimation(DeltaSeconds);
  ReceiveThreadSafeUpdateAnimation(DeltaSeconds);
}

void URustealAnimInstance::NativePostEvaluateAnimation() {
  Super::NativePostEvaluateAnimation();
  ReceivePostEvaluateAnimation();
}

void URustealAnimInstance::NativeUninitializeAnimation() {
  ReceiveUninitializeAnimation();
  Super::NativeUninitializeAnimation();
}

void URustealCharacterMovementComponent::PhysCustom(float DeltaTime, int32 Iterations) {
  Super::PhysCustom(DeltaTime, Iterations);
  ReceivePhysCustom(DeltaTime, Iterations);
}

float URustealCharacterMovementComponent::GetMaxSpeed() const {
  return ReceiveGetMaxSpeed();
}

float URustealCharacterMovementComponent::ReceiveGetMaxSpeed_Implementation() const {
  return Super::GetMaxSpeed();
}

float URustealCharacterMovementComponent::GetMaxAcceleration() const {
  return ReceiveGetMaxAcceleration();
}

float URustealCharacterMovementComponent::ReceiveGetMaxAcceleration_Implementation() const {
  return Super::GetMaxAcceleration();
}

void URustealCharacterMovementComponent::OnMovementModeChanged(EMovementMode PreviousMovementMode, uint8 PreviousCustomMode) {
  Super::OnMovementModeChanged(PreviousMovementMode, PreviousCustomMode);
  ReceiveOnMovementModeChanged(PreviousMovementMode, PreviousCustomMode);
}

int32 URustealCommandlet::Main(const FString &Params) {
  return ReceiveMain(Params);
}

int32 URustealCommandlet::ReceiveMain_Implementation(const FString &Params) {
  return Super::Main(Params);
}
