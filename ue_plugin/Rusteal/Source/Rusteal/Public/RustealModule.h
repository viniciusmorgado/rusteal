#pragma once

#include "Modules/ModuleInterface.h"

DECLARE_LOG_CATEGORY_EXTERN(LogRusteal, Log, All);

class FRustealModule : public IModuleInterface {
public:
  /** Fills the API table and loads the game's library
   * (Plugins/Rusteal/Binaries/<Platform>/librusteal.so). */
  virtual void StartupModule() override;

  /** Unloads every library still loaded. */
  virtual void ShutdownModule() override;
};
