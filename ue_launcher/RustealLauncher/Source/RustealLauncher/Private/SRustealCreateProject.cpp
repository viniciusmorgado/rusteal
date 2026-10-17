// Copyright 2026 Vinicius Donatto Morgado.

#include "SRustealCreateProject.h"

#include "Framework/Application/SlateApplication.h"
#include "Misc/App.h"
#include "Styling/AppStyle.h"
#include "UnrealEdMisc.h"
#include "Widgets/Images/SThrobber.h"
#include "Widgets/Input/SButton.h"
#include "Widgets/Layout/SBorder.h"
#include "Widgets/Layout/SBox.h"
#include "Widgets/Layout/SScrollBox.h"
#include "Widgets/SBoxPanel.h"
#include "Widgets/SWindow.h"
#include "Widgets/Text/STextBlock.h"

#define LOCTEXT_NAMESPACE "RustealLauncher"

static constexpr int32 MaxLogLines = 400;

void SRustealCreateProject::Construct(const FArguments &InArgs) {
  Stub = InArgs._Stub;
  Rusteal = RustealLauncherCli::FindRusteal();

  if (Rusteal.IsEmpty()) {
    Status = LOCTEXT("NoRusteal",
                     "The Rusteal CLI was not found. Install it with `cargo "
                     "install rusteal`, then open this project again.");
  } else {
    RustealVersion =
        RustealLauncherCli::ReadVersion(Rusteal).Get(TEXT("unknown"));

    Found = FText::Format(LOCTEXT("Ready", "Rusteal {0} at {1}"),
                          FText::FromString(RustealVersion),
                          FText::FromString(Rusteal));

    Status = Found;
  }

  const FText Summary = FText::Format(
      LOCTEXT("Summary",
              "{0} was made from a Rust template. Rusteal is turning it into "
              "a project with its gameplay in Rust, from the {1} template, "
              "and building it; the editor restarts into it when it is done."),
      FText::FromString(FApp::GetProjectName()),
      FText::FromString(Stub.Template));

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
                    8)[SNew(STextBlock).AutoWrapText(true).Text(Summary)] +
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
                     SHorizontalBox::Slot().AutoWidth().VAlign(
                         VAlign_Center)[SNew(SThrobber).Visibility(
                         this, &SRustealCreateProject::GetThrobberVisibility)] +
                     SHorizontalBox::Slot().FillWidth(1.0f) +
                     SHorizontalBox::Slot().AutoWidth().Padding(4, 0)
                         [SNew(SButton)
                              .ButtonStyle(FAppStyle::Get(), "PrimaryButton")
                              .Visibility(this, &SRustealCreateProject::
                                                    GetRetryVisibility)
                              .IsEnabled(this,
                                         &SRustealCreateProject::CanCreate)
                              .Text(LOCTEXT("Retry", "Retry"))
                              .OnClicked(this,
                                         &SRustealCreateProject::OnCreate)] +
                     SHorizontalBox::Slot().AutoWidth().Padding(4, 0)
                         [SNew(SButton)
                              .Text(LOCTEXT("Close", "Close"))
                              .OnClicked(this,
                                         &SRustealCreateProject::OnClose)]]]];
}

void SRustealCreateProject::Create() {
  if (CanCreate()) {
    OnCreate();
  }
}

FReply SRustealCreateProject::OnCreate() {
  RustealLauncherCli::AddCargoBinToPath();
  Lines.Reset();
  bFailed = false;

  const FString Arguments = RustealLauncherCli::NewProjectArguments(Stub);
  AppendLine(FString::Printf(TEXT("$ %s %s"), *Rusteal, *Arguments));

  Process = MakeUnique<FRustealLauncherProcess>();
  Process->OnLine.BindSP(this, &SRustealCreateProject::AppendLine);
  Process->OnExit.BindSP(this, &SRustealCreateProject::OnFinished);

  if (!Process->Launch(Rusteal, Arguments,
                       RustealLauncherCli::ProjectParentDir())) {
    Process.Reset();
    OnFinished(-1);
    return FReply::Handled();
  }

  Status = Found;
  return FReply::Handled();
}

FReply SRustealCreateProject::OnClose() {
  if (Process.IsValid()) {
    Process->Cancel();
    Process.Reset();
  }

  if (TSharedPtr<SWindow> Window =
          FSlateApplication::Get().FindWidgetWindow(AsShared())) {
    Window->RequestDestroyWindow();
  }

  return FReply::Handled();
}

void SRustealCreateProject::AppendLine(const FString &Line) {
  Lines.Add(Line);

  if (Lines.Num() > MaxLogLines) {
    Lines.RemoveAt(0, Lines.Num() - MaxLogLines);
  }

  if (LogScroll.IsValid()) {
    LogScroll->ScrollToEnd();
  }
}

void SRustealCreateProject::OnFinished(int32 ReturnCode) {
  Process.Reset();

  if (ReturnCode == 0) {
    Status = LOCTEXT("Done", "The project is ready. Restarting the editor...");
    FUnrealEdMisc::Get().RestartEditor(false);
    return;
  }

  bFailed = true;

  Status = FText::Format(
      LOCTEXT("Failed",
              "Rusteal stopped with exit code {0}; the log above says why. "
              "The project was left as it was: press Retry, or open the "
              "project again to start over."),
      FText::AsNumber(ReturnCode));
}

bool SRustealCreateProject::CanCreate() const {
  return !Rusteal.IsEmpty() && !Process.IsValid();
}

FText SRustealCreateProject::GetLog() const {
  return FText::FromString(FString::Join(Lines, TEXT("\n")));
}

FText SRustealCreateProject::GetStatus() const { return Status; }

EVisibility SRustealCreateProject::GetRetryVisibility() const {
  return bFailed ? EVisibility::Visible : EVisibility::Collapsed;
}

EVisibility SRustealCreateProject::GetThrobberVisibility() const {
  return Process.IsValid() ? EVisibility::Visible : EVisibility::Hidden;
}

#undef LOCTEXT_NAMESPACE
