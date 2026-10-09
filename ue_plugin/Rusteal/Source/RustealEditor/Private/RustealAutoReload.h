#pragma once

#include "CoreMinimal.h"
#include "Containers/Ticker.h"

class FRustealAutoReload {
public:
  void Start();
  void Stop();

private:
  struct FWatchedLibrary {
    FName Name;
    FString File;
    FString Directory;
    FDelegateHandle WatchHandle;
    double ChangedAt = 0.0;
  };

  void Watch(FName Name, const FString &DeployedPath);
  void OnDirectoryChanged(const TArray<struct FFileChangeData> &Changes,
                          FName Name);
  bool Tick(float DeltaSeconds);

  TArray<FWatchedLibrary> Libraries;
  FDelegateHandle RegisteredHandle;
  FTSTicker::FDelegateHandle TickerHandle;
};
