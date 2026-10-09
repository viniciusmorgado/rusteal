#include "RustealModule.h"
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

#if PLATFORM_LINUX || PLATFORM_MAC
#include <unistd.h>
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

extern void RustealReifyRegisterDeleteListener();
extern void RustealReifyUnregisterDeleteListener();
extern void RustealReifyForEachReifiedInstance(
    TFunctionRef<void(UObject *, URustealReifiedClass *)> Callback);
extern void RustealReifyReinstanceReplacedClasses();

extern void RustealRegisterComponentListResync();
extern void RustealUnregisterComponentListResync();

extern void RustealConsoleForgetLibrary(FRustealLibrary *Library);

extern void RustealPinnedForgetLibrary(FRustealLibrary *Library);
extern void RustealPinnedShutdown();

extern void RustealFillFuncTable();
extern void **RustealGetFuncTable();
extern uint32_t RustealGetFuncCount();

#define LOCTEXT_NAMESPACE "FRustealModule"

static void RustealLogImpl(uint8 Level, const uint8 *Msg, uint32 MsgLen) {
  const FString MsgStr(MsgLen,
                       UTF8_TO_TCHAR(reinterpret_cast<const char *>(Msg)));

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

static FRustealLoggingApi GLoggingApi = {&RustealLogImpl};

static FRustealApiTable GApiTable;
static bool GApiTableFilled = false;

static uint32 RustealPluginVersion() {
  TSharedPtr<IPlugin> Plugin =
      IPluginManager::Get().FindPlugin(TEXT("Rusteal"));

  return Plugin.IsValid() ? static_cast<uint32>(Plugin->GetDescriptor().Version)
                          : 0;
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

FString RustealLibraryFileName(const FString &Stem) {
  return FString::Printf(TEXT("%s%s.%s"), FPlatformProcess::GetModulePrefix(),
                         *Stem, FPlatformProcess::GetModuleExtension());
}

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

static void FreeHandle(FRustealLibrary &Library) {
  FPlatformProcess::FreeDllHandle(Library.Handle);
  Library.Handle = nullptr;
}

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

static bool MakeLoadCopy(const FString &Source, const FString &Copy) {
#if PLATFORM_LINUX || PLATFORM_MAC
  IFileManager::Get().Delete(*Copy, false, true, true);

  if (link(TCHAR_TO_UTF8(*Source), TCHAR_TO_UTF8(*Copy)) == 0) {
    return true;
  }

#endif
  return IFileManager::Get().Copy(*Copy, *Source) == COPY_OK;
}

static bool LoadLibrary(FRustealLibrary &Library) {
  if (!FPaths::FileExists(Library.SourcePath)) {
    UE_LOG(LogRusteal, Warning,
           TEXT("[Rusteal] %s: no Rust library at %s, its Rust side will not "
                "be loaded."),
           *Library.Name.ToString(), *Library.SourcePath);

    return false;
  }

  Library.ReloadCount++;
  const FString CopyPath = HotCopyPath(Library);
  bool bLoaded;

  if (!MakeLoadCopy(Library.SourcePath, CopyPath)) {
    UE_LOG(LogRusteal, Warning,
           TEXT("[Rusteal] %s: could not copy %s to %s, loading it in place."),
           *Library.Name.ToString(), *Library.SourcePath, *CopyPath);

    bLoaded = LoadLibraryFile(Library, Library.SourcePath);
  } else {
    bLoaded = LoadLibraryFile(Library, CopyPath);
  }

  if (bLoaded) {
    UE_LOG(LogRusteal, Display, TEXT("[Rusteal] %s: Rust library loaded."),
           *Library.Name.ToString());
  }

  return bLoaded;
}

static void UnloadLibrary(FRustealLibrary &Library) {
  if (!Library.Handle) {
    return;
  }

  RustealCallLibrary(&Library, [](const FRustealRustCallbacks &Callbacks) {
    if (Callbacks.on_shutdown) {
      Callbacks.on_shutdown();
    }
  });

  auto ShutdownFn = reinterpret_cast<FRustealShutdownFn>(
      FPlatformProcess::GetDllExport(Library.Handle, TEXT("rusteal_shutdown")));

  if (ShutdownFn) {
    FRustealLibraryScope Scope(&Library);
    ShutdownFn();
  }

  Library.Callbacks = nullptr;
  RustealPinnedForgetLibrary(&Library);
  RustealConsoleForgetLibrary(&Library);
  FreeHandle(Library);

  if (!Library.LoadedPath.IsEmpty() &&
      Library.LoadedPath != Library.SourcePath) {
    IFileManager::Get().Delete(*Library.LoadedPath, false, true, true);
  }

  Library.LoadedPath.Reset();

  UE_LOG(LogRusteal, Display, TEXT("[Rusteal] %s: Rust library unloaded."),
         *Library.Name.ToString());
}

static bool ClassBelongsTo(const URustealReifiedClass *Class,
                           const FRustealLibrary *Library) {
  return Class->Library == Library;
}

static bool ReloadLibrary(FRustealLibrary &Library) {
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
}

static bool RegisterLibrary(FName Name, const FString &PackagePath,
                            const FString &LibraryPath, void *const *FuncTable,
                            uint32 FuncCount) {
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
  DeleteStaleLoadCopies(*Library);
  Library->Table = GApiTable;
  Library->Table.func_table = reinterpret_cast<const void *const *>(FuncTable);
  Library->Table.func_count = FuncCount;
  const bool bLoaded = LoadLibrary(*Library);
  RustealOnLibraryRegistered().Broadcast(Name, LibraryPath);
  return bLoaded;
}

bool RustealRegisterLibrary(FName Name, const FString &LibraryPath,
                            void *const *FuncTable, uint32 FuncCount) {
  FModuleManager::Get().LoadModuleChecked<IModuleInterface>(TEXT("Rusteal"));

  return RegisterLibrary(Name, TEXT("/Script/") + Name.ToString(), LibraryPath,
                         FuncTable, FuncCount);
}

bool RustealRegisterPluginLibrary(const FString &PluginName,
                                  void *const *FuncTable, uint32 FuncCount) {
  TSharedPtr<IPlugin> Plugin = IPluginManager::Get().FindPlugin(PluginName);

  if (!Plugin.IsValid()) {
    UE_LOG(LogRusteal, Error,
           TEXT("[Rusteal] %s: no plugin of that name, its Rust library is "
                "not loaded."),
           *PluginName);

    return false;
  }

  const FString Path =
      FPaths::Combine(Plugin->GetBaseDir(), TEXT("Binaries"),
                      FPlatformProcess::GetBinariesSubdirectory(),
                      RustealLibraryFileName(TEXT("rusteal_") + PluginName));

  return RustealRegisterLibrary(FName(*PluginName), Path, FuncTable, FuncCount);
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

  RustealFillFuncTable();

  const FString GamePath = FPaths::Combine(
      FPaths::ProjectPluginsDir(), TEXT("Rusteal"), TEXT("Binaries"),
      FPlatformProcess::GetBinariesSubdirectory(),
      RustealLibraryFileName(TEXT("rusteal")));

  RegisterLibrary(FName(FApp::GetProjectName()), TEXT("/Script/Rusteal"),
                  GamePath, RustealGetFuncTable(), RustealGetFuncCount());
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
