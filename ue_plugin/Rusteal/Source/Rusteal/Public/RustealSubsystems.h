#pragma once

#include "CoreMinimal.h"
#include "Subsystems/EngineSubsystem.h"
#include "Subsystems/GameInstanceSubsystem.h"
#include "Subsystems/LocalPlayerSubsystem.h"
#include "Subsystems/WorldSubsystem.h"
#include "RustealSubsystems.generated.h"

UCLASS(Abstract, Blueprintable)
class RUSTEAL_API URustealGameInstanceSubsystem
    : public UGameInstanceSubsystem {
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

UCLASS(Abstract, Blueprintable)
class RUSTEAL_API URustealWorldSubsystem : public UTickableWorldSubsystem {
  GENERATED_BODY()

public:
  UPROPERTY(EditDefaultsOnly, BlueprintReadWrite,
            Category = "Rusteal|Subsystem")
  bool bWantsTick = false;

  UPROPERTY(EditDefaultsOnly, BlueprintReadOnly, Category = "Rusteal|Subsystem")
  bool bCreateInEditorWorlds = false;

  virtual bool ShouldCreateSubsystem(UObject *Outer) const override;
  virtual void Initialize(FSubsystemCollectionBase &Collection) override;
  virtual void PostInitialize() override;
  virtual void OnWorldBeginPlay(UWorld &InWorld) override;
  virtual void Deinitialize() override;
  virtual void Tick(float DeltaTime) override;
  virtual bool IsTickable() const override;
  virtual TStatId GetStatId() const override;

  UFUNCTION(BlueprintNativeEvent, Category = "Rusteal|Subsystem")
  bool ReceiveShouldCreateSubsystem(UObject *Outer) const;

  UFUNCTION(BlueprintImplementableEvent, Category = "Rusteal|Subsystem")
  void ReceiveInitialize();

  UFUNCTION(BlueprintImplementableEvent, Category = "Rusteal|Subsystem")
  void ReceivePostInitialize();

  UFUNCTION(BlueprintImplementableEvent, Category = "Rusteal|Subsystem")
  void ReceiveWorldBeginPlay();

  UFUNCTION(BlueprintImplementableEvent, Category = "Rusteal|Subsystem")
  void ReceiveTick(float DeltaSeconds);

  UFUNCTION(BlueprintImplementableEvent, Category = "Rusteal|Subsystem")
  void ReceiveDeinitialize();

protected:
  virtual bool
  DoesSupportWorldType(const EWorldType::Type WorldType) const override;
};

UCLASS(Abstract, Blueprintable)
class RUSTEAL_API URustealEngineSubsystem : public UEngineSubsystem {
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

UCLASS(Abstract, Blueprintable)
class RUSTEAL_API URustealLocalPlayerSubsystem : public ULocalPlayerSubsystem {
  GENERATED_BODY()

public:
  virtual bool ShouldCreateSubsystem(UObject *Outer) const override;
  virtual void Initialize(FSubsystemCollectionBase &Collection) override;
  virtual void Deinitialize() override;
  virtual void
  PlayerControllerChanged(APlayerController *NewPlayerController) override;

  UFUNCTION(BlueprintNativeEvent, Category = "Rusteal|Subsystem")
  bool ReceiveShouldCreateSubsystem(UObject *Outer) const;

  UFUNCTION(BlueprintImplementableEvent, Category = "Rusteal|Subsystem")
  void ReceiveInitialize();

  UFUNCTION(BlueprintImplementableEvent, Category = "Rusteal|Subsystem")
  void ReceivePlayerControllerChanged(APlayerController *NewPlayerController);

  UFUNCTION(BlueprintImplementableEvent, Category = "Rusteal|Subsystem")
  void ReceiveDeinitialize();
};
