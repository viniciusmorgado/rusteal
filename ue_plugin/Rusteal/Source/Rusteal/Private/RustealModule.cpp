#include "RustealModule.h"
#include "RustealApiTable.h"
#include "URustealReifiedClass.h"
#include "HAL/PlatformProcess.h"
#include "HAL/PlatformFileManager.h"
#include "HAL/FileManager.h"
#include "Misc/Paths.h"

DEFINE_LOG_CATEGORY(LogRusteal);

// External API sub-table instances (defined in their respective *Impl.cpp files)
extern FRustealCoreApi       GCoreApi;
extern FRustealReflectionApi GReflectionApi;
extern FRustealPropertyApi   GPropertyApi;
extern FRustealContainerApi  GContainerApi;
extern FRustealDelegateApi   GDelegateApi;
extern FRustealLifecycleApi  GLifecycleApi;
extern FRustealReifyApi      GReifyApi;
extern FRustealWorldApi      GWorldApi;
extern FRustealWidgetApi     GWidgetApi;

// Reify helpers (defined in RustealReifyApiImpl.cpp)
extern void RustealReifyRegisterDeleteListener();
extern void RustealReifyUnregisterDeleteListener();

// Pinned lifecycle helpers (defined in RustealLifecycleApiImpl.cpp)
extern void RustealPinnedUnregisterDeleteListener();
extern void RustealReifyForEachReifiedInstance(
    TFunctionRef<void(UObject*, URustealReifiedClass*)> Callback);

// Module-level storage for Rust callbacks (set during StartupModule, read by URustealDelegateProxy).
static const FRustealRustCallbacks* GRustCallbacks = nullptr;

const FRustealRustCallbacks* GetRustealRustCallbacks()
{
    return GRustCallbacks;
}

// Forward declarations for generated func_table code
extern void RustealFillFuncTable();
extern void** RustealGetFuncTable();
extern uint32_t RustealGetFuncCount();

#define LOCTEXT_NAMESPACE "FRustealModule"

// ---------------------------------------------------------------------------
// Logging bridge (the one API implemented in Phase 1 for end-to-end testing)
// ---------------------------------------------------------------------------

static void RustealLogImpl(uint8 Level, const uint8* Msg, uint32 MsgLen)
{
    const FString MsgStr(MsgLen, UTF8_TO_TCHAR(reinterpret_cast<const char*>(Msg)));
    switch (Level)
    {
    case 0:  UE_LOG(LogRusteal, Display, TEXT("%s"), *MsgStr); break;
    case 1:  UE_LOG(LogRusteal, Warning, TEXT("%s"), *MsgStr); break;
    default: UE_LOG(LogRusteal, Error,   TEXT("%s"), *MsgStr); break;
    }
}

static FRustealLoggingApi GLoggingApi = { &RustealLogImpl };

// ---------------------------------------------------------------------------
// API table instance
// ---------------------------------------------------------------------------

static FRustealApiTable GApiTable;

static void FillApiTable()
{
    FMemory::Memzero(GApiTable);
    GApiTable.version = 1;

    // Implemented sub-tables
    GApiTable.logging    = &GLoggingApi;
    GApiTable.core       = &GCoreApi;
    GApiTable.property   = &GPropertyApi;
    GApiTable.reflection = &GReflectionApi;
    GApiTable.container    = &GContainerApi;
    GApiTable.delegate     = &GDelegateApi;
    GApiTable.lifecycle    = &GLifecycleApi;
    GApiTable.reify        = &GReifyApi;
    GApiTable.world        = &GWorldApi;
    GApiTable.widget       = &GWidgetApi;

    // Fill generated func_table (Phase 6)
    RustealFillFuncTable();
    GApiTable.func_table = reinterpret_cast<const void* const*>(RustealGetFuncTable());
    GApiTable.func_count = static_cast<uint32>(RustealGetFuncCount());
}

// ---------------------------------------------------------------------------
// Console command
// ---------------------------------------------------------------------------

static FAutoConsoleCommand CmdReload(
    TEXT("Rusteal.Reload"),
    TEXT("Hot-reload the Rust DLL (unload → copy → load)."),
    FConsoleCommandDelegate::CreateStatic(&FRustealModule::StaticReload));

void FRustealModule::StaticReload()
{
    FRustealModule& Module = FModuleManager::GetModuleChecked<FRustealModule>(TEXT("Rusteal"));
    Module.ReloadRustDll();
}

// ---------------------------------------------------------------------------
// Module lifecycle
// ---------------------------------------------------------------------------

FString FRustealModule::HotCopyPath() const
{
    // Same directory and extension as the source library, numbered per (re)load so the
    // source file is never locked while loaded.
    return FPaths::Combine(
        FPaths::GetPath(DllSourcePath),
        FString::Printf(TEXT("%srusteal_hot_%d.%s"),
            FPlatformProcess::GetModulePrefix(),
            ReloadCount,
            FPlatformProcess::GetModuleExtension()));
}

void FRustealModule::StartupModule()
{
    // 1. Fill the API table
    FillApiTable();

    // 2. Locate the Rust DLL
    const FString PluginDir = FPaths::Combine(
        FPaths::ProjectPluginsDir(), TEXT("Rusteal"));
    // Platform-native name: rusteal.dll on Windows, librusteal.so on Linux, librusteal.dylib on
    // macOS. The rusteal-cli deploy step (HostPlatform::deployed_lib_filename) must agree.
    DllSourcePath = FPaths::Combine(
        PluginDir, TEXT("Binaries"),
        FPlatformProcess::GetBinariesSubdirectory(),
        FString::Printf(TEXT("%srusteal.%s"),
            FPlatformProcess::GetModulePrefix(),
            FPlatformProcess::GetModuleExtension()));

    if (!FPaths::FileExists(DllSourcePath))
    {
        UE_LOG(LogRusteal, Warning,
            TEXT("[Rusteal] Rust DLL not found at %s — Rust side will not be loaded."),
            *DllSourcePath);
        return;
    }

    // 3. Copy-on-load: never lock the source DLL so that build.py / cargo
    //    can always overwrite it, and hot reload always reads the latest.
    ReloadCount++;
    const FString InitialCopyPath = HotCopyPath();

    uint32 CopyResult = IFileManager::Get().Copy(*InitialCopyPath, *DllSourcePath);
    if (CopyResult != 0)
    {
        UE_LOG(LogRusteal, Error,
            TEXT("[Rusteal] Failed to copy DLL %s → %s (error %u). Falling back to direct load."),
            *DllSourcePath, *InitialCopyPath, CopyResult);
        // Fallback: load directly (will lock the source, but at least it works)
        UE_LOG(LogRusteal, Warning, TEXT("[Rusteal] Using fallback DLL path: %s"), *DllSourcePath);
        if (!LoadRustDll(DllSourcePath))
        {
            return;
        }
    }
    else if (!LoadRustDll(InitialCopyPath))
    {
        return;
    }

    UE_LOG(LogRusteal, Display, TEXT("[Rusteal] Rust DLL loaded and initialized successfully."));
}

void FRustealModule::ShutdownModule()
{
    UnloadRustDll();

    // Clean up the hot-copy DLL (now unlocked).
    if (!CurrentLoadedDllPath.IsEmpty() && CurrentLoadedDllPath != DllSourcePath)
    {
        IFileManager::Get().Delete(*CurrentLoadedDllPath, false, true, true);
    }
}

// ---------------------------------------------------------------------------
// DLL load / unload helpers
// ---------------------------------------------------------------------------

bool FRustealModule::LoadRustDll(const FString& LoadPath)
{
    DllHandle = FPlatformProcess::GetDllHandle(*LoadPath);
    if (!DllHandle)
    {
        UE_LOG(LogRusteal, Error, TEXT("[Rusteal] Failed to load DLL: %s"), *LoadPath);
        return false;
    }
    CurrentLoadedDllPath = LoadPath;

    // Resolve entry points
    auto InitFn = reinterpret_cast<FRustealInitFn>(
        FPlatformProcess::GetDllExport(DllHandle, TEXT("rusteal_init")));
    if (!InitFn)
    {
        UE_LOG(LogRusteal, Error, TEXT("[Rusteal] rusteal_init not found in DLL"));
        FPlatformProcess::FreeDllHandle(DllHandle);
        DllHandle = nullptr;
        return false;
    }

    // Initialize Rust side
    RustCallbacks = InitFn(&GApiTable);
    if (!RustCallbacks)
    {
        UE_LOG(LogRusteal, Error, TEXT("[Rusteal] rusteal_init returned null"));
        FPlatformProcess::FreeDllHandle(DllHandle);
        DllHandle = nullptr;
        return false;
    }

    // Store globally so URustealDelegateProxy can access Rust callbacks.
    GRustCallbacks = RustCallbacks;

    // Register the UObject delete listener for reified class instance cleanup.
    RustealReifyRegisterDeleteListener();

    return true;
}

void FRustealModule::UnloadRustDll()
{
    // Unregister delete listeners before shutting down Rust.
    RustealReifyUnregisterDeleteListener();
    RustealPinnedUnregisterDeleteListener();

    if (DllHandle)
    {
        // Notify Rust side
        if (RustCallbacks && RustCallbacks->on_shutdown)
        {
            RustCallbacks->on_shutdown();
        }

        // Call rusteal_shutdown if available
        auto ShutdownFn = reinterpret_cast<FRustealShutdownFn>(
            FPlatformProcess::GetDllExport(DllHandle, TEXT("rusteal_shutdown")));
        if (ShutdownFn)
        {
            ShutdownFn();
        }

        FPlatformProcess::FreeDllHandle(DllHandle);
        DllHandle = nullptr;
        RustCallbacks = nullptr;
        GRustCallbacks = nullptr;

        UE_LOG(LogRusteal, Display, TEXT("[Rusteal] Rust DLL unloaded."));
    }
}

// ---------------------------------------------------------------------------
// Reified instance teardown / reconstruct helpers
// ---------------------------------------------------------------------------

void FRustealModule::TeardownReifiedInstances()
{
    if (DllHandle && RustCallbacks && RustCallbacks->drop_rust_instance)
    {
        int32 InstanceCount = 0;
        RustealReifyForEachReifiedInstance(
            [this, &InstanceCount](UObject* Obj, URustealReifiedClass* ReifiedClass)
            {
                // TODO: pass bIsCDO to drop_rust_instance when callback signature is extended
                // bool bIsCDO = Obj->HasAnyFlags(RF_ClassDefaultObject);
                RustCallbacks->drop_rust_instance(
                    RustealUObjectHandle{ Obj },
                    ReifiedClass->RustTypeId,
                    nullptr);
                InstanceCount++;
            });
        UE_LOG(LogRusteal, Display,
            TEXT("[Rusteal] Dropped %d Rust instances"), InstanceCount);
    }
}

void FRustealModule::ReconstructReifiedInstances()
{
    if (RustCallbacks && RustCallbacks->construct_rust_instance)
    {
        int32 ReconstructCount = 0;
        RustealReifyForEachReifiedInstance(
            [this, &ReconstructCount](UObject* Obj, URustealReifiedClass* ReifiedClass)
            {
                bool bIsCDO = Obj->HasAnyFlags(RF_ClassDefaultObject);
                RustCallbacks->construct_rust_instance(
                    RustealUObjectHandle{ Obj },
                    ReifiedClass->RustTypeId,
                    bIsCDO);
                ReconstructCount++;
            });
        UE_LOG(LogRusteal, Display,
            TEXT("[Rusteal] Reconstructed %d Rust instances"), ReconstructCount);
    }
}

// ---------------------------------------------------------------------------
// Hot reload (DLL swap)
// ---------------------------------------------------------------------------

void FRustealModule::ReloadRustDll()
{
    UE_LOG(LogRusteal, Display, TEXT("[Rusteal] === Hot Reload Begin ==="));

    if (DllSourcePath.IsEmpty())
    {
        UE_LOG(LogRusteal, Error,
            TEXT("[Rusteal] Hot reload failed: DLL source path not set (was initial load skipped?)"));
        return;
    }

    // Phase 1: Teardown — drop all Rust instances and unload old DLL
    TeardownReifiedInstances();

    FString PreviousLoadedPath = CurrentLoadedDllPath;
    UnloadRustDll();

    if (!PreviousLoadedPath.IsEmpty() && PreviousLoadedPath != DllSourcePath)
    {
        IFileManager::Get().Delete(*PreviousLoadedPath, false, true, true);
    }

    // Phase 2: Copy-on-reload — copy the new DLL to avoid Windows lock
    if (!FPaths::FileExists(DllSourcePath))
    {
        UE_LOG(LogRusteal, Error,
            TEXT("[Rusteal] Hot reload failed: %s not found. Did cargo build succeed?"),
            *DllSourcePath);
        return;
    }

    ReloadCount++;
    const FString HotDllPath = HotCopyPath();

    uint32 CopyResult = IFileManager::Get().Copy(*HotDllPath, *DllSourcePath);
    if (CopyResult != 0)
    {
        UE_LOG(LogRusteal, Error,
            TEXT("[Rusteal] Hot reload failed: could not copy %s → %s (error %u)"),
            *DllSourcePath, *HotDllPath, CopyResult);
        return;
    }

    // Phase 2b: Load the new DLL and re-initialize Rust
    if (!LoadRustDll(HotDllPath))
    {
        UE_LOG(LogRusteal, Error, TEXT("[Rusteal] Hot reload failed: could not load new DLL"));
        return;
    }

    // Phase 3: Reconstruct — rebuild Rust instance data
    ReconstructReifiedInstances();

    UE_LOG(LogRusteal, Display, TEXT("[Rusteal] === Hot Reload Complete ==="));
}

#undef LOCTEXT_NAMESPACE

IMPLEMENT_MODULE(FRustealModule, Rusteal)
