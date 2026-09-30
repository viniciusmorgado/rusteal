// RustealInputApiImpl.cpp — FRustealInputApi implementation.
// Binds Enhanced Input actions to UFUNCTIONs: UEnhancedInputComponent's
// BindAction is a C++ template, not in reflection.

#include "EnhancedInputComponent.h"
#include "GameFramework/Actor.h"
#include "InputAction.h"
#include "InputActionValue.h"
#include "RustealApiTable.h"
#include "RustealModule.h"

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

// The FInputActionValue parameter of a handler, or null when it takes none.
// Sets bValid to false for any other signature.
static FStructProperty *ActionValueParam(const UFunction *Func, bool &bValid) {
  bValid = true;
  if (Func->NumParms == 0) {
    return nullptr;
  }
  FStructProperty *Param = CastField<FStructProperty>(Func->PropertyLink);
  if (Func->NumParms == 1 && Param &&
      Param->Struct == FInputActionValue::StaticStruct()) {
    return Param;
  }
  bValid = false;
  return nullptr;
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
  bool bValid = false;
  ActionValueParam(Func, bValid);
  if (!bValid) {
    UE_LOG(LogRusteal, Warning,
           TEXT("[Rusteal] BindAction: %s::%s must take no parameters or one "
                "FInputActionValue"),
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
        bool bHandlerValid = false;
        FStructProperty *ValueParam = ActionValueParam(Handler, bHandlerValid);
        if (!bHandlerValid) {
          return;
        }
        if (!ValueParam) {
          Target->ProcessEvent(Handler, nullptr);
          return;
        }
        uint8 *Params =
            static_cast<uint8 *>(FMemory_Alloca(Handler->ParmsSize));
        FMemory::Memzero(Params, Handler->ParmsSize);
        ValueParam->InitializeValue_InContainer(Params);
        const FInputActionValue Value = Instance.GetValue();
        ValueParam->CopyCompleteValue(
            ValueParam->ContainerPtrToValuePtr<void>(Params), &Value);
        Target->ProcessEvent(Handler, Params);
        ValueParam->DestroyValue_InContainer(Params);
      });
  return ERustealErrorCode::Ok;
}

// ---------------------------------------------------------------------------
// Static instance
// ---------------------------------------------------------------------------

FRustealInputApi GInputApi = {
    &BindActionImpl,
};
