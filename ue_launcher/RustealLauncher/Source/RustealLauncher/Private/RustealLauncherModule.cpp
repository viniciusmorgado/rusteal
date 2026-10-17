// Copyright 2026 Vinicius Donatto Morgado.

#include "Framework/Application/SlateApplication.h"
#include "Interfaces/IMainFrameModule.h"
#include "Misc/Paths.h"
#include "Modules/ModuleManager.h"
#include "RustealLauncherCli.h"
#include "SRustealCreateProject.h"
#include "Widgets/SWindow.h"

#define LOCTEXT_NAMESPACE "RustealLauncher"

class FRustealLauncherModule : public IModuleInterface {
public:
  virtual void StartupModule() override {
    TOptional<FRustealLauncherStub> Found =
        FRustealLauncherStub::Read(FPaths::ProjectDir());

    if (!Found.IsSet()) {
      return;
    }

    Stub = Found.GetValue();

    IMainFrameModule &MainFrame =
        FModuleManager::LoadModuleChecked<IMainFrameModule>(TEXT("MainFrame"));

    if (MainFrame.IsWindowInitialized()) {
      Open(MainFrame.GetParentWindow(), false);
      return;
    }

    MainFrameHandle = MainFrame.OnMainFrameCreationFinished().AddRaw(
        this, &FRustealLauncherModule::Open);
  }

  virtual void ShutdownModule() override {
    if (MainFrameHandle.IsValid() &&
        FModuleManager::Get().IsModuleLoaded(TEXT("MainFrame"))) {
      FModuleManager::GetModuleChecked<IMainFrameModule>(TEXT("MainFrame"))
          .OnMainFrameCreationFinished()
          .Remove(MainFrameHandle);
    }
  }

private:
  void Open(TSharedPtr<SWindow> Parent, bool bIsRunningStartupDialog) {
    if (bIsRunningStartupDialog || !Parent.IsValid()) {
      return;
    }

    TSharedRef<SWindow> Window =
        SNew(SWindow)
            .Title(LOCTEXT("WindowTitle", "Rusteal"))
            .ClientSize(FVector2D(760.0f, 520.0f))
            .SupportsMinimize(false)
            .SupportsMaximize(false)[SNew(SRustealCreateProject).Stub(Stub)];

    FSlateApplication::Get().AddWindowAsNativeChild(Window,
                                                    Parent.ToSharedRef());
  }

  FRustealLauncherStub Stub;
  FDelegateHandle MainFrameHandle;
};

#undef LOCTEXT_NAMESPACE

IMPLEMENT_MODULE(FRustealLauncherModule, RustealLauncher)
