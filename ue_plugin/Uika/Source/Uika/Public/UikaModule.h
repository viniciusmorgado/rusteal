#pragma once

#include "Modules/ModuleManager.h"

DECLARE_LOG_CATEGORY_EXTERN(LogUika, Log, All);

/** The callback table the Rust library handed to the module at load; null while unloaded.
 *  Defined in UikaModule.cpp; declared here, with the module that owns it, so every
 *  translation unit that uses it sees the same declaration (unity builds used to hide a
 *  missing one). */
const struct FUikaRustCallbacks* GetUikaRustCallbacks();

class FUikaModule : public IModuleInterface
{
public:
    virtual void StartupModule() override;
    virtual void ShutdownModule() override;

    /** Unload the current Rust DLL, copy the new one, and reload. */
    void ReloadRustDll();

    /** Static entry point for the Uika.Reload console command. */
    static void StaticReload();

private:
    /** Drop Rust instance data for all reified objects. */
    void TeardownReifiedInstances();

    /** Reconstruct Rust instance data for all reified objects. */
    void ReconstructReifiedInstances();
    /** Unload the Rust DLL (teardown phase of reload, and used by ShutdownModule). */
    void UnloadRustDll();

    /** Load a Rust DLL from the given path and initialize it. */
    bool LoadRustDll(const FString& LoadPath);

    void* DllHandle = nullptr;
    const struct FUikaRustCallbacks* RustCallbacks = nullptr;

    /** Canonical path to the Rust library (uika.dll / libuika.so / libuika.dylib). */
    FString DllSourcePath;

    /** Path of the numbered copy the current (re)load uses. */
    FString HotCopyPath() const;

    /** Path of the currently loaded DLL (may be a hot-copy). */
    FString CurrentLoadedDllPath;

    /** Incrementing counter for copy-on-reload filenames. */
    int32 ReloadCount = 0;
};
