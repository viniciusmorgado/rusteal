#pragma once

#include "CoreMinimal.h"
#include "Containers/Ticker.h"

// Hot reloads a Rust library as soon as `rusteal build` deploys it again: the
// directory each library is deployed to is watched, and a change to the
// deployed file reloads that library once the writes settle. On by default;
// Rusteal.AutoReload 0 turns it off.
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
    /** When the deployed file last changed; 0 when no reload is pending. */
    double ChangedAt = 0.0;
  };

  void Watch(FName Name, const FString &DeployedPath);
  // The library's name comes after the changes, as a delegate's payload.
  void OnDirectoryChanged(const TArray<struct FFileChangeData> &Changes, FName Name);
  bool Tick(float DeltaSeconds);

  TArray<FWatchedLibrary> Libraries;
  FDelegateHandle RegisteredHandle;
  FTSTicker::FDelegateHandle TickerHandle;
};
