// Copyright 2026 Vinicius Donatto Morgado.

#include "RustealLauncherCli.h"

#include "Dom/JsonObject.h"
#include "HAL/PlatformMisc.h"
#include "HAL/PlatformProcess.h"
#include "Misc/App.h"
#include "Misc/FileHelper.h"
#include "Misc/Paths.h"
#include "Serialization/JsonReader.h"
#include "Serialization/JsonSerializer.h"

static const TCHAR *StubMarker = TEXT("RustealLauncher.json");

static FString ExecutableName(const FString &Name) {
#if PLATFORM_WINDOWS
  return Name + TEXT(".exe");
#else
  return Name;
#endif
}

static FString FullDirectory(const FString &Path) {
  FString Full = FPaths::ConvertRelativePathToFull(Path);
  FPaths::NormalizeDirectoryName(Full);
  return Full;
}

static FString Quoted(const FString &Value) {
  return FString::Printf(TEXT("\"%s\""), *Value);
}

TOptional<FRustealLauncherStub>
FRustealLauncherStub::Read(const FString &ProjectDir) {
  FString Text;

  if (!FFileHelper::LoadFileToString(
          Text, *FPaths::Combine(ProjectDir, StubMarker))) {
    return {};
  }

  TSharedPtr<FJsonObject> Json;

  if (!FJsonSerializer::Deserialize(TJsonReaderFactory<>::Create(Text), Json) ||
      !Json.IsValid()) {
    return {};
  }

  bool bStub = false;

  if (!Json->TryGetBoolField(TEXT("stub"), bStub) || !bStub) {
    return {};
  }

  FRustealLauncherStub Stub;

  if (!Json->TryGetStringField(TEXT("template"), Stub.Template)) {
    return {};
  }

  Json->TryGetStringField(TEXT("variant"), Stub.Variant);
  return Stub;
}

FString RustealLauncherCli::CargoBinDir() {
  FString CargoHome = FPlatformMisc::GetEnvironmentVariable(TEXT("CARGO_HOME"));

  if (CargoHome.IsEmpty()) {
    CargoHome =
        FPaths::Combine(FPlatformProcess::UserHomeDir(), TEXT(".cargo"));
  }

  return FullDirectory(FPaths::Combine(CargoHome, TEXT("bin")));
}

FString RustealLauncherCli::FindRusteal() {
  const FString Override =
      FPlatformMisc::GetEnvironmentVariable(TEXT("RUSTEAL_CLI"));

  if (!Override.IsEmpty()) {
    return FPaths::FileExists(Override) ? Override : FString();
  }

  const FString Name = ExecutableName(TEXT("rusteal"));
  TArray<FString> Directories;

  FPlatformMisc::GetEnvironmentVariable(TEXT("PATH"))
      .ParseIntoArray(Directories, FPlatformMisc::GetPathVarDelimiter());

  Directories.Add(CargoBinDir());

  for (const FString &Directory : Directories) {
    const FString Candidate = FPaths::Combine(Directory, Name);

    if (FPaths::FileExists(Candidate)) {
      return Candidate;
    }
  }

  return FString();
}

TOptional<FString> RustealLauncherCli::ReadVersion(const FString &Executable) {
  int32 ReturnCode = -1;
  FString StdOut;
  FString StdErr;

  if (!FPlatformProcess::ExecProcess(*Executable, TEXT("--version"),
                                     &ReturnCode, &StdOut, &StdErr) ||
      ReturnCode != 0) {
    return {};
  }

  TArray<FString> Words;
  StdOut.TrimStartAndEnd().ParseIntoArrayWS(Words);

  if (Words.Num() < 2) {
    return {};
  }

  return Words[1];
}

void RustealLauncherCli::AddCargoBinToPath() {
  const FString CargoBin = CargoBinDir();

  if (!FPaths::DirectoryExists(CargoBin)) {
    return;
  }

  const FString Path = FPlatformMisc::GetEnvironmentVariable(TEXT("PATH"));
  TArray<FString> Directories;
  Path.ParseIntoArray(Directories, FPlatformMisc::GetPathVarDelimiter());

  for (const FString &Directory : Directories) {
    if (FullDirectory(Directory) == CargoBin) {
      return;
    }
  }

  FPlatformMisc::SetEnvironmentVar(
      TEXT("PATH"), *(CargoBin + FPlatformMisc::GetPathVarDelimiter() + Path));
}

FString RustealLauncherCli::ProjectParentDir() {
  return FPaths::GetPath(FullDirectory(FPaths::ProjectDir()));
}

FString
RustealLauncherCli::NewProjectArguments(const FRustealLauncherStub &Stub) {
  FString Arguments = FString::Printf(
      TEXT("new %s --dir %s --template %s --engine %s --in-place"),
      FApp::GetProjectName(), *Quoted(ProjectParentDir()), *Stub.Template,
      *Quoted(FullDirectory(FPaths::RootDir())));

  if (!Stub.Variant.IsEmpty()) {
    Arguments += FString::Printf(TEXT(" --variant %s"), *Stub.Variant);
  }

  return Arguments;
}
