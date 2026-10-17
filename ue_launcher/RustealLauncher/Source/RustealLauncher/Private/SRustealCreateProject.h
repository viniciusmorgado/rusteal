// Copyright 2026 Vinicius Donatto Morgado.

#pragma once

#include "CoreMinimal.h"
#include "RustealLauncherCli.h"
#include "RustealLauncherProcess.h"
#include "Widgets/SCompoundWidget.h"

class SScrollBox;
class SWindow;

class SRustealCreateProject : public SCompoundWidget {
public:
  SLATE_BEGIN_ARGS(SRustealCreateProject) {}
  SLATE_ARGUMENT(FRustealLauncherStub, Stub)
  SLATE_END_ARGS()

  void Construct(const FArguments &InArgs);
  void Create();

private:
  FReply OnCreate();
  FReply OnClose();
  void AppendLine(const FString &Line);
  void OnFinished(int32 ReturnCode);
  bool CanCreate() const;
  FText GetLog() const;
  FText GetStatus() const;
  FText GetCreateLabel() const;
  EVisibility GetThrobberVisibility() const;

  FRustealLauncherStub Stub;
  FString Rusteal;
  FString RustealVersion;
  FText Status;
  TArray<FString> Lines;
  bool bFailed = false;
  TSharedPtr<SScrollBox> LogScroll;
  TUniquePtr<FRustealLauncherProcess> Process;
};
