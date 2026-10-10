#include "RustealDelegateProxy.h"
#include "RustealApiTable.h"
#include "RustealLibrary.h"
#include "RustealModule.h"

FName URustealDelegateProxy::FakeFuncName(TEXT("RustFakeCallable"));

void URustealDelegateProxy::RustFakeCallable() {}

void URustealDelegateProxy::ProcessEvent(UFunction *Function, void *Parms) {
  if (Function->GetFName() != FakeFuncName) {
    Super::ProcessEvent(Function, Parms);
    return;
  }

  if (Library && Library->IsLoaded()) {
    RustealCallLibrary(Library, [this, Parms](const FRustealRustCallbacks &Cb) {
      Cb.invoke_delegate_callback(CallbackId, static_cast<uint8 *>(Parms));
    });
  } else {
    UE_LOG(LogRusteal, Warning,
           TEXT("[Rusteal] DelegateProxy: Rust callbacks not available "
                "(CallbackId=%llu)"),
           CallbackId);
  }
}
