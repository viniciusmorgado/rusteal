#include "Engine/World.h"
#include "HAL/IConsoleManager.h"
#include "RustealApiTable.h"
#include "RustealLibrary.h"
#include "RustealModule.h"

static TMap<FRustealLibrary *, TArray<FString>> GConsoleNames;

static FString ConsoleUtf8(const uint8 *Text, uint32 Len) {
  return Text
             ? FString(Len, UTF8_TO_TCHAR(reinterpret_cast<const char *>(Text)))
             : FString();
}

static void Remember(const FString &Name) {
  GConsoleNames.FindOrAdd(RustealCurrentLibrary()).AddUnique(Name);
}

static ERustealErrorCode RegisterCommandImpl(const uint8 *Name, uint32 NameLen,
                                             const uint8 *Help, uint32 HelpLen,
                                             uint64 CallbackId) {
  const FString CommandName = ConsoleUtf8(Name, NameLen);

  if (CommandName.IsEmpty()) {
    return ERustealErrorCode::NullArgument;
  }

  if (IConsoleManager::Get().FindConsoleObject(*CommandName)) {
    UE_LOG(LogRusteal, Error,
           TEXT("[Rusteal] Console: %s is already a command or variable"),
           *CommandName);

    return ERustealErrorCode::InvalidOperation;
  }

  FRustealLibrary *Library = RustealCurrentLibrary();

  IConsoleManager::Get().RegisterConsoleCommand(
      *CommandName, *ConsoleUtf8(Help, HelpLen),
      FConsoleCommandWithWorldAndArgsDelegate::CreateLambda(
          [Library, CallbackId](const TArray<FString> &Args, UWorld *World) {
            const FTCHARToUTF8 Utf8(*FString::Join(Args, TEXT("\n")));
            FRustealConsoleArgs Params;
            Params.args = reinterpret_cast<const uint8 *>(Utf8.Get());
            Params.args_len = static_cast<uint32>(Utf8.Length());
            Params.world = RustealUObjectHandle{World};

            RustealCallLibrary(Library, [CallbackId, &Params](
                                            const FRustealRustCallbacks &Cb) {
              Cb.invoke_delegate_callback(CallbackId,
                                          reinterpret_cast<uint8 *>(&Params));
            });
          }),
      ECVF_Default);

  Remember(CommandName);
  return ERustealErrorCode::Ok;
}

static ERustealErrorCode RegisterVariableImpl(const uint8 *Name, uint32 NameLen,
                                              const uint8 *Help, uint32 HelpLen,
                                              uint32 Kind, const uint8 *Default,
                                              uint32 DefaultLen) {
  const FString VariableName = ConsoleUtf8(Name, NameLen);

  if (VariableName.IsEmpty()) {
    return ERustealErrorCode::NullArgument;
  }

  IConsoleManager &Manager = IConsoleManager::Get();

  if (Manager.FindConsoleObject(*VariableName)) {
    UE_LOG(LogRusteal, Error,
           TEXT("[Rusteal] Console: %s is already a command or variable"),
           *VariableName);

    return ERustealErrorCode::InvalidOperation;
  }

  const FString HelpText = ConsoleUtf8(Help, HelpLen);
  const FString DefaultText = ConsoleUtf8(Default, DefaultLen);
  IConsoleVariable *Variable = nullptr;

  switch (Kind) {
  case 0:
    Variable = Manager.RegisterConsoleVariable(
        *VariableName, DefaultText.ToBool(), *HelpText, ECVF_Default);
    break;
  case 1:
    Variable = Manager.RegisterConsoleVariable(
        *VariableName, FCString::Atoi(*DefaultText), *HelpText, ECVF_Default);
    break;
  case 2:
    Variable = Manager.RegisterConsoleVariable(
        *VariableName, FCString::Atof(*DefaultText), *HelpText, ECVF_Default);
    break;
  case 3:
    Variable = Manager.RegisterConsoleVariable(*VariableName, DefaultText,
                                               *HelpText, ECVF_Default);
    break;
  default:
    return ERustealErrorCode::TypeMismatch;
  }

  if (!Variable) {
    return ERustealErrorCode::InternalError;
  }

  Remember(VariableName);
  return ERustealErrorCode::Ok;
}

static ERustealErrorCode UnregisterImpl(const uint8 *Name, uint32 NameLen) {
  const FString ObjectName = ConsoleUtf8(Name, NameLen);
  TArray<FString> *Names = GConsoleNames.Find(RustealCurrentLibrary());

  if (!Names || Names->Remove(ObjectName) == 0) {
    return ERustealErrorCode::InvalidOperation;
  }

  if (IConsoleObject *Object =
          IConsoleManager::Get().FindConsoleObject(*ObjectName)) {
    IConsoleManager::Get().UnregisterConsoleObject(Object, false);
  }

  return ERustealErrorCode::Ok;
}

static ERustealErrorCode GetVariableImpl(const uint8 *Name, uint32 NameLen,
                                         uint8 *Buf, uint32 BufLen,
                                         uint32 *OutLen) {
  IConsoleVariable *Variable =
      IConsoleManager::Get().FindConsoleVariable(*ConsoleUtf8(Name, NameLen));

  if (!Variable) {
    return ERustealErrorCode::PropertyNotFound;
  }

  const FTCHARToUTF8 Utf8(*Variable->GetString());
  const uint32 Len = static_cast<uint32>(Utf8.Length());

  if (OutLen) {
    *OutLen = Len;
  }

  if (Buf && BufLen > 0) {
    FMemory::Memcpy(Buf, Utf8.Get(), FMath::Min(Len, BufLen));
  }

  return ERustealErrorCode::Ok;
}

static ERustealErrorCode SetVariableImpl(const uint8 *Name, uint32 NameLen,
                                         const uint8 *Value, uint32 ValueLen) {
  IConsoleVariable *Variable =
      IConsoleManager::Get().FindConsoleVariable(*ConsoleUtf8(Name, NameLen));

  if (!Variable) {
    return ERustealErrorCode::PropertyNotFound;
  }

  Variable->Set(*ConsoleUtf8(Value, ValueLen), ECVF_SetByCode);
  return ERustealErrorCode::Ok;
}

void RustealConsoleForgetLibrary(FRustealLibrary *Library) {
  TArray<FString> Names;

  if (!GConsoleNames.RemoveAndCopyValue(Library, Names)) {
    return;
  }

  for (const FString &Name : Names) {
    if (IConsoleObject *Object =
            IConsoleManager::Get().FindConsoleObject(*Name)) {
      IConsoleManager::Get().UnregisterConsoleObject(Object, false);
    }
  }
}

FRustealConsoleApi GConsoleApi = {
    &RegisterCommandImpl, &RegisterVariableImpl, &UnregisterImpl,
    &GetVariableImpl,     &SetVariableImpl,
};
