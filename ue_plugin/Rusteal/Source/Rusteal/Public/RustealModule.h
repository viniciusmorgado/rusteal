#pragma once

#include "Modules/ModuleInterface.h"

DECLARE_LOG_CATEGORY_EXTERN(LogRusteal, Log, All);

class FRustealModule : public IModuleInterface {
public:
  virtual void StartupModule() override;

  virtual void ShutdownModule() override;
};
