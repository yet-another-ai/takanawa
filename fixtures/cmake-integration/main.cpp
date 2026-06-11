#include <cstddef>

#include "takanawa.h"

int main() {
  TknwGlobalConfig config{
    TKNW_ABI_VERSION,
    sizeof(TknwGlobalConfig),
    0,
  };

  TknwStatus status = tknw_global_init(&config);
  if (status != TKNW_STATUS_OK) {
    return static_cast<int>(status);
  }

  status = tknw_global_shutdown();
  return status == TKNW_STATUS_OK ? 0 : static_cast<int>(status);
}
