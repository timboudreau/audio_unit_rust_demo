#pragma once

#import <AudioToolbox/AudioToolbox.h>
#import <algorithm>
#import <vector>
#import <span>
#import <mock_dsp_lib.h>
#import "RustDspInProcessDemoExtensionParameterAddresses.h"

/*
 RustDspPlainDemoExtensionDSPKernel
 As a non-ObjC class, this is safe to use from render thread.
 */
class RustDspInProcessDemoExtensionDSPKernel {
public:
    void initialize(int inputChannelCount, int outputChannelCount, double inSampleRate) {
        mSampleRate = inSampleRate;
        if (processorHandle != 0 ) {
            dispose_mock_dsp(processorHandle);
        }
        processorHandle = create_mock_dsp(&params.gain, &params.pan);
    }

    void deInitialize() {
        if (processorHandle != 0) {
            size_t oldHandle = processorHandle;
            processorHandle = 0;
            dispose_mock_dsp(oldHandle);
        }
    }

    // MARK: - Bypass
    bool isBypassed() {
        return mBypassed;
    }

    void setBypass(bool shouldBypass) {
        mBypassed = shouldBypass;
    }

    // MARK: - Parameter Getter / Setter
    void setParameter(AUParameterAddress address, AUValue value) {
        switch (address) {
            case RustDspInProcessDemoExtensionParameterAddress::gain:
                write_f32_atomic(&params.gain, value);
                break;
            case RustDspInProcessDemoExtensionParameterAddress::pan:
                write_f32_atomic(&params.pan, value);
                break;
        }
    }

    AUValue getParameter(AUParameterAddress address) {
        switch (address) {
            case RustDspInProcessDemoExtensionParameterAddress::gain:
                return read_f32_atomic(&params.gain);
            case RustDspInProcessDemoExtensionParameterAddress::pan:
                return read_f32_atomic(&params.pan);
            default: return 0.f;
        }
    }

    // MARK: - Max Frames
    AUAudioFrameCount maximumFramesToRender() const {
        return mMaxFramesToRender;
    }

    void setMaximumFramesToRender(const AUAudioFrameCount &maxFrames) {
        mMaxFramesToRender = maxFrames;
    }

    // MARK: - Musical Context
    void setMusicalContextBlock(AUHostMusicalContextBlock contextBlock) {
        mMusicalContextBlock = contextBlock;
    }

    /**
     MARK: - Internal Process

     This function does the core siginal processing.
     Do your custom DSP here.
     */
    void process(std::span<float const*> inputBuffers, std::span<float *> outputBuffers, AUEventSampleTime bufferStartTime, AUAudioFrameCount frameCount) {
        /*
         Note: For an Audio Unit with 'n' input channels to 'n' output channels, remove the assert below and
         modify the check in [RustDspPlainDemoExtensionAudioUnit allocateRenderResourcesAndReturnError]
         */
        assert(inputBuffers.size() == outputBuffers.size());

        if (mBypassed || processorHandle == 0) {
            // Pass the samples through
            for (UInt32 channel = 0; channel < inputBuffers.size(); ++channel) {
                std::copy_n(inputBuffers[channel], frameCount, outputBuffers[channel]);
            }
            return;
        }

        const float *input_left = inputBuffers[0];
        const float *input_right = inputBuffers[1];
        float *output_left = outputBuffers[0];
        float *output_right = outputBuffers[1];

        mock_dsp_process_stereo(processorHandle, input_left, input_right, (size_t) frameCount,
                                output_left, output_right);
    }

    void handleOneEvent(AUEventSampleTime now, AURenderEvent const *event) {
        switch (event->head.eventType) {
            case AURenderEventParameter: {
                handleParameterEvent(now, event->parameter);
                break;
            }

            default:
                break;
        }
    }

    void handleParameterEvent(AUEventSampleTime now, AUParameterEvent const& parameterEvent) {
        // Xcode 14's c++ stdlib does not have all the atomics we need, so call rust to do this.
        switch(parameterEvent.parameterAddress) {
            case RustDspInProcessDemoExtensionParameterAddress::gain:
                write_f32_atomic(&params.gain, parameterEvent.value);
                break;
            case RustDspInProcessDemoExtensionParameterAddress::pan:
                write_f32_atomic(&params.pan, parameterEvent.value);
                break;
        }
    }

    // MARK: Member Variables
    struct alignas(8) {
        float gain = (float) 1.0;
        float pan = (float) 0.5;
    } params;

    size_t processorHandle = 0;

    AUHostMusicalContextBlock mMusicalContextBlock;

    double mSampleRate = 44100.0;
    bool mBypassed = false;
    AUAudioFrameCount mMaxFramesToRender = 1024;
};
