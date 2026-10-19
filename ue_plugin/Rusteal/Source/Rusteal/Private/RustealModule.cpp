#include "RustealModule.h"
#include "RustealErrors.h"
#include "HAL/FileManager.h"
#include "HAL/IConsoleManager.h"
#include "HAL/PlatformProcess.h"
#include "Interfaces/IPluginManager.h"
#include "Misc/App.h"
#include "Misc/Paths.h"
#include "Containers/Ticker.h"
#include "Modules/ModuleManager.h"
#include "RustealApiTable.h"
#include "RustealLibraries.h"
#include "RustealLibrary.h"
#include "URustealReifiedClass.h"

#if WITH_EDITOR && PLATFORM_LINUX
#include <unistd.h>
#include "RustealStrings.h"
#endif

#ifndef RUSTEAL_STATIC
#define RUSTEAL_STATIC 0
#endif

#ifndef RUSTEAL_HOSTS_GAME_LIBRARY
#define RUSTEAL_HOSTS_GAME_LIBRARY 1
#endif

DEFINE_LOG_CATEGORY(LogRusteal);

extern FRustealCoreApi GCoreApi;
extern FRustealReflectionApi GReflectionApi;
extern FRustealPropertyApi GPropertyApi;
extern FRustealContainerApi GContainerApi;
extern FRustealDelegateApi GDelegateApi;
extern FRustealLifecycleApi GLifecycleApi;
extern FRustealReifyApi GReifyApi;
extern FRustealWorldApi GWorldApi;
extern FRustealWidgetApi GWidgetApi;
extern FRustealInputApi GInputApi;
extern FRustealConsoleApi GConsoleApi;
extern FRustealPlatformApi GPlatformApi;

extern void RustealReifyRegisterDeleteListener();
extern void RustealReifyUnregisterDeleteListener();
extern void RustealReifyForEachReifiedInstance(
    TFunctionRef<void(UObject *, URustealReifiedClass *)> Callback);
extern void RustealReifyReinstanceReplacedClasses();
extern void RustealReifyRefreshChangedInterfaces();

extern void RustealRegisterComponentListResync();
extern void RustealUnregisterComponentListResync();

extern void RustealConsoleForgetLibrary(FRustealLibrary *Library);

extern void RustealPinnedForgetLibrary(FRustealLibrary *Library);
extern void RustealPinnedShutdown();

extern void RustealFillFuncTable();
extern void **RustealGetFuncTable();
extern uint32_t RustealGetFuncCount();
extern uint64_t RustealGetFuncFingerprint();

#define LOCTEXT_NAMESPACE "FRustealModule"

static void RustealLogImpl(uint8 Level, const uint8 *Msg, uint32 MsgLen) {
  const FString MsgStr = RustealUtf8ToFString(Msg, MsgLen);

  switch (Level) {
  case 0:
    UE_LOG(LogRusteal, Display, TEXT("%s"), *MsgStr);
    break;
  case 1:
    UE_LOG(LogRusteal, Warning, TEXT("%s"), *MsgStr);
    break;
  default:
    UE_LOG(LogRusteal, Error, TEXT("%s"), *MsgStr);
    break;
  }
}

static FRustealLoggingApi GLoggingApi = {&RustealLogImpl, &RustealReport};

static FRustealApiTable GApiTable;
static bool GApiTableFilled = false;

static uint32 RustealPluginVersion() {
#ifdef RUSTEAL_PLUGIN_VERSION
  return RUSTEAL_PLUGIN_VERSION;
#else
  TSharedPtr<IPlugin> Plugin =
      IPluginManager::Get().FindPlugin(TEXT("Rusteal"));

  return Plugin.IsValid() ? static_cast<uint32>(Plugin->GetDescriptor().Version)
                          : 0;
#endif
}

static FString RustealVersionString(uint32 Version) {
  return FString::Printf(TEXT("%u.%u.%u"), Version / 1000000,
                         Version / 1000 % 1000, Version % 1000);
}

static void FillApiTable() {
  if (GApiTableFilled) {
    return;
  }

  FMemory::Memzero(GApiTable);
  GApiTable.version = RustealPluginVersion();

  GApiTable.logging = &GLoggingApi;
  GApiTable.core = &GCoreApi;
  GApiTable.property = &GPropertyApi;
  GApiTable.reflection = &GReflectionApi;
  GApiTable.container = &GContainerApi;
  GApiTable.delegate = &GDelegateApi;
  GApiTable.lifecycle = &GLifecycleApi;
  GApiTable.reify = &GReifyApi;
  GApiTable.world = &GWorldApi;
  GApiTable.widget = &GWidgetApi;
  GApiTable.input = &GInputApi;
  GApiTable.console = &GConsoleApi;
  GApiTable.platform = &GPlatformApi;
  GApiTableFilled = true;
}

static TArray<TUniquePtr<FRustealLibrary>> GLibraries;
static FRustealLibrary *GCurrentLibrary = nullptr;

static FTSTicker::FDelegateHandle GTickerHandle;

static bool TickLibraries(float DeltaSeconds) {
  for (int32 Index = 0; Index < GLibraries.Num(); ++Index) {
    RustealCallLibrary(GLibraries[Index].Get(),
                       [DeltaSeconds](const FRustealRustCallbacks &Cb) {
                         Cb.on_tick(DeltaSeconds);
                       });
  }

  return true;
}

FRustealLibrary *RustealCurrentLibrary() { return GCurrentLibrary; }

UPackage *RustealCurrentPackage() {
  const FString Path =
      GCurrentLibrary && !GCurrentLibrary->PackagePath.IsEmpty()
          ? GCurrentLibrary->PackagePath
          : FString(TEXT("/Script/Rusteal"));

  UPackage *Package = FindPackage(nullptr, *Path);

  if (!Package) {
    Package = CreatePackage(*Path);
    Package->SetPackageFlags(PKG_CompiledIn);
    Package->AddToRoot();
  }

  return Package;
}

FRustealLibraryScope::FRustealLibraryScope(FRustealLibrary *Library)
    : Previous(GCurrentLibrary) {
  GCurrentLibrary = Library;
}

FRustealLibraryScope::~FRustealLibraryScope() { GCurrentLibrary = Previous; }

static FRustealLibrary *FindLibrary(FName Name) {
  for (const TUniquePtr<FRustealLibrary> &Library : GLibraries) {
    if (Library->Name == Name) {
      return Library.Get();
    }
  }

  return nullptr;
}

// The name `rusteal build` deploys: lib<stem> everywhere but Windows. Not
// FPlatformProcess::GetModulePrefix(), which is empty on Mac.
FString RustealLibraryFileName(const FString &Stem) {
  return FString::Printf(TEXT("%s%s.%s"),
                         PLATFORM_WINDOWS ? TEXT("") : TEXT("lib"), *Stem,
                         FPlatformProcess::GetModuleExtension());
}

#if !RUSTEAL_STATIC && WITH_EDITOR
static FString HotCopyPath(const FRustealLibrary &Library) {
  return FPaths::Combine(
      FPaths::GetPath(Library.SourcePath),
      FString::Printf(
          TEXT("%s_hot_%d.%s"), *FPaths::GetBaseFilename(Library.SourcePath),
          Library.ReloadCount, FPlatformProcess::GetModuleExtension()));
}

static void DeleteStaleLoadCopies(const FRustealLibrary &Library) {
  const FString Directory = FPaths::GetPath(Library.SourcePath);

  const FString Pattern = FString::Printf(
      TEXT("%s_hot_*.%s"), *FPaths::GetBaseFilename(Library.SourcePath),
      FPlatformProcess::GetModuleExtension());

  TArray<FString> Stale;

  IFileManager::Get().FindFiles(Stale, *FPaths::Combine(Directory, Pattern),
                                true, false);

  for (const FString &Name : Stale) {
    IFileManager::Get().Delete(*FPaths::Combine(Directory, Name), false, true,
                               true);
  }
}
#endif

static void FreeHandle(FRustealLibrary &Library) {
  FPlatformProcess::FreeDllHandle(Library.Handle);
  Library.Handle = nullptr;
}

#if !RUSTEAL_STATIC
static bool LoadLibraryFile(FRustealLibrary &Library, const FString &LoadPath) {
  Library.Handle = FPlatformProcess::GetDllHandle(*LoadPath);

  if (!Library.Handle) {
    UE_LOG(LogRusteal, Error, TEXT("[Rusteal] %s: failed to load %s"),
           *Library.Name.ToString(), *LoadPath);

    return false;
  }

  Library.LoadedPath = LoadPath;

  auto VersionFn = reinterpret_cast<FRustealVersionFn>(
      FPlatformProcess::GetDllExport(Library.Handle, TEXT("rusteal_version")));

  const uint32 LibraryVersion = VersionFn ? VersionFn() : 0;

  if (LibraryVersion != GApiTable.version) {
    UE_LOG(LogRusteal, Error,
           TEXT("[Rusteal] %s: version mismatch: plugin %s, library %s. "
                "Rebuild the project with `rusteal build --all` (run `rusteal "
                "upgrade` first if the CLI is newer than the project)."),
           *Library.Name.ToString(), *RustealVersionString(GApiTable.version),
           VersionFn ? *RustealVersionString(LibraryVersion)
                     : TEXT("without a version (0.2.1 or older)"));

    FreeHandle(Library);
    return false;
  }

  auto InitFn = reinterpret_cast<FRustealInitFn>(
      FPlatformProcess::GetDllExport(Library.Handle, TEXT("rusteal_init")));

  if (!InitFn) {
    UE_LOG(LogRusteal, Error, TEXT("[Rusteal] %s: rusteal_init not found"),
           *Library.Name.ToString());

    FreeHandle(Library);
    return false;
  }

  auto CallbacksFn =
      reinterpret_cast<FRustealCallbacksFn>(FPlatformProcess::GetDllExport(
          Library.Handle, TEXT("rusteal_callbacks")));

  Library.Callbacks = CallbacksFn ? CallbacksFn() : nullptr;
  const FRustealRustCallbacks *Callbacks = nullptr;

  {
    FRustealLibraryScope Scope(&Library);
    Callbacks = InitFn(&Library.Table);
  }

  Library.Callbacks = nullptr;

  if (!Callbacks) {
    UE_LOG(LogRusteal, Error, TEXT("[Rusteal] %s: rusteal_init returned null"),
           *Library.Name.ToString());

    FreeHandle(Library);
    return false;
  }

  Library.Callbacks = Callbacks;
  return true;
}
#endif

#if RUSTEAL_STATIC
extern "C" const FRustealStaticModule *rusteal_static_modules(uint32 *Count);

static const FRustealModuleExports *FindStaticModule(FName Name) {
  uint32 Count = 0;
  const FRustealStaticModule *Modules = rusteal_static_modules(&Count);

  for (uint32 Index = 0; Index < Count; ++Index) {
    const FUTF8ToTCHAR Converted(
        reinterpret_cast<const ANSICHAR *>(Modules[Index].name),
        Modules[Index].name_len);

    if (FName(FString(Converted.Length(), Converted.Get())) == Name) {
      return Modules[Index].exports;
    }
  }

  return nullptr;
}

static bool LoadStaticLibrary(FRustealLibrary &Library) {
  const FRustealModuleExports *Exports = FindStaticModule(Library.Name);

  if (!Exports) {
    UE_LOG(LogRusteal, Error,
           TEXT("[Rusteal] %s: no Rust module of that name is linked into "
                "the game. Package it again with `rusteal package --static`."),
           *Library.Name.ToString());

    return false;
  }

  if (Exports->version != GApiTable.version) {
    UE_LOG(LogRusteal, Error,
           TEXT("[Rusteal] %s: version mismatch: plugin %s, Rust module %s."),
           *Library.Name.ToString(), *RustealVersionString(GApiTable.version),
           *RustealVersionString(Exports->version));

    return false;
  }

  Library.Exports = Exports;
  Library.Callbacks = Exports->callbacks();
  const FRustealRustCallbacks *Callbacks = nullptr;

  {
    FRustealLibraryScope Scope(&Library);
    Callbacks = Exports->init(&Library.Table);
  }

  Library.Callbacks = Callbacks;

  if (!Callbacks) {
    UE_LOG(LogRusteal, Error, TEXT("[Rusteal] %s: rusteal_init returned null"),
           *Library.Name.ToString());

    Library.Exports = nullptr;
    return false;
  }

  UE_LOG(LogRusteal, Display,
         TEXT("[Rusteal] %s: Rust module started, linked into the game."),
         *Library.Name.ToString());

  return true;
}
#endif

#if !RUSTEAL_STATIC && WITH_EDITOR
// Not a hard link on Mac: dlclose never unloads a Rust library there, and
// dlopen returns the library still loaded from the same file (inode).
static bool MakeLoadCopy(const FString &Source, const FString &Copy) {
#if PLATFORM_LINUX
  IFileManager::Get().Delete(*Copy, false, true, true);

  if (link(TCHAR_TO_UTF8(*Source), TCHAR_TO_UTF8(*Copy)) == 0) {
    return true;
  }

#endif
  return IFileManager::Get().Copy(*Copy, *Source) == COPY_OK;
}
#endif

static bool LoadLibrary(FRustealLibrary &Library) {
#if RUSTEAL_STATIC
  return LoadStaticLibrary(Library);
#else

  if (!FPaths::FileExists(Library.SourcePath)) {
    UE_LOG(LogRusteal, Warning,
           TEXT("[Rusteal] %s: no Rust library at %s, its Rust side will not "
                "be loaded."),
           *Library.Name.ToString(), *Library.SourcePath);

    return false;
  }

  Library.ReloadCount++;
  bool bLoaded;

#if WITH_EDITOR
  const FString CopyPath = HotCopyPath(Library);

  if (!MakeLoadCopy(Library.SourcePath, CopyPath)) {
    UE_LOG(LogRusteal, Warning,
           TEXT("[Rusteal] %s: could not copy %s to %s, loading it in place."),
           *Library.Name.ToString(), *Library.SourcePath, *CopyPath);

    bLoaded = LoadLibraryFile(Library, Library.SourcePath);
  } else {
    bLoaded = LoadLibraryFile(Library, CopyPath);
  }
#else
  // A packaged game never reloads: it loads the library in place, and writes
  // nothing next to it (on Mac, into its signed bundle).
  bLoaded = LoadLibraryFile(Library, Library.SourcePath);
#endif

  if (bLoaded) {
    UE_LOG(LogRusteal, Display, TEXT("[Rusteal] %s: Rust library loaded."),
           *Library.Name.ToString());
  }

  return bLoaded;
#endif
}

static void UnloadLibrary(FRustealLibrary &Library) {
  if (!Library.Handle && !Library.Exports) {
    return;
  }

  RustealCallLibrary(&Library, [](const FRustealRustCallbacks &Callbacks) {
    if (Callbacks.on_shutdown) {
      Callbacks.on_shutdown();
    }
  });

  auto ShutdownFn =
      Library.Exports
          ? Library.Exports->shutdown
          : reinterpret_cast<FRustealShutdownFn>(FPlatformProcess::GetDllExport(
                Library.Handle, TEXT("rusteal_shutdown")));

  if (ShutdownFn) {
    FRustealLibraryScope Scope(&Library);
    ShutdownFn();
  }

  Library.Callbacks = nullptr;
  Library.Exports = nullptr;
  RustealPinnedForgetLibrary(&Library);
  RustealConsoleForgetLibrary(&Library);

  if (Library.Handle) {
    FreeHandle(Library);
  }

  if (!Library.LoadedPath.IsEmpty() &&
      Library.LoadedPath != Library.SourcePath) {
    IFileManager::Get().Delete(*Library.LoadedPath, false, true, true);
  }

  Library.LoadedPath.Reset();

  UE_LOG(LogRusteal, Display, TEXT("[Rusteal] %s: Rust library unloaded."),
         *Library.Name.ToString());
}

#if !RUSTEAL_STATIC
static bool ClassBelongsTo(const URustealReifiedClass *Class,
                           const FRustealLibrary *Library) {
  return Class->Library == Library;
}
#endif

static bool ReloadLibrary(FRustealLibrary &Library) {
#if RUSTEAL_STATIC
  UE_LOG(LogRusteal, Warning,
         TEXT("[Rusteal] %s is linked into the game: it cannot be reloaded."),
         *Library.Name.ToString());

  return false;
#else

  UE_LOG(LogRusteal, Display, TEXT("[Rusteal] === Hot reload of %s ==="),
         *Library.Name.ToString());

  const double Start = FPlatformTime::Seconds();
  double Mark = Start;

  auto Lap = [&Mark]() {
    const double Now = FPlatformTime::Seconds();
    const double Ms = (Now - Mark) * 1000.0;
    Mark = Now;
    return Ms;
  };

  if (Library.IsLoaded()) {
    int32 Dropped = 0;

    RustealReifyForEachReifiedInstance(
        [&Library, &Dropped](UObject *Obj, URustealReifiedClass *Class) {
          if (!ClassBelongsTo(Class, &Library)) {
            return;
          }

          RustealCallLibrary(
              &Library, [Obj, Class](const FRustealRustCallbacks &Cb) {
                Cb.drop_rust_instance(RustealUObjectHandle{Obj},
                                      Class->RustTypeId, nullptr);
              });

          Dropped++;
        });

    UE_LOG(LogRusteal, Display, TEXT("[Rusteal] Dropped %d Rust instances"),
           Dropped);
  }

  const double DropMs = Lap();
  UnloadLibrary(Library);

  const bool bLoaded = LoadLibrary(Library);
  const double LoadMs = Lap();

  RustealReifyReinstanceReplacedClasses();
  RustealReifyRefreshChangedInterfaces();
  const double ReinstanceMs = Lap();

  if (!bLoaded) {
    UE_LOG(LogRusteal, Error, TEXT("[Rusteal] Hot reload of %s failed."),
           *Library.Name.ToString());

    return false;
  }

  int32 Constructed = 0;

  RustealReifyForEachReifiedInstance(
      [&Library, &Constructed](UObject *Obj, URustealReifiedClass *Class) {
        if (!ClassBelongsTo(Class, &Library)) {
          return;
        }

        const bool bIsCDO = Obj->HasAnyFlags(RF_ClassDefaultObject);

        RustealCallLibrary(
            &Library, [Obj, Class, bIsCDO](const FRustealRustCallbacks &Cb) {
              Cb.construct_rust_instance(RustealUObjectHandle{Obj},
                                         Class->RustTypeId, bIsCDO);
            });

        Constructed++;
      });

  UE_LOG(LogRusteal, Display, TEXT("[Rusteal] Reconstructed %d Rust instances"),
         Constructed);

  const double ConstructMs = Lap();

  UE_LOG(LogRusteal, Display,
         TEXT("[Rusteal] === Hot reload of %s done in %.0f ms (Rust data "
              "dropped %.0f, library loaded %.0f, classes reinstanced %.0f, "
              "Rust data built %.0f) ==="),
         *Library.Name.ToString(), (Mark - Start) * 1000.0, DropMs, LoadMs,
         ReinstanceMs, ConstructMs);

  return true;
#endif
}

static bool RegisterLibrary(FName Name, const FString &PackagePath,
                            const FString &LibraryPath, void *const *FuncTable,
                            uint32 FuncCount, uint64 FuncFingerprint) {
  FRustealLibrary *Library = FindLibrary(Name);

  if (Library && Library->IsLoaded()) {
    UE_LOG(LogRusteal, Error,
           TEXT("[Rusteal] %s: a library of that name is "
                "already loaded."),
           *Name.ToString());

    return false;
  }

  if (!Library) {
    Library = GLibraries.Add_GetRef(MakeUnique<FRustealLibrary>()).Get();
    Library->Name = Name;
  }

  Library->SourcePath = LibraryPath;
  Library->PackagePath = PackagePath;
#if WITH_EDITOR
  DeleteStaleLoadCopies(*Library);
#endif
  Library->Table = GApiTable;
  Library->Table.func_table = reinterpret_cast<const void *const *>(FuncTable);
  Library->Table.func_count = FuncCount;
  Library->Table.func_fingerprint = FuncFingerprint;
  const bool bLoaded = LoadLibrary(*Library);
  RustealOnLibraryRegistered().Broadcast(Name, LibraryPath);
  return bLoaded;
}

bool RustealRegisterLibrary(FName Name, const FString &LibraryPath,
                            void *const *FuncTable, uint32 FuncCount) {
  return RustealRegisterLibraryChecked(Name, LibraryPath, FuncTable, FuncCount,
                                       0);
}

bool RustealRegisterLibraryChecked(FName Name, const FString &LibraryPath,
                                   void *const *FuncTable, uint32 FuncCount,
                                   uint64 FuncFingerprint) {
  FModuleManager::Get().LoadModuleChecked<IModuleInterface>(TEXT("Rusteal"));

  return RegisterLibrary(Name, TEXT("/Script/") + Name.ToString(), LibraryPath,
                         FuncTable, FuncCount, FuncFingerprint);
}

bool RustealRegisterPluginLibrary(const FString &PluginName,
                                  void *const *FuncTable, uint32 FuncCount) {
  return RustealRegisterPluginLibraryChecked(PluginName, FuncTable, FuncCount,
                                             0);
}

bool RustealRegisterPluginLibraryChecked(const FString &PluginName,
                                         void *const *FuncTable,
                                         uint32 FuncCount,
                                         uint64 FuncFingerprint) {
  return RustealRegisterPluginModuleLibraryChecked(
      PluginName, PluginName, FuncTable, FuncCount, FuncFingerprint);
}

bool RustealRegisterPluginModuleLibraryChecked(const FString &PluginName,
                                               const FString &LibraryName,
                                               void *const *FuncTable,
                                               uint32 FuncCount,
                                               uint64 FuncFingerprint) {
  TSharedPtr<IPlugin> Plugin = IPluginManager::Get().FindPlugin(PluginName);

  if (!Plugin.IsValid()) {
    UE_LOG(LogRusteal, Error,
           TEXT("[Rusteal] %s: no plugin of that name, its Rust library is "
                "not loaded."),
           *PluginName);

    return false;
  }

  const FString FileName =
      RustealLibraryFileName(TEXT("rusteal_") + LibraryName);

  FString Path =
      FPaths::Combine(Plugin->GetBaseDir(), TEXT("Binaries"),
                      FPlatformProcess::GetBinariesSubdirectory(), FileName);

  const FString Prebuilt =
      FPaths::Combine(Plugin->GetBaseDir(), TEXT("Source"), TEXT("ThirdParty"),
                      TEXT("RustLibraries"),
                      FPlatformProcess::GetBinariesSubdirectory(), FileName);

  if (!FPaths::FileExists(Path) && FPaths::FileExists(Prebuilt)) {
    Path = Prebuilt;
  }

  return RustealRegisterLibraryChecked(FName(*LibraryName), Path, FuncTable,
                                       FuncCount, FuncFingerprint);
}

void RustealUnregisterLibrary(FName Name) {
  if (FRustealLibrary *Library = FindLibrary(Name)) {
    UnloadLibrary(*Library);
  }
}

void RustealForEachLibrary(
    TFunctionRef<void(FName Name, const FString &DeployedPath)> Callback) {
  for (const TUniquePtr<FRustealLibrary> &Library : GLibraries) {
    Callback(Library->Name, Library->SourcePath);
  }
}

FRustealLibraryRegistered &RustealOnLibraryRegistered() {
  static FRustealLibraryRegistered Registered;
  return Registered;
}

bool RustealReloadLibrary(FName Name) {
  FRustealLibrary *Library = FindLibrary(Name);
  return Library && ReloadLibrary(*Library);
}

static void ReloadCommand(const TArray<FString> &Args) {
  if (Args.Num() == 0) {
    for (const TUniquePtr<FRustealLibrary> &Library : GLibraries) {
      if (!Library->SourcePath.IsEmpty()) {
        ReloadLibrary(*Library);
      }
    }

    return;
  }

  for (const FString &Arg : Args) {
    if (FRustealLibrary *Library = FindLibrary(FName(*Arg))) {
      ReloadLibrary(*Library);
    } else {
      UE_LOG(LogRusteal, Error, TEXT("[Rusteal] Rusteal.Reload: no library %s"),
             *Arg);
    }
  }
}

static FAutoConsoleCommand CmdReload(
    TEXT("Rusteal.Reload"),
    TEXT("Hot-reload Rust libraries (unload, copy, load): the ones named, or "
         "all of them."),
    FConsoleCommandWithArgsDelegate::CreateStatic(&ReloadCommand));

void FRustealModule::StartupModule() {
  FillApiTable();
  RustealRegisterComponentListResync();
  RustealReifyRegisterDeleteListener();

  GTickerHandle = FTSTicker::GetCoreTicker().AddTicker(
      FTickerDelegate::CreateStatic(&TickLibraries));

#if RUSTEAL_HOSTS_GAME_LIBRARY
  RustealFillFuncTable();

  const FString GamePath = FPaths::Combine(
      FPaths::ProjectPluginsDir(), TEXT("Rusteal"), TEXT("Binaries"),
      FPlatformProcess::GetBinariesSubdirectory(),
      RustealLibraryFileName(TEXT("rusteal")));

  RegisterLibrary(FName(FApp::GetProjectName()), TEXT("/Script/Rusteal"),
                  GamePath, RustealGetFuncTable(), RustealGetFuncCount(),
                  RustealGetFuncFingerprint());
#endif
}

void FRustealModule::ShutdownModule() {
  FTSTicker::GetCoreTicker().RemoveTicker(GTickerHandle);
  RustealUnregisterComponentListResync();

  for (const TUniquePtr<FRustealLibrary> &Library : GLibraries) {
    UnloadLibrary(*Library);
  }

  RustealReifyUnregisterDeleteListener();
  RustealPinnedShutdown();
}

#undef LOCTEXT_NAMESPACE

IMPLEMENT_MODULE(FRustealModule, Rusteal)
