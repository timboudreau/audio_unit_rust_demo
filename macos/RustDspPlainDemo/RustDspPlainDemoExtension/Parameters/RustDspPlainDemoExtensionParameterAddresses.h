#pragma once

#include <AudioToolbox/AUParameters.h>

#ifdef __cplusplus
namespace RustDspPlainDemoExtensionParameterAddress {
#endif

typedef NS_ENUM(AUParameterAddress, RustDspPlainDemoExtensionParameterAddress) {
    gain = 0,
    pan = 1
};

#ifdef __cplusplus
}
#endif
