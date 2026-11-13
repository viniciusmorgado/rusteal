#include "RustealErrors.h"
#include "Engine/World.h"

#include "Logging/MessageLog.h"
#include "Logging/TokenizedMessage.h"
#include "Misc/UObjectToken.h"
#include "RustealLibraries.h"
#include "RustealModule.h"
#include "UObject/Stack.h"
#include "UObject/UnrealType.h"
#include "UObject/WeakObjectPtr.h"
#include "RustealStrings.h"

#define LOCTEXT_NAMESPACE "RustealErrors"

struct FRustealLastError {
  ERustealErrorCode Code = ERustealErrorCode::Ok;
  FString Message;
  uint64 Frame = 0;
};

static thread_local FRustealLastError GLastError;
static thread_local FString GPendingScriptError;
static FRustealSourceOpener GSourceOpener;
static double GLastNotify = 0.0;

void RustealSetSourceOpener(FRustealSourceOpener Opener) {
  GSourceOpener = MoveTemp(Opener);
}

ERustealErrorCode RustealFail(ERustealErrorCode Code, const FString &Message) {
  GLastError.Code = Code;
  GLastError.Message = Message;
  GLastError.Frame = GFrameCounter;
  return Code;
}

FString RustealDescribe(const UObject *Object) {
  if (!Object) {
    return TEXT("null");
  }

  return FString::Printf(TEXT("%s (%s)"), *Object->GetName(),
                         *Object->GetClass()->GetName());
}

FString RustealDescribe(const FProperty *Property) {
  if (!Property) {
    return TEXT("no property");
  }

  const UStruct *Owner = Property->GetOwnerStruct();

  return FString::Printf(TEXT("%s (%s of %s)"), *Property->GetName(),
                         *Property->GetClass()->GetName(),
                         Owner ? *Owner->GetName() : TEXT("no struct"));
}

static FString PropertyTypeName(const TCHAR *Type) {
  const FString Name(Type);
  return Name.StartsWith(TEXT("F")) ? Name.RightChop(1) : Name;
}

ERustealErrorCode RustealFailType(const void *Property, const TCHAR *Expected) {
  const FProperty *Prop = static_cast<const FProperty *>(Property);

  if (!Prop) {
    return RustealFail(
        ERustealErrorCode::TypeMismatch,
        FString::Printf(TEXT("no property where a %s was expected"),
                        *PropertyTypeName(Expected)));
  }

  return RustealFail(ERustealErrorCode::TypeMismatch,
                     FString::Printf(TEXT("%s is not a %s"),
                                     *RustealDescribe(Prop),
                                     *PropertyTypeName(Expected)));
}

bool RustealTakeErrorMessage(ERustealErrorCode Code, uint8 *Buf, uint32 BufLen,
                             uint32 *OutLen) {
  if (OutLen) {
    *OutLen = 0;
  }

  if (GLastError.Code != Code || GLastError.Frame != GFrameCounter ||
      GLastError.Message.IsEmpty()) {
    return false;
  }

  const FTCHARToUTF8 Utf8(*GLastError.Message);
  const uint32 Len = static_cast<uint32>(Utf8.Length());

  if (OutLen) {
    *OutLen = Len;
  }

  if (Buf && BufLen > 0) {
    FMemory::Memcpy(Buf, Utf8.Get(), FMath::Min(Len, BufLen));
  }

  if (BufLen >= Len) {
    GLastError = FRustealLastError();
  }

  return true;
}

static FString Utf8String(const uint8 *Text, uint32 Len) {
  return Text && Len > 0 ? RustealUtf8ToFString(Text, Len) : FString();
}

void RustealReport(const FRustealErrorReport *Report) {
  if (!Report) {
    return;
  }

  const FString Summary = Utf8String(Report->summary, Report->summary_len);
  FString Details = Utf8String(Report->details, Report->details_len);
  const FString File = Utf8String(Report->file, Report->file_len);
  const FString ScriptStack = FFrame::GetScriptCallstack(true);

  if (!ScriptStack.IsEmpty()) {
    Details += TEXT("\n") + ScriptStack.TrimEnd();
  }

  switch (Report->level) {
  case 0:
    UE_LOG(LogRusteal, Display, TEXT("[Rusteal] %s"), *Details);
    break;
  case 1:
    UE_LOG(LogRusteal, Warning, TEXT("[Rusteal] %s"), *Details);
    break;
  default:
    UE_LOG(LogRusteal, Error, TEXT("[Rusteal] %s"), *Details);
    break;
  }

  if (Report->script_error) {
    GPendingScriptError = Summary;
  }

  if (!IsInGameThread()) {
    return;
  }

  const EMessageSeverity::Type Severity =
      Report->level == 0   ? EMessageSeverity::Info
      : Report->level == 1 ? EMessageSeverity::Warning
                           : EMessageSeverity::Error;

  TSharedRef<FTokenizedMessage> Message = FTokenizedMessage::Create(Severity);
  Message->AddToken(FTextToken::Create(FText::FromString(Summary)));

  FWeakObjectPtr Weak;
  FMemory::Memcpy(&Weak, &Report->object, sizeof(Report->object));

  if (UObject *Object = Weak.Get()) {
    TSharedRef<FUObjectToken> Token = FUObjectToken::Create(Object);
    const FString EditorPath = UWorld::RemovePIEPrefix(Object->GetPathName());

    if (EditorPath != Object->GetPathName()) {
      Token->OnMessageTokenActivated(FOnMessageTokenActivated::CreateLambda(
          [EditorPath](const TSharedRef<IMessageToken> &Activated) {
            const FOnMessageTokenActivated &Default =
                FUObjectToken::DefaultOnMessageTokenActivated();

            if (StaticCastSharedRef<FUObjectToken>(Activated)
                    ->GetObject()
                    .IsValid()) {
              Default.ExecuteIfBound(Activated);
            } else if (UObject *Counterpart =
                           FindObject<UObject>(nullptr, *EditorPath)) {
              Default.ExecuteIfBound(FUObjectToken::Create(Counterpart));
            }
          }));
    }

    Message->AddToken(Token);
  }

  if (!File.IsEmpty()) {
    const int32 Line = static_cast<int32>(Report->line);
    const int32 Column = static_cast<int32>(Report->column);

    Message->AddToken(FActionToken::Create(
        FText::FromString(FString::Printf(
            TEXT("%s:%d"), *FPaths::GetCleanFilename(File), Line)),
        FText::FromString(
            FString::Printf(TEXT("Open %s:%d:%d"), *File, Line, Column)),
        FOnActionTokenExecuted::CreateLambda([File, Line, Column]() {
          if (GSourceOpener) {
            GSourceOpener(File, Line, Column);
          }
        }),
        false));
  }

  FMessageLog Log(TEXT("Rusteal"));
  Log.AddMessage(Message);

  const double Now = FPlatformTime::Seconds();

  if (Severity == EMessageSeverity::Error && Now - GLastNotify > 5.0) {
    GLastNotify = Now;
    Log.Notify(LOCTEXT("RustError", "Rust reported an error"),
               EMessageSeverity::Error);
  }
}

FString RustealTakePendingScriptError() {
  FString Pending = MoveTemp(GPendingScriptError);
  GPendingScriptError.Reset();
  return Pending;
}

#undef LOCTEXT_NAMESPACE
