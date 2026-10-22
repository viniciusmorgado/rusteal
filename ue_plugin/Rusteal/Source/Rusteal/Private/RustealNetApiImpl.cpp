#include "RustealApiTable.h"
#include "RustealStrings.h"
#include "UObject/CoreNet.h"

void RustealNet_RpcValidateFailed(const uint8 *Reason, uint32 ReasonLen) {
  RPC_ValidateFailed(*RustealUtf8ToFString(Reason, ReasonLen));
}
