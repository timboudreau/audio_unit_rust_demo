#pragma once

#include <AudioToolbox/AUParameters.h>

#ifdef __cplusplus
namespace RustDspInProcessDemoExtensionParameterAddress {
#endif

typedef NS_ENUM(AUParameterAddress, RustDspInProcessDemoExtensionParameterAddress) {
    gain = 0,
    pan = 1
};

#ifdef __cplusplus
}
#endif
