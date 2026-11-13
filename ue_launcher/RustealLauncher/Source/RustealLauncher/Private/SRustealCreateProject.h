// Copyright 2026 Vinicius Donatto Morgado.

#pragma once

#include "CoreMinimal.h"
#include "RustealLauncherCli.h"
#include "RustealLauncherProcess.h"
#include "Widgets/SCompoundWidget.h"

class SScrollBox;

class SRustealCreateProject : public SCompoundWidget {
public:
  SLATE_BEGIN_ARGS(SRustealCreateProject) {}
  SLATE_ARGUMENT(FRustealLauncherStub, Stub)
  SLATE_END_ARGS()

  ~SRustealCreateProject();

  void Construct(const FArguments &InArgs);
  void Create();

private:
  enum class ERun { None, Install, Create };

  void Check();
  bool IsReady() const;
  void Run(ERun Kind, const FString &Executable, const FString &Arguments);
  void AppendLine(const FString &Line);
  void OnFinished(int32 ReturnCode);
  void RestoreStub();

  FReply OnGetRust();
  FReply OnInstallRusteal();
  FReply OnCheckAgain();
  FReply OnRetry();
  FReply OnCancel();
  FReply OnClose();

  FText GetRustState() const;
  FText GetRustealState() const;
  FText GetInstallLabel() const;
  FText GetLog() const;
  FText GetStatus() const;
  EVisibility VisibleWhen(bool bVisible) const;

  FRustealLauncherStub Stub;
  FString StubProject;
  TArray<FString> CodeDirectoriesToRemove;
  FString Cargo;
  FString CargoVersion;
  FString Rusteal;
  FString RustealVersion;
  bool bRustealCreatesFromStubs = false;
  bool bValidName = false;
  bool bFailed = false;
  ERun Running = ERun::None;
  FText Status;
  TArray<FString> Lines;
  TSharedPtr<SScrollBox> LogScroll;
  TUniquePtr<FRustealLauncherProcess> Process;
};
