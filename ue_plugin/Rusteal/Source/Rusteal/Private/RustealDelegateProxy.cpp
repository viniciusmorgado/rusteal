#include "RustealDelegateProxy.h"
#include "RustealApiTable.h"
#include "RustealLibrary.h"
#include "RustealModule.h"

// Static member initialization.
FName URustealDelegateProxy::FakeFuncName(TEXT("RustFakeCallable"));

void URustealDelegateProxy::RustFakeCallable() {
  // Empty body. This UFUNCTION exists solely to register the FName
  // "RustFakeCallable" in UE's reflection system, so that
  // BindUFunction(Proxy, FakeFuncName) resolves correctly.
}

void URustealDelegateProxy::ProcessEvent(UFunction *Function, void *Parms) {
  // Normal path: if the function isn't our fake callable, delegate to Super.
  if (Function->GetFName() != FakeFuncName) {
    Super::ProcessEvent(Function, Parms);
    return;
  }

  // Delegate invocation path: forward to Rust.
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
