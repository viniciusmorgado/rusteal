#pragma once

#include "CoreMinimal.h"
#include "Subsystems/EngineSubsystem.h"
#include "Subsystems/GameInstanceSubsystem.h"
#include "Subsystems/LocalPlayerSubsystem.h"
#include "Subsystems/WorldSubsystem.h"
#include "RustealSubsystems.generated.h"

// Parents for subsystems written in Rust.
//
// The engine creates a subsystem for every non-abstract class deriving from
// a subsystem base, Rust classes included, but announces its lifetime through
// C++ virtuals (Initialize, Deinitialize...), which a Rust class cannot
// override. These parents turn them into events a Rust class overrides with
// `#[ufunction(Override)]`.

/** A subsystem living as long as the game instance: one per game. */
UCLASS(Abstract, Blueprintable)
class RUSTEAL_API URustealGameInstanceSubsystem
    : public UGameInstanceSubsystem {
  GENERATED_BODY()

public:
  virtual bool ShouldCreateSubsystem(UObject *Outer) const override;
  virtual void Initialize(FSubsystemCollectionBase &Collection) override;
  virtual void Deinitialize() override;

  /** Whether to create the subsystem for this game instance; true unless
   * overridden. Called on the class default object. */
  UFUNCTION(BlueprintNativeEvent, Category = "Rusteal|Subsystem")
  bool ReceiveShouldCreateSubsystem(UObject *Outer) const;

  /** The subsystem was created. */
  UFUNCTION(BlueprintImplementableEvent, Category = "Rusteal|Subsystem")
  void ReceiveInitialize();

  /** The subsystem is about to be destroyed. */
  UFUNCTION(BlueprintImplementableEvent, Category = "Rusteal|Subsystem")
  void ReceiveDeinitialize();
};

/** A subsystem living as long as a world, optionally ticking with it. By
 * default it exists in game and PIE worlds only. */
UCLASS(Abstract, Blueprintable)
class RUSTEAL_API URustealWorldSubsystem : public UTickableWorldSubsystem {
  GENERATED_BODY()

public:
  /** Call ReceiveTick every frame the world ticks. */
  UPROPERTY(EditDefaultsOnly, BlueprintReadWrite, Category = "Rusteal|Subsystem")
  bool bWantsTick = false;

  /** Create the subsystem in editor worlds too, not only in game and PIE
   * ones. It does not tick there. */
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

  /** Whether to create the subsystem for this world; true unless overridden.
   * Called on the class default object. */
  UFUNCTION(BlueprintNativeEvent, Category = "Rusteal|Subsystem")
  bool ReceiveShouldCreateSubsystem(UObject *Outer) const;

  /** The subsystem was created, before the world's actors are. */
  UFUNCTION(BlueprintImplementableEvent, Category = "Rusteal|Subsystem")
  void ReceiveInitialize();

  /** Every subsystem of the world is initialized. */
  UFUNCTION(BlueprintImplementableEvent, Category = "Rusteal|Subsystem")
  void ReceivePostInitialize();

  /** The world's actors have begun play. */
  UFUNCTION(BlueprintImplementableEvent, Category = "Rusteal|Subsystem")
  void ReceiveWorldBeginPlay();

  /** A frame, when bWantsTick is set. */
  UFUNCTION(BlueprintImplementableEvent, Category = "Rusteal|Subsystem")
  void ReceiveTick(float DeltaSeconds);

  /** The subsystem is about to be destroyed. */
  UFUNCTION(BlueprintImplementableEvent, Category = "Rusteal|Subsystem")
  void ReceiveDeinitialize();

protected:
  virtual bool DoesSupportWorldType(
      const EWorldType::Type WorldType) const override;
};

/** A subsystem living as long as the engine: one per process, editor
 * included. */
UCLASS(Abstract, Blueprintable)
class RUSTEAL_API URustealEngineSubsystem : public UEngineSubsystem {
  GENERATED_BODY()

public:
  virtual bool ShouldCreateSubsystem(UObject *Outer) const override;
  virtual void Initialize(FSubsystemCollectionBase &Collection) override;
  virtual void Deinitialize() override;

  /** Whether to create the subsystem; true unless overridden. Called on the
   * class default object. */
  UFUNCTION(BlueprintNativeEvent, Category = "Rusteal|Subsystem")
  bool ReceiveShouldCreateSubsystem(UObject *Outer) const;

  /** The subsystem was created. */
  UFUNCTION(BlueprintImplementableEvent, Category = "Rusteal|Subsystem")
  void ReceiveInitialize();

  /** The subsystem is about to be destroyed. */
  UFUNCTION(BlueprintImplementableEvent, Category = "Rusteal|Subsystem")
  void ReceiveDeinitialize();
};

/** A subsystem living as long as a local player: one per player on this
 * machine. */
UCLASS(Abstract, Blueprintable)
class RUSTEAL_API URustealLocalPlayerSubsystem : public ULocalPlayerSubsystem {
  GENERATED_BODY()

public:
  virtual bool ShouldCreateSubsystem(UObject *Outer) const override;
  virtual void Initialize(FSubsystemCollectionBase &Collection) override;
  virtual void Deinitialize() override;
  virtual void
  PlayerControllerChanged(APlayerController *NewPlayerController) override;

  /** Whether to create the subsystem for this local player; true unless
   * overridden. Called on the class default object. */
  UFUNCTION(BlueprintNativeEvent, Category = "Rusteal|Subsystem")
  bool ReceiveShouldCreateSubsystem(UObject *Outer) const;

  /** The subsystem was created. */
  UFUNCTION(BlueprintImplementableEvent, Category = "Rusteal|Subsystem")
  void ReceiveInitialize();

  /** The local player's controller changed. */
  UFUNCTION(BlueprintImplementableEvent, Category = "Rusteal|Subsystem")
  void ReceivePlayerControllerChanged(APlayerController *NewPlayerController);

  /** The subsystem is about to be destroyed. */
  UFUNCTION(BlueprintImplementableEvent, Category = "Rusteal|Subsystem")
  void ReceiveDeinitialize();
};
