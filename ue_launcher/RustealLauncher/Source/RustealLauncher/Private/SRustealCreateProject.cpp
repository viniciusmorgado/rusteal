// Copyright 2026 Vinicius Donatto Morgado.

#include "SRustealCreateProject.h"

#include "Framework/Application/SlateApplication.h"
#include "HAL/FileManager.h"
#include "HAL/PlatformProcess.h"
#include "Misc/App.h"
#include "Misc/FileHelper.h"
#include "Misc/Paths.h"
#include "Styling/AppStyle.h"
#include "UnrealEdMisc.h"
#include "Widgets/Images/SThrobber.h"
#include "Widgets/Input/SButton.h"
#include "Widgets/Layout/SBorder.h"
#include "Widgets/Layout/SScrollBox.h"
#include "Widgets/SBoxPanel.h"
#include "Widgets/SWindow.h"
#include "Widgets/Text/STextBlock.h"

#define LOCTEXT_NAMESPACE "RustealLauncher"

DEFINE_LOG_CATEGORY_STATIC(LogRustealLauncher, Log, All);

static constexpr int32 MaxLogLines = 400;

void SRustealCreateProject::Construct(const FArguments &InArgs) {
  Stub = InArgs._Stub;
  FFileHelper::LoadFileToString(StubProject, *FPaths::GetProjectFilePath());

  for (const TCHAR *Directory :
       {TEXT("Source"), TEXT("Plugins"), TEXT("Binaries")}) {
    const FString Path = FPaths::Combine(FPaths::ProjectDir(), Directory);

    if (!FPaths::DirectoryExists(Path)) {
      CodeDirectoriesToRemove.Add(Path);
    }
  }
  bValidName = RustealLauncherCli::IsValidProjectName(FApp::GetProjectName());
  Check();

  const FText Summary = FText::Format(
      LOCTEXT("Summary",
              "{0} was made from a Rust template. Rusteal is turning it into "
              "a project with its gameplay in Rust, from the {1} template, "
              "and building it; the editor restarts into it when it is done."),
      FText::FromString(FApp::GetProjectName()),
      FText::FromString(Stub.Template));

  auto Requirement = [this](const FText &Name, TAttribute<FText> State,
                            TSharedRef<SWidget> Action) {
    return SNew(SHorizontalBox) +
           SHorizontalBox::Slot()
               .AutoWidth()
               .VAlign(VAlign_Center)
               .Padding(0, 0, 12, 0)[SNew(SBox).WidthOverride(
                   80.0f)[SNew(STextBlock)
                              .Font(FAppStyle::GetFontStyle("NormalFontBold"))
                              .Text(Name)]] +
           SHorizontalBox::Slot().FillWidth(1.0f).VAlign(
               VAlign_Center)[SNew(STextBlock).Text(State)] +
           SHorizontalBox::Slot().AutoWidth().VAlign(VAlign_Center)[Action];
  };

  ChildSlot
      [SNew(SBorder)
           .BorderImage(FAppStyle::GetBrush("ToolPanel.GroupBorder"))
           .Padding(16.0f)
               [SNew(SVerticalBox) +
                SVerticalBox::Slot().AutoHeight().Padding(0, 0, 0, 8)
                    [SNew(STextBlock)
                         .Font(FAppStyle::GetFontStyle("HeadingExtraSmall"))
                         .Text(LOCTEXT("Title", "Creating the Rust project"))] +
                SVerticalBox::Slot().AutoHeight().Padding(
                    0, 0, 0,
                    12)[SNew(STextBlock).AutoWrapText(true).Text(Summary)] +
                SVerticalBox::Slot().AutoHeight().Padding(0, 0, 0,
                                                          4)[Requirement(
                    LOCTEXT("Rust", "Rust"),
                    TAttribute<FText>::CreateSP(
                        this, &SRustealCreateProject::GetRustState),
                    SNew(SButton)
                        .Visibility_Lambda([this] {
                          return VisibleWhen(Cargo.IsEmpty() &&
                                             Running == ERun::None);
                        })
                        .Text(LOCTEXT("GetRust", "Get Rust"))
                        .OnClicked(this, &SRustealCreateProject::OnGetRust))] +
                SVerticalBox::Slot().AutoHeight().Padding(0, 0, 0,
                                                          12)[Requirement(
                    LOCTEXT("Rusteal", "Rusteal"),
                    TAttribute<FText>::CreateSP(
                        this, &SRustealCreateProject::GetRustealState),
                    SNew(SButton)
                        .Visibility_Lambda([this] {
                          return VisibleWhen(!Cargo.IsEmpty() &&
                                             !bRustealCreatesFromStubs &&
                                             Running == ERun::None);
                        })
                        .Text(this, &SRustealCreateProject::GetInstallLabel)
                        .OnClicked(this,
                                   &SRustealCreateProject::OnInstallRusteal))] +
                SVerticalBox::Slot().AutoHeight().Padding(
                    0, 0, 0,
                    8)[SNew(STextBlock)
                           .AutoWrapText(true)
                           .Text(this, &SRustealCreateProject::GetStatus)] +
                SVerticalBox::Slot().FillHeight(1.0f).Padding(0, 0, 0, 8)
                    [SNew(SBorder)
                         .BorderImage(FAppStyle::GetBrush("Brushes.Recessed"))
                         .Padding(
                             8.0f)[SAssignNew(LogScroll, SScrollBox) +
                                   SScrollBox::Slot()
                                       [SNew(STextBlock)
                                            .Font(FAppStyle::GetFontStyle(
                                                "MonospacedText"))
                                            .Text(this, &SRustealCreateProject::
                                                            GetLog)]]] +
                SVerticalBox::Slot().AutoHeight()
                    [SNew(SHorizontalBox) +
                     SHorizontalBox::Slot().AutoWidth().VAlign(VAlign_Center)
                         [SNew(SThrobber).Visibility_Lambda([this] {
                           return Running == ERun::None ? EVisibility::Hidden
                                                        : EVisibility::Visible;
                         })] +
                     SHorizontalBox::Slot().FillWidth(1.0f) +
                     SHorizontalBox::Slot().AutoWidth().Padding(4, 0)
                         [SNew(SButton)
                              .Visibility_Lambda([this] {
                                return VisibleWhen(bValidName && !IsReady() &&
                                                   Running == ERun::None);
                              })
                              .Text(LOCTEXT("CheckAgain", "Check again"))
                              .OnClicked(
                                  this, &SRustealCreateProject::OnCheckAgain)] +
                     SHorizontalBox::Slot().AutoWidth().Padding(4, 0)
                         [SNew(SButton)
                              .ButtonStyle(FAppStyle::Get(), "PrimaryButton")
                              .Visibility_Lambda([this] {
                                return VisibleWhen(bFailed && IsReady() &&
                                                   Running == ERun::None);
                              })
                              .Text(LOCTEXT("Retry", "Retry"))
                              .OnClicked(this,
                                         &SRustealCreateProject::OnRetry)] +
                     SHorizontalBox::Slot().AutoWidth().Padding(4, 0)
                         [SNew(SButton)
                              .Visibility_Lambda([this] {
                                return VisibleWhen(Running != ERun::None);
                              })
                              .Text(LOCTEXT("Cancel", "Cancel"))
                              .OnClicked(this,
                                         &SRustealCreateProject::OnCancel)] +
                     SHorizontalBox::Slot().AutoWidth().Padding(4, 0)
                         [SNew(SButton)
                              .Text(LOCTEXT("Close", "Close"))
                              .OnClicked(this,
                                         &SRustealCreateProject::OnClose)]]]];
}

void SRustealCreateProject::Create() {
  if (Running == ERun::None && IsReady()) {
    Lines.Reset();
    Run(ERun::Create, Rusteal, RustealLauncherCli::NewProjectArguments(Stub));
  }
}

void SRustealCreateProject::Check() {
  Cargo = RustealLauncherCli::FindCargo();
  CargoVersion = Cargo.IsEmpty()
                     ? FString()
                     : RustealLauncherCli::ReadVersion(Cargo).Get(FString());

  Rusteal = RustealLauncherCli::FindRusteal();
  RustealVersion =
      Rusteal.IsEmpty()
          ? FString()
          : RustealLauncherCli::ReadVersion(Rusteal).Get(TEXT("unknown"));

  bRustealCreatesFromStubs =
      !Rusteal.IsEmpty() && RustealLauncherCli::CreatesFromStubs(Rusteal);

  UE_LOG(LogRustealLauncher, Display, TEXT("Rust: %s; Rusteal: %s%s"),
         Cargo.IsEmpty() ? TEXT("not found") : *(TEXT("cargo ") + CargoVersion),
         Rusteal.IsEmpty() ? TEXT("not found") : *RustealVersion,
         Rusteal.IsEmpty() || bRustealCreatesFromStubs
             ? TEXT("")
             : TEXT(" (too old for the Rust templates)"));

  if (!bValidName) {
    Status = FText::Format(
        LOCTEXT("InvalidName",
                "{0} cannot be the name of a Rust project: it takes letters "
                "and digits only, starting with a letter, 20 characters at "
                "most. Close this project and create it again from the New "
                "Project dialog with another name."),
        FText::FromString(FApp::GetProjectName()));
  } else if (Cargo.IsEmpty()) {
    Status = LOCTEXT("NeedRust",
                     "Rust is not installed. Install it with Get Rust, then "
                     "press Check again.");
  } else if (Rusteal.IsEmpty()) {
    Status = LOCTEXT("NeedRusteal", "The Rusteal CLI is not installed: "
                                    "Install Rusteal installs it with cargo.");
  } else if (!bRustealCreatesFromStubs) {
    Status = LOCTEXT("OldRusteal",
                     "This Rusteal CLI is too old to set up a project from a "
                     "Rust template: Update Rusteal updates it with cargo.");
  } else {
    Status = FText::GetEmpty();
  }

  if (!bValidName) {
    UE_LOG(LogRustealLauncher, Warning, TEXT("%s"), *Status.ToString());
  }
}

bool SRustealCreateProject::IsReady() const {
  return bValidName && !Cargo.IsEmpty() && bRustealCreatesFromStubs;
}

void SRustealCreateProject::Run(ERun Kind, const FString &Executable,
                                const FString &Arguments) {
  RustealLauncherCli::AddCargoBinToPath();
  bFailed = false;
  AppendLine(FString::Printf(TEXT("$ %s %s"), *Executable, *Arguments));

  Process = MakeUnique<FRustealLauncherProcess>();
  Process->OnLine.BindSP(this, &SRustealCreateProject::AppendLine);
  Process->OnExit.BindSP(this, &SRustealCreateProject::OnFinished);

  if (!Process->Launch(Executable, Arguments,
                       RustealLauncherCli::ProjectParentDir())) {
    Running = Kind;
    OnFinished(-1);
    return;
  }

  Running = Kind;

  Status = Kind == ERun::Install
               ? LOCTEXT("Installing", "Installing the Rusteal CLI with cargo; "
                                       "this takes a few minutes.")
               : LOCTEXT("Building", "Setting the project up and building it; "
                                     "this takes a few minutes.");
}

void SRustealCreateProject::AppendLine(const FString &Line) {
  UE_LOG(LogRustealLauncher, Log, TEXT("%s"), *Line);
  Lines.Add(Line);

  if (Lines.Num() > MaxLogLines) {
    Lines.RemoveAt(0, Lines.Num() - MaxLogLines);
  }

  if (LogScroll.IsValid()) {
    LogScroll->ScrollToEnd();
  }
}

void SRustealCreateProject::OnFinished(int32 ReturnCode) {
  const ERun Finished = Running;
  Running = ERun::None;

  UE_LOG(LogRustealLauncher, Display, TEXT("%s finished with exit code %d"),
         Finished == ERun::Install ? TEXT("cargo install")
                                   : TEXT("rusteal new"),
         ReturnCode);

  if (Finished == ERun::Install) {
    Check();

    if (ReturnCode == 0 && IsReady()) {
      Create();
      return;
    }

    if (ReturnCode != 0) {
      bFailed = true;

      Status = FText::Format(
          LOCTEXT("InstallFailed", "cargo stopped with exit code {0}; the log "
                                   "above says why."),
          FText::AsNumber(ReturnCode));
    }

    return;
  }

  if (ReturnCode == 0) {
    Status = LOCTEXT("Done", "The project is ready. Restarting the editor...");
    FUnrealEdMisc::Get().RestartEditor(false);
    return;
  }

  bFailed = true;
  RestoreStub();

  Status = FText::Format(
      LOCTEXT("Failed",
              "Rusteal stopped with exit code {0}; the log above says why. "
              "The project was left as it was: press Retry, or open the "
              "project again to start over."),
      FText::AsNumber(ReturnCode));
}

SRustealCreateProject::~SRustealCreateProject() {
  if (Running == ERun::Create && Process.IsValid()) {
    Process->Cancel();
    RestoreStub();
  }
}

void SRustealCreateProject::RestoreStub() {
  if (!StubProject.IsEmpty()) {
    FFileHelper::SaveStringToFile(StubProject, *FPaths::GetProjectFilePath());
  }

  for (const FString &Directory : CodeDirectoriesToRemove) {
    IFileManager::Get().DeleteDirectory(*Directory, false, true);
  }
}

FReply SRustealCreateProject::OnGetRust() {
  FPlatformProcess::LaunchURL(TEXT("https://rustup.rs"), nullptr, nullptr);
  return FReply::Handled();
}

FReply SRustealCreateProject::OnInstallRusteal() {
  Lines.Reset();
  Run(ERun::Install, Cargo, TEXT("install rusteal --locked"));
  return FReply::Handled();
}

FReply SRustealCreateProject::OnCheckAgain() {
  Check();
  Create();
  return FReply::Handled();
}

FReply SRustealCreateProject::OnRetry() {
  Check();
  Create();
  return FReply::Handled();
}

FReply SRustealCreateProject::OnCancel() {
  const ERun Canceled = Running;

  if (Process.IsValid()) {
    Process->Cancel();
  }

  Running = ERun::None;
  bFailed = true;
  AppendLine(TEXT("Canceled."));
  UE_LOG(LogRustealLauncher, Display, TEXT("Canceled"));

  if (Canceled == ERun::Create) {
    RestoreStub();

    Status = LOCTEXT("CanceledCreate",
                     "Canceled. The project was left as it was: press Retry, "
                     "or open the project again to start over.");
  } else {
    Check();
  }

  return FReply::Handled();
}

FReply SRustealCreateProject::OnClose() {
  if (Running != ERun::None) {
    OnCancel();
  }

  if (TSharedPtr<SWindow> Window =
          FSlateApplication::Get().FindWidgetWindow(AsShared())) {
    Window->RequestDestroyWindow();
  }

  return FReply::Handled();
}

FText SRustealCreateProject::GetRustState() const {
  return Cargo.IsEmpty() ? LOCTEXT("NoCargo", "Not found")
                         : FText::FromString(TEXT("cargo ") + CargoVersion);
}

FText SRustealCreateProject::GetRustealState() const {
  if (Rusteal.IsEmpty()) {
    return LOCTEXT("NoCli", "Not found");
  }

  if (!bRustealCreatesFromStubs) {
    return FText::Format(LOCTEXT("TooOld", "{0}, too old for Rust templates"),
                         FText::FromString(RustealVersion));
  }

  return FText::Format(LOCTEXT("Found", "{0} at {1}"),
                       FText::FromString(RustealVersion),
                       FText::FromString(Rusteal));
}

FText SRustealCreateProject::GetInstallLabel() const {
  return Rusteal.IsEmpty() ? LOCTEXT("Install", "Install Rusteal")
                           : LOCTEXT("Update", "Update Rusteal");
}

FText SRustealCreateProject::GetLog() const {
  return FText::FromString(FString::Join(Lines, TEXT("\n")));
}

FText SRustealCreateProject::GetStatus() const { return Status; }

EVisibility SRustealCreateProject::VisibleWhen(bool bVisible) const {
  return bVisible ? EVisibility::Visible : EVisibility::Collapsed;
}

#undef LOCTEXT_NAMESPACE
