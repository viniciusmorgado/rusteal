// Copyright 2026 Vinicius Donatto Morgado.

#pragma once

#include "Containers/Ticker.h"
#include "CoreMinimal.h"

class FRustealLauncherProcess {
public:
  DECLARE_DELEGATE_OneParam(FOnLine, const FString &);
  DECLARE_DELEGATE_OneParam(FOnExit, int32);

  ~FRustealLauncherProcess();

  bool Launch(const FString &Executable, const FString &Arguments,
              const FString &WorkingDirectory);
  void Cancel();
  bool IsRunning() const { return Handle.IsValid(); }

  FOnLine OnLine;
  FOnExit OnExit;

private:
  bool Tick(float DeltaTime);
  void Drain();
  void Release();

  FProcHandle Handle;
  void *ReadPipe = nullptr;
  void *WritePipe = nullptr;
  FString Pending;
  FTSTicker::FDelegateHandle TickerHandle;
};
