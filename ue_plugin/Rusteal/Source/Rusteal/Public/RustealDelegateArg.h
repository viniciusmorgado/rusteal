#pragma once

#include "CoreMinimal.h"
#include <type_traits>

struct FRustealDelegateArg {
  const void *Delegate;

  template <typename DelegateType,
            std::enable_if_t<std::is_base_of_v<FScriptDelegate, DelegateType>,
                             int> = 0>
  operator DelegateType() const {
    return *static_cast<const DelegateType *>(Delegate);
  }
};
