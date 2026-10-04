#include "RustealAutoReload.h"

#include "DirectoryWatcherModule.h"
#include "Framework/Application/SlateApplication.h"
#include "Framework/Notifications/NotificationManager.h"
#include "HAL/FileManager.h"
#include "HAL/IConsoleManager.h"
#include "IDirectoryWatcher.h"
#include "Misc/Paths.h"
#include "Modules/ModuleManager.h"
#include "RustealLibraries.h"
#include "Widgets/Notifications/SNotificationList.h"

#define LOCTEXT_NAMESPACE "RustealAutoReload"

DEFINE_LOG_CATEGORY_STATIC(LogRustealAutoReload, Log, All);

static TAutoConsoleVariable<bool> CVarAutoReload(
    TEXT("Rusteal.AutoReload"), true,
    TEXT("Hot reload a Rust library as soon as `rusteal build` deploys it "
         "again (1, the default), or only with Rusteal.Reload (0)."));

// How long the deployed file must stay unchanged before it is loaded: a copy
// that is not a rename writes it in several steps.
static constexpr double SettleSeconds = 0.5;

static FString FullPath(const FString &Path) {
  FString Full = FPaths::ConvertRelativePathToFull(Path);
  FPaths::NormalizeFilename(Full);
  return Full;
}

static IDirectoryWatcher *DirectoryWatcher() {
  FDirectoryWatcherModule &Module =
      FModuleManager::LoadModuleChecked<FDirectoryWatcherModule>(
          TEXT("DirectoryWatcher"));
  return Module.Get();
}

static void Notify(const FText &Text, bool bSucceeded) {
  if (!FSlateApplication::IsInitialized()) {
    return;
  }
  FNotificationInfo Info(Text);
  Info.ExpireDuration = bSucceeded ? 3.0f : 6.0f;
  Info.bFireAndForget = true;
  if (TSharedPtr<SNotificationItem> Item =
          FSlateNotificationManager::Get().AddNotification(Info)) {
    Item->SetCompletionState(bSucceeded ? SNotificationItem::CS_Success
                                        : SNotificationItem::CS_Fail);
  }
}

void FRustealAutoReload::Start() {
  RustealForEachLibrary([this](FName Name, const FString &DeployedPath) {
    Watch(Name, DeployedPath);
  });
  RegisteredHandle = RustealOnLibraryRegistered().AddLambda(
      [this](FName Name, const FString &DeployedPath) {
        Watch(Name, DeployedPath);
      });
  TickerHandle = FTSTicker::GetCoreTicker().AddTicker(
      FTickerDelegate::CreateRaw(this, &FRustealAutoReload::Tick));
}

void FRustealAutoReload::Stop() {
  FTSTicker::GetCoreTicker().RemoveTicker(TickerHandle);
  RustealOnLibraryRegistered().Remove(RegisteredHandle);
  if (FModuleManager::Get().IsModuleLoaded(TEXT("DirectoryWatcher"))) {
    IDirectoryWatcher *Watcher = DirectoryWatcher();
    for (const FWatchedLibrary &Library : Libraries) {
      if (Watcher && Library.WatchHandle.IsValid()) {
        Watcher->UnregisterDirectoryChangedCallback_Handle(Library.Directory,
                                                           Library.WatchHandle);
      }
    }
  }
  Libraries.Empty();
}

void FRustealAutoReload::Watch(FName Name, const FString &DeployedPath) {
  if (DeployedPath.IsEmpty() ||
      Libraries.ContainsByPredicate(
          [Name](const FWatchedLibrary &Library) { return Library.Name == Name; })) {
    return;
  }
  IDirectoryWatcher *Watcher = DirectoryWatcher();
  if (!Watcher) {
    return;
  }
  FWatchedLibrary Library;
  Library.Name = Name;
  Library.File = FullPath(DeployedPath);
  Library.Directory = FPaths::GetPath(Library.File);
  // Before the first deploy there is nothing to watch yet; the deploy step
  // creates this directory anyway.
  IFileManager::Get().MakeDirectory(*Library.Directory, /*Tree=*/true);
  Watcher->RegisterDirectoryChangedCallback_Handle(
      Library.Directory,
      IDirectoryWatcher::FDirectoryChanged::CreateRaw(
          this, &FRustealAutoReload::OnDirectoryChanged, Name),
      Library.WatchHandle);
  UE_LOG(LogRustealAutoReload, Display, TEXT("Watching %s (%s)"), *Library.File,
         *Name.ToString());
  Libraries.Add(MoveTemp(Library));
}

void FRustealAutoReload::OnDirectoryChanged(
    const TArray<FFileChangeData> &Changes, FName Name) {
  FWatchedLibrary *Library = Libraries.FindByPredicate(
      [Name](const FWatchedLibrary &Watched) { return Watched.Name == Name; });
  if (!Library || !CVarAutoReload.GetValueOnGameThread()) {
    return;
  }
  // The deployed file only: the numbered copies a reload loads sit next to
  // it, and so does the temporary file the deploy step renames over it.
  for (const FFileChangeData &Change : Changes) {
    if (Change.Action != FFileChangeData::FCA_Removed &&
        FPaths::IsSamePath(FullPath(Change.Filename), Library->File)) {
      Library->ChangedAt = FPlatformTime::Seconds();
    }
  }
}

bool FRustealAutoReload::Tick(float DeltaSeconds) {
  const double Now = FPlatformTime::Seconds();
  for (FWatchedLibrary &Library : Libraries) {
    if (Library.ChangedAt == 0.0 || Now - Library.ChangedAt < SettleSeconds ||
        !FPaths::FileExists(Library.File)) {
      continue;
    }
    Library.ChangedAt = 0.0;
    UE_LOG(LogRustealAutoReload, Display, TEXT("%s was deployed again"),
           *Library.File);
    const bool bReloaded = RustealReloadLibrary(Library.Name);
    Notify(FText::Format(bReloaded
                             ? LOCTEXT("Reloaded", "Rusteal: {0} reloaded")
                             : LOCTEXT("ReloadFailed",
                                       "Rusteal: {0} failed to reload, see the "
                                       "Output Log"),
                         FText::FromName(Library.Name)),
           bReloaded);
  }
  return true;
}

#undef LOCTEXT_NAMESPACE
