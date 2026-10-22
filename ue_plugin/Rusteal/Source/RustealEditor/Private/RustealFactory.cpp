#include "RustealFactory.h"

UObject *URustealFactory::FactoryCreateNew(UClass *InClass, UObject *InParent,
                                           FName InName, EObjectFlags Flags,
                                           UObject *Context,
                                           FFeedbackContext *Warn) {
  UObject *Created =
      NewObject<UObject>(InParent, InClass, InName, Flags | RF_Transactional);

  ReceiveCreatedNew(Created, Context);
  return Created;
}

UObject *URustealFactory::FactoryCreateFile(UClass *InClass, UObject *InParent,
                                            FName InName, EObjectFlags Flags,
                                            const FString &Filename,
                                            const TCHAR *Parms,
                                            FFeedbackContext *Warn,
                                            bool &bOutOperationCanceled) {
  UObject *Created =
      NewObject<UObject>(InParent, InClass, InName, Flags | RF_Transactional);

  if (!ReceiveImportFile(Created, Filename)) {
    Created->MarkAsGarbage();
    return nullptr;
  }

  return Created;
}

bool URustealFactory::FactoryCanImport(const FString &Filename) {
  return ReceiveCanImport(Filename);
}

bool URustealFactory::ShouldShowInNewMenu() const {
  return ReceiveShouldShowInNewMenu();
}

FText URustealFactory::GetDisplayName() const {
  return ReceiveGetDisplayName();
}

bool URustealFactory::ReceiveImportFile_Implementation(
    UObject *NewObject, const FString &Filename) {
  return false;
}

bool URustealFactory::ReceiveCanImport_Implementation(
    const FString &Filename) const {
  return const_cast<URustealFactory *>(this)->Super::FactoryCanImport(Filename);
}

bool URustealFactory::ReceiveShouldShowInNewMenu_Implementation() const {
  return Super::ShouldShowInNewMenu();
}

FText URustealFactory::ReceiveGetDisplayName_Implementation() const {
  return Super::GetDisplayName();
}
