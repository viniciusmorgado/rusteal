#pragma once

#include "CoreMinimal.h"
#include "Factories/Factory.h"
#include "RustealFactory.generated.h"

UCLASS(Abstract, Blueprintable)
class RUSTEALEDITOR_API URustealFactory : public UFactory {
  GENERATED_BODY()

public:
  virtual UObject *FactoryCreateNew(UClass *InClass, UObject *InParent,
                                    FName InName, EObjectFlags Flags,
                                    UObject *Context,
                                    FFeedbackContext *Warn) override;

  virtual UObject *FactoryCreateFile(UClass *InClass, UObject *InParent,
                                     FName InName, EObjectFlags Flags,
                                     const FString &Filename,
                                     const TCHAR *Parms, FFeedbackContext *Warn,
                                     bool &bOutOperationCanceled) override;

  virtual bool FactoryCanImport(const FString &Filename) override;
  virtual bool ShouldShowInNewMenu() const override;
  virtual FText GetDisplayName() const override;

  UFUNCTION(BlueprintImplementableEvent, Category = "Rusteal|Factory")
  void ReceiveCreatedNew(UObject *NewObject, UObject *Context);

  UFUNCTION(BlueprintNativeEvent, Category = "Rusteal|Factory")
  bool ReceiveImportFile(UObject *NewObject, const FString &Filename);

  UFUNCTION(BlueprintNativeEvent, Category = "Rusteal|Factory")
  bool ReceiveCanImport(const FString &Filename) const;

  UFUNCTION(BlueprintNativeEvent, Category = "Rusteal|Factory")
  bool ReceiveShouldShowInNewMenu() const;

  UFUNCTION(BlueprintNativeEvent, Category = "Rusteal|Factory")
  FText ReceiveGetDisplayName() const;
};
