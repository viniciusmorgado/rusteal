#pragma once

#include "CoreMinimal.h"
#include "GameFramework/Actor.h"
#include "GameFramework/Pawn.h"
#include "GameFramework/Character.h"
#include "GameFramework/PlayerController.h"
#include "GameFramework/GameModeBase.h"
#include "Components/ActorComponent.h"
#include "Components/SceneComponent.h"
#include "RustealShims.generated.h"

UCLASS(Abstract, Blueprintable)
class RUSTEAL_API ARustealActor : public AActor {
  GENERATED_BODY()

public:
  virtual void PreInitializeComponents() override;
  virtual void PostInitializeComponents() override;
  virtual void PostActorCreated() override;
  virtual void PostRegisterAllComponents() override;
  virtual void OnConstruction(const FTransform &Transform) override;

  UFUNCTION(BlueprintImplementableEvent, Category = "Rusteal|Actor")
  void ReceivePreInitializeComponents();

  UFUNCTION(BlueprintImplementableEvent, Category = "Rusteal|Actor")
  void ReceivePostInitializeComponents();

  UFUNCTION(BlueprintImplementableEvent, Category = "Rusteal|Actor")
  void ReceivePostActorCreated();

  UFUNCTION(BlueprintImplementableEvent, Category = "Rusteal|Actor")
  void ReceivePostRegisterAllComponents();

  UFUNCTION(BlueprintImplementableEvent, Category = "Rusteal|Actor")
  void ReceiveOnConstruction(const FTransform &Transform);
};

UCLASS(Abstract, Blueprintable)
class RUSTEAL_API ARustealPawn : public APawn {
  GENERATED_BODY()

public:
  virtual void PreInitializeComponents() override;
  virtual void PostInitializeComponents() override;
  virtual void PostActorCreated() override;
  virtual void PostRegisterAllComponents() override;
  virtual void OnConstruction(const FTransform &Transform) override;

  UFUNCTION(BlueprintImplementableEvent, Category = "Rusteal|Pawn")
  void ReceivePreInitializeComponents();

  UFUNCTION(BlueprintImplementableEvent, Category = "Rusteal|Pawn")
  void ReceivePostInitializeComponents();

  UFUNCTION(BlueprintImplementableEvent, Category = "Rusteal|Pawn")
  void ReceivePostActorCreated();

  UFUNCTION(BlueprintImplementableEvent, Category = "Rusteal|Pawn")
  void ReceivePostRegisterAllComponents();

  UFUNCTION(BlueprintImplementableEvent, Category = "Rusteal|Pawn")
  void ReceiveOnConstruction(const FTransform &Transform);
};

UCLASS(Abstract, Blueprintable)
class RUSTEAL_API ARustealCharacter : public ACharacter {
  GENERATED_BODY()

public:
  virtual void PreInitializeComponents() override;
  virtual void PostInitializeComponents() override;
  virtual void PostActorCreated() override;
  virtual void PostRegisterAllComponents() override;
  virtual void OnConstruction(const FTransform &Transform) override;

  UFUNCTION(BlueprintImplementableEvent, Category = "Rusteal|Character")
  void ReceivePreInitializeComponents();

  UFUNCTION(BlueprintImplementableEvent, Category = "Rusteal|Character")
  void ReceivePostInitializeComponents();

  UFUNCTION(BlueprintImplementableEvent, Category = "Rusteal|Character")
  void ReceivePostActorCreated();

  UFUNCTION(BlueprintImplementableEvent, Category = "Rusteal|Character")
  void ReceivePostRegisterAllComponents();

  UFUNCTION(BlueprintImplementableEvent, Category = "Rusteal|Character")
  void ReceiveOnConstruction(const FTransform &Transform);
};

UCLASS(Abstract, Blueprintable)
class RUSTEAL_API ARustealPlayerController : public APlayerController {
  GENERATED_BODY()

public:
  virtual void PreInitializeComponents() override;
  virtual void PostInitializeComponents() override;
  virtual void PostActorCreated() override;
  virtual void PostRegisterAllComponents() override;
  virtual void OnConstruction(const FTransform &Transform) override;

  UFUNCTION(BlueprintImplementableEvent, Category = "Rusteal|PlayerController")
  void ReceivePreInitializeComponents();

  UFUNCTION(BlueprintImplementableEvent, Category = "Rusteal|PlayerController")
  void ReceivePostInitializeComponents();

  UFUNCTION(BlueprintImplementableEvent, Category = "Rusteal|PlayerController")
  void ReceivePostActorCreated();

  UFUNCTION(BlueprintImplementableEvent, Category = "Rusteal|PlayerController")
  void ReceivePostRegisterAllComponents();

  UFUNCTION(BlueprintImplementableEvent, Category = "Rusteal|PlayerController")
  void ReceiveOnConstruction(const FTransform &Transform);
};

UCLASS(Abstract, Blueprintable)
class RUSTEAL_API ARustealGameModeBase : public AGameModeBase {
  GENERATED_BODY()

public:
  virtual void PreInitializeComponents() override;
  virtual void PostInitializeComponents() override;
  virtual void PostActorCreated() override;
  virtual void PostRegisterAllComponents() override;
  virtual void OnConstruction(const FTransform &Transform) override;

  UFUNCTION(BlueprintImplementableEvent, Category = "Rusteal|GameModeBase")
  void ReceivePreInitializeComponents();

  UFUNCTION(BlueprintImplementableEvent, Category = "Rusteal|GameModeBase")
  void ReceivePostInitializeComponents();

  UFUNCTION(BlueprintImplementableEvent, Category = "Rusteal|GameModeBase")
  void ReceivePostActorCreated();

  UFUNCTION(BlueprintImplementableEvent, Category = "Rusteal|GameModeBase")
  void ReceivePostRegisterAllComponents();

  UFUNCTION(BlueprintImplementableEvent, Category = "Rusteal|GameModeBase")
  void ReceiveOnConstruction(const FTransform &Transform);
};

UCLASS(Abstract, Blueprintable)
class RUSTEAL_API URustealActorComponent : public UActorComponent {
  GENERATED_BODY()

public:
  URustealActorComponent(const FObjectInitializer &ObjectInitializer);

  virtual void OnRegister() override;
  virtual void OnUnregister() override;
  virtual void InitializeComponent() override;
  virtual void UninitializeComponent() override;
  virtual void TickComponent(float DeltaTime, ELevelTick TickType, FActorComponentTickFunction *ThisTickFunction) override;

  UFUNCTION(BlueprintImplementableEvent, Category = "Rusteal|ActorComponent")
  void ReceiveOnRegister();

  UFUNCTION(BlueprintImplementableEvent, Category = "Rusteal|ActorComponent")
  void ReceiveOnUnregister();

  UFUNCTION(BlueprintImplementableEvent, Category = "Rusteal|ActorComponent")
  void ReceiveInitializeComponent();

  UFUNCTION(BlueprintImplementableEvent, Category = "Rusteal|ActorComponent")
  void ReceiveUninitializeComponent();

  UFUNCTION(BlueprintImplementableEvent, Category = "Rusteal|ActorComponent")
  void ReceiveTickComponent(float DeltaTime);
};

UCLASS(Abstract, Blueprintable)
class RUSTEAL_API URustealSceneComponent : public USceneComponent {
  GENERATED_BODY()

public:
  URustealSceneComponent(const FObjectInitializer &ObjectInitializer);

  virtual void OnRegister() override;
  virtual void OnUnregister() override;
  virtual void InitializeComponent() override;
  virtual void UninitializeComponent() override;
  virtual void TickComponent(float DeltaTime, ELevelTick TickType, FActorComponentTickFunction *ThisTickFunction) override;

  UFUNCTION(BlueprintImplementableEvent, Category = "Rusteal|SceneComponent")
  void ReceiveOnRegister();

  UFUNCTION(BlueprintImplementableEvent, Category = "Rusteal|SceneComponent")
  void ReceiveOnUnregister();

  UFUNCTION(BlueprintImplementableEvent, Category = "Rusteal|SceneComponent")
  void ReceiveInitializeComponent();

  UFUNCTION(BlueprintImplementableEvent, Category = "Rusteal|SceneComponent")
  void ReceiveUninitializeComponent();

  UFUNCTION(BlueprintImplementableEvent, Category = "Rusteal|SceneComponent")
  void ReceiveTickComponent(float DeltaTime);
};
