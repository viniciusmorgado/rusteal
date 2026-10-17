// Copyright 2026 Vinicius Donatto Morgado.

#include "Framework/Application/SlateApplication.h"
#include "HAL/IConsoleManager.h"
#include "Interfaces/IMainFrameModule.h"
#include "Misc/App.h"
#include "Misc/Paths.h"
#include "Modules/ModuleManager.h"
#include "RustealLauncherCli.h"
#include "SRustealCreateProject.h"
#include "Widgets/SWindow.h"

#define LOCTEXT_NAMESPACE "RustealLauncher"

DEFINE_LOG_CATEGORY_STATIC(LogRustealLauncher, Log, All);

class FRustealLauncherModule : public IModuleInterface {
public:
  virtual void StartupModule() override {
    TOptional<FRustealLauncherStub> Found =
        FRustealLauncherStub::Read(FPaths::ProjectDir());

    if (!Found.IsSet()) {
      return;
    }

    Stub = Found.GetValue();

    CreateCommand = IConsoleManager::Get().RegisterConsoleCommand(
        TEXT("Rusteal.Launcher.Create"),
        TEXT("Create the Rust project, as the Create button of the Rusteal "
             "window does."),
        FConsoleCommandDelegate::CreateRaw(this,
                                           &FRustealLauncherModule::Create));

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
    if (CreateCommand) {
      IConsoleManager::Get().UnregisterConsoleObject(CreateCommand);
      CreateCommand = nullptr;
    }

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

    UE_LOG(LogRustealLauncher, Display,
           TEXT("%s was made from the Rust template '%s': opening the Rusteal "
                "window"),
           FApp::GetProjectName(), *Stub.Template);

    TSharedRef<SWindow> Window =
        SNew(SWindow)
            .Title(LOCTEXT("WindowTitle", "Rusteal"))
            .ClientSize(FVector2D(760.0f, 520.0f))
            .SupportsMinimize(false)
            .SupportsMaximize(
                false)[SAssignNew(Widget, SRustealCreateProject).Stub(Stub)];

    FSlateApplication::Get().AddWindowAsNativeChild(Window,
                                                    Parent.ToSharedRef());
  }

  void Create() {
    if (TSharedPtr<SRustealCreateProject> Pinned = Widget.Pin()) {
      Pinned->Create();
    }
  }

  FRustealLauncherStub Stub;
  FDelegateHandle MainFrameHandle;
  TWeakPtr<SRustealCreateProject> Widget;
  IConsoleObject *CreateCommand = nullptr;
};

#undef LOCTEXT_NAMESPACE

IMPLEMENT_MODULE(FRustealLauncherModule, RustealLauncher)
