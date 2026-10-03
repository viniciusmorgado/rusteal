// RustealInputApiImpl.cpp — FRustealInputApi implementation.
// Binds Enhanced Input actions to UFUNCTIONs: UEnhancedInputComponent's
// BindAction is a C++ template, not in reflection. Also answers whether the
// platform shows touch controls, which only Slate's SVirtualJoystick knows.

#include "EnhancedInputComponent.h"
#include "GameFramework/Actor.h"
#include "InputAction.h"
#include "InputActionValue.h"
#include "RustealApiTable.h"
#include "RustealModule.h"
#include "Widgets/Input/SVirtualJoystick.h"

// ---------------------------------------------------------------------------
// Implementations
// ---------------------------------------------------------------------------

// The bindings made so far, so binding the same function to the same action
// and event on the same component again is a no-op: ReceiveRestarted, where
// Rust binds, can fire more than once for one input component.
using FRustealActionBinding =
    TTuple<TWeakObjectPtr<UEnhancedInputComponent>,
           TWeakObjectPtr<UInputAction>, uint8, FName>;
static TSet<FRustealActionBinding> GActionBindings;

// Record a binding; false when it was already made. Drops the entries of
// components that are gone.
static bool AddActionBinding(const FRustealActionBinding &Binding) {
  for (auto It = GActionBindings.CreateIterator(); It; ++It) {
    if (!It->Get<0>().IsValid()) {
      It.RemoveCurrent();
    }
  }
  bool bAlreadyBound = false;
  GActionBindings.Add(Binding, &bAlreadyBound);
  return !bAlreadyBound;
}

// The parameters a handler may take, in this order and any leading part of
// it, as the engine's dynamic binding signature has them: the action's value,
// the seconds it has been evaluated (ElapsedSeconds) and triggered
// (TriggeredSeconds), and the action itself.
struct FRustealActionHandlerParams {
  FStructProperty *Value = nullptr;
  FFloatProperty *ElapsedSeconds = nullptr;
  FFloatProperty *TriggeredSeconds = nullptr;
  FObjectProperty *SourceAction = nullptr;
};

// Match Func's parameters against that signature; false for any other.
static bool ActionHandlerParams(const UFunction *Func,
                                FRustealActionHandlerParams &Out) {
  Out = FRustealActionHandlerParams();
  int32 Index = 0;
  for (TFieldIterator<FProperty> It(Func); It && It->HasAnyPropertyFlags(CPF_Parm);
       ++It, ++Index) {
    FProperty *Param = *It;
    if (Param->HasAnyPropertyFlags(CPF_ReturnParm)) {
      return false;
    }
    switch (Index) {
    case 0: {
      FStructProperty *Value = CastField<FStructProperty>(Param);
      if (!Value || Value->Struct != FInputActionValue::StaticStruct()) {
        return false;
      }
      Out.Value = Value;
      break;
    }
    case 1:
      Out.ElapsedSeconds = CastField<FFloatProperty>(Param);
      if (!Out.ElapsedSeconds) {
        return false;
      }
      break;
    case 2:
      Out.TriggeredSeconds = CastField<FFloatProperty>(Param);
      if (!Out.TriggeredSeconds) {
        return false;
      }
      break;
    case 3: {
      FObjectProperty *Source = CastField<FObjectProperty>(Param);
      if (!Source || !UInputAction::StaticClass()->IsChildOf(Source->PropertyClass)) {
        return false;
      }
      Out.SourceAction = Source;
      break;
    }
    default:
      return false;
    }
  }
  return true;
}

static ERustealErrorCode BindActionImpl(RustealUObjectHandle ActorHandle,
                                        RustealUObjectHandle ActionHandle,
                                        uint8 TriggerEvent,
                                        const uint8 *FunctionName,
                                        uint32 FunctionNameLen) {
  AActor *Actor = Cast<AActor>(static_cast<UObject *>(ActorHandle.ptr));
  UInputAction *Action =
      Cast<UInputAction>(static_cast<UObject *>(ActionHandle.ptr));
  if (!IsValid(Actor) || !IsValid(Action) || !FunctionName) {
    return ERustealErrorCode::NullArgument;
  }

  UEnhancedInputComponent *Input =
      Cast<UEnhancedInputComponent>(Actor->InputComponent);
  if (!Input) {
    UE_LOG(LogRusteal, Warning,
           TEXT("[Rusteal] BindAction: %s has no Enhanced Input component"),
           *Actor->GetName());
    return ERustealErrorCode::InvalidOperation;
  }

  // The name is not null-terminated: convert exactly FunctionNameLen bytes.
  const FUTF8ToTCHAR NameChars(reinterpret_cast<const ANSICHAR *>(FunctionName),
                               FunctionNameLen);
  const FName Name(NameChars.Length(), NameChars.Get());
  UFunction *Func = Actor->FindFunction(Name);
  if (!Func) {
    UE_LOG(LogRusteal, Warning,
           TEXT("[Rusteal] BindAction: %s has no function %s"),
           *Actor->GetName(), *Name.ToString());
    return ERustealErrorCode::FunctionNotFound;
  }
  FRustealActionHandlerParams Params;
  if (!ActionHandlerParams(Func, Params)) {
    UE_LOG(LogRusteal, Warning,
           TEXT("[Rusteal] BindAction: %s::%s must take (FInputActionValue, "
                "float ElapsedSeconds, float TriggeredSeconds, UInputAction*) "
                "or a leading part of it"),
           *Actor->GetName(), *Name.ToString());
    return ERustealErrorCode::TypeMismatch;
  }

  if (!AddActionBinding(
          FRustealActionBinding(Input, Action, TriggerEvent, Name))) {
    return ERustealErrorCode::Ok;
  }

  // The function is looked up again on each call, so a hot reload that
  // replaces it is picked up; the binding goes away with the component.
  TWeakObjectPtr<AActor> WeakActor(Actor);
  Input->BindActionInstanceLambda(
      Action, static_cast<ETriggerEvent>(TriggerEvent),
      [WeakActor, Name](const FInputActionInstance &Instance) {
        AActor *Target = WeakActor.Get();
        UFunction *Handler = Target ? Target->FindFunction(Name) : nullptr;
        if (!Handler) {
          return;
        }
        FRustealActionHandlerParams HandlerParams;
        if (!ActionHandlerParams(Handler, HandlerParams)) {
          return;
        }
        if (!HandlerParams.Value) {
          Target->ProcessEvent(Handler, nullptr);
          return;
        }
        uint8 *Params =
            static_cast<uint8 *>(FMemory_Alloca(Handler->ParmsSize));
        FMemory::Memzero(Params, Handler->ParmsSize);
        FStructProperty *ValueParam = HandlerParams.Value;
        ValueParam->InitializeValue_InContainer(Params);
        const FInputActionValue Value = Instance.GetValue();
        ValueParam->CopyCompleteValue(
            ValueParam->ContainerPtrToValuePtr<void>(Params), &Value);
        if (HandlerParams.ElapsedSeconds) {
          HandlerParams.ElapsedSeconds->SetPropertyValue_InContainer(
              Params, Instance.GetElapsedTime());
        }
        if (HandlerParams.TriggeredSeconds) {
          HandlerParams.TriggeredSeconds->SetPropertyValue_InContainer(
              Params, Instance.GetTriggeredTime());
        }
        if (HandlerParams.SourceAction) {
          HandlerParams.SourceAction->SetObjectPropertyValue_InContainer(
              Params, const_cast<UInputAction *>(Instance.GetSourceAction().Get()));
        }
        Target->ProcessEvent(Handler, Params);
        ValueParam->DestroyValue_InContainer(Params);
      });
  return ERustealErrorCode::Ok;
}

static bool ShouldDisplayTouchInterfaceImpl() {
  return SVirtualJoystick::ShouldDisplayTouchInterface();
}

// ---------------------------------------------------------------------------
// Static instance
// ---------------------------------------------------------------------------

FRustealInputApi GInputApi = {
    &BindActionImpl,
    &ShouldDisplayTouchInterfaceImpl,
};
