// Copyright 2026 Vinicius Donatto Morgado.

#include "RustealLauncherProcess.h"

#include "HAL/PlatformProcess.h"

FRustealLauncherProcess::~FRustealLauncherProcess() { Cancel(); }

bool FRustealLauncherProcess::Launch(const FString &Executable,
                                     const FString &Arguments,
                                     const FString &WorkingDirectory) {
  if (IsRunning() || !FPlatformProcess::CreatePipe(ReadPipe, WritePipe)) {
    return false;
  }

  Handle = FPlatformProcess::CreateProc(*Executable, *Arguments, false, true,
                                        true, nullptr, 0, *WorkingDirectory,
                                        WritePipe, nullptr, WritePipe);

  if (!Handle.IsValid()) {
    Release();
    return false;
  }

  TickerHandle = FTSTicker::GetCoreTicker().AddTicker(
      FTickerDelegate::CreateRaw(this, &FRustealLauncherProcess::Tick), 0.1f);

  return true;
}

void FRustealLauncherProcess::Cancel() {
  if (!IsRunning()) {
    return;
  }

  FPlatformProcess::TerminateProc(Handle, true);
  Release();
}

bool FRustealLauncherProcess::Tick(float DeltaTime) {
  Drain();

  if (FPlatformProcess::IsProcRunning(Handle)) {
    return true;
  }

  Drain();

  int32 ReturnCode = -1;
  FPlatformProcess::GetProcReturnCode(Handle, &ReturnCode);

  if (!Pending.IsEmpty()) {
    OnLine.ExecuteIfBound(Pending);
    Pending.Reset();
  }

  TickerHandle.Reset();
  Release();
  OnExit.ExecuteIfBound(ReturnCode);
  return false;
}

void FRustealLauncherProcess::Drain() {
  Pending += FPlatformProcess::ReadPipe(ReadPipe);

  int32 Newline = INDEX_NONE;

  while (Pending.FindChar(TEXT('\n'), Newline)) {
    FString Line = Pending.Left(Newline);
    Line.RemoveFromEnd(TEXT("\r"));
    Pending.RightChopInline(Newline + 1);
    OnLine.ExecuteIfBound(Line);
  }
}

void FRustealLauncherProcess::Release() {
  if (TickerHandle.IsValid()) {
    FTSTicker::GetCoreTicker().RemoveTicker(TickerHandle);
    TickerHandle.Reset();
  }

  if (Handle.IsValid()) {
    FPlatformProcess::CloseProc(Handle);
    Handle.Reset();
  }

  if (ReadPipe || WritePipe) {
    FPlatformProcess::ClosePipe(ReadPipe, WritePipe);
    ReadPipe = nullptr;
    WritePipe = nullptr;
  }
}
