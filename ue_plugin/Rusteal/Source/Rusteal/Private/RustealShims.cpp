#include "RustealShims.h"

void ARustealActor::PreInitializeComponents() {
  Super::PreInitializeComponents();
  ReceivePreInitializeComponents();
}

void ARustealActor::PostInitializeComponents() {
  Super::PostInitializeComponents();
  ReceivePostInitializeComponents();
}
