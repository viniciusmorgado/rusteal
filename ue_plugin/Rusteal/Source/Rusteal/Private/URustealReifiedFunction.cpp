#include "URustealReifiedFunction.h"
#include "Blueprint/BlueprintExceptionInfo.h"
#include "RustealApiTable.h"
#include "RustealErrors.h"
#include "UObject/Script.h"
#include "RustealLibrary.h"
#include "RustealModule.h"
#include "URustealReifiedClass.h"

DEFINE_FUNCTION(URustealReifiedFunction::execCallRustFunction) {
  URustealReifiedFunction *ReifiedFunc = nullptr;

  if (Stack.CurrentNativeFunction) {
    ReifiedFunc = Cast<URustealReifiedFunction>(Stack.CurrentNativeFunction);
  }

  if (!ReifiedFunc) {
    ReifiedFunc = Cast<URustealReifiedFunction>(Stack.Node);
  }

  if (!ReifiedFunc) {
    UFunction *StartFunc =
        Stack.CurrentNativeFunction ? Stack.CurrentNativeFunction : Stack.Node;

    for (UFunction *F = StartFunc ? StartFunc->GetSuperFunction() : nullptr; F;
         F = F->GetSuperFunction()) {
      ReifiedFunc = Cast<URustealReifiedFunction>(F);

      if (ReifiedFunc)
        break;
    }
  }

  if (!ReifiedFunc && P_THIS) {
    UFunction *LookupFunc =
        Stack.CurrentNativeFunction ? Stack.CurrentNativeFunction : Stack.Node;

    if (LookupFunc) {
      FName FuncName = LookupFunc->GetFName();

      for (UClass *Cls = P_THIS->GetClass(); Cls; Cls = Cls->GetSuperClass()) {
        if (URustealReifiedClass *RC = Cast<URustealReifiedClass>(Cls)) {
          if (UFunction *Found = RC->FindFunctionByName(
                  FuncName, EIncludeSuperFlag::ExcludeSuper)) {
            ReifiedFunc = Cast<URustealReifiedFunction>(Found);

            if (ReifiedFunc)
              break;
          }
        }
      }
    }
  }

  if (!ReifiedFunc) {
    UE_LOG(LogRusteal, Error,
           TEXT("[Rusteal] execCallRustFunction: cannot find "
                "URustealReifiedFunction")
               TEXT(" (Node='%s', CurrentNative='%s')"),
           Stack.Node ? *Stack.Node->GetName() : TEXT("(null)"),
           Stack.CurrentNativeFunction ? *Stack.CurrentNativeFunction->GetName()
                                       : TEXT("(null)"));

    P_FINISH;
    return;
  }

  const bool bFromProcessEvent = (Stack.Node == ReifiedFunc);
  uint8 *ParamsPtr = nullptr;

  TArray<TPair<FProperty *, uint8 *>, TInlineAllocator<4>> OutDestinations;

  if (bFromProcessEvent) {
    P_FINISH;
    ParamsPtr = Stack.Locals;

    for (FOutParmRec *Out = Stack.OutParms; Out; Out = Out->NextOutParm) {
      if (Out->Property &&
          !Out->Property->HasAnyPropertyFlags(CPF_ReturnParm | CPF_ConstParm)) {
        OutDestinations.Emplace(Out->Property, Out->PropAddr);
      }
    }
  } else {
    const int32 PropsSize = ReifiedFunc->PropertiesSize;

    if (PropsSize > 0) {
      ParamsPtr = (uint8 *)FMemory_Alloca(PropsSize);
      FMemory::Memzero(ParamsPtr, PropsSize);

      for (FField *Field = ReifiedFunc->ChildProperties; Field;
           Field = Field->Next) {
        FProperty *Prop = CastField<FProperty>(Field);

        if (!Prop)
          continue;

        if (Prop->HasAnyPropertyFlags(CPF_ReturnParm))
          continue;

        if (!Prop->HasAnyPropertyFlags(CPF_Parm))
          continue;

        Stack.MostRecentPropertyAddress = nullptr;
        Stack.Step(Stack.Object, ParamsPtr + Prop->GetOffset_ForUFunction());

        if (Prop->HasAnyPropertyFlags(CPF_OutParm) &&
            !Prop->HasAnyPropertyFlags(CPF_ConstParm) &&
            Stack.MostRecentPropertyAddress) {
          OutDestinations.Emplace(Prop, Stack.MostRecentPropertyAddress);
        }
      }
    }

    P_FINISH;
  }

  UObject *Self = P_THIS;

  if (!ReifiedFunc->Library || !ReifiedFunc->Library->IsLoaded()) {
    UE_LOG(LogRusteal, Warning,
           TEXT("[Rusteal] %s was called on %s, but the Rust library it came "
                "from is no longer loaded: it does nothing."),
           *ReifiedFunc->GetPathName(), *GetNameSafe(Self));
  }

  RustealTakePendingScriptError();

  RustealCallLibrary(
      ReifiedFunc->Library,
      [ReifiedFunc, Self, ParamsPtr](const FRustealRustCallbacks &Cb) {
        Cb.invoke_rust_function(ReifiedFunc->CallbackId,
                                RustealUObjectHandle{Self}, ParamsPtr);
      });

  const FString ScriptError = RustealTakePendingScriptError();

  if (!ScriptError.IsEmpty() && !bFromProcessEvent) {
    FBlueprintCoreDelegates::ThrowScriptException(
        Self, Stack,
        FBlueprintExceptionInfo(EBlueprintExceptionType::UserRaisedError,
                                FText::FromString(ScriptError)));
  }

  for (const TPair<FProperty *, uint8 *> &Out : OutDestinations) {
    uint8 *Value = ParamsPtr + Out.Key->GetOffset_ForUFunction();

    if (!Out.Value || Out.Value == Value) {
      continue;
    }

    if (bFromProcessEvent) {
      FMemory::Memcpy(Out.Value, Value, Out.Key->GetSize());
    } else {
      Out.Key->CopyCompleteValue(Out.Value, Value);
    }
  }

  if (RESULT_PARAM && ParamsPtr) {
    if (FProperty *RetProp = ReifiedFunc->GetReturnProperty()) {
      uint8 *Value = ParamsPtr + RetProp->GetOffset_ForUFunction();

      if (bFromProcessEvent) {
        if (RESULT_PARAM != Value) {
          FMemory::Memcpy(RESULT_PARAM, Value, RetProp->GetSize());
        }
      } else {
        RetProp->CopyCompleteValue(RESULT_PARAM, Value);
      }
    }
  }

  if (!bFromProcessEvent && ParamsPtr) {
    for (FField *Field = ReifiedFunc->ChildProperties; Field;
         Field = Field->Next) {
      FProperty *Prop = CastField<FProperty>(Field);

      if (Prop && Prop->HasAnyPropertyFlags(CPF_Parm)) {
        Prop->DestroyValue_InContainer(ParamsPtr);
      }
    }
  }
}
