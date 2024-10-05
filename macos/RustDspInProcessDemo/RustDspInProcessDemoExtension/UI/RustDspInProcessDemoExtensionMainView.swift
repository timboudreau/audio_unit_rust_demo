import SwiftUI

struct RustDspInProcessDemoExtensionMainView: View {

    let inProcess : Bool;
    let gain : ObservableAUParameter;
    let pan : ObservableAUParameter;

    init (parameterTree: ObservableAUParameterGroup, audioUnit: RustDspInProcessDemoExtensionAudioUnit) {
        inProcess = audioUnit.isLoadedInProcess
        gain = parameterTree.global.gain;
        pan = parameterTree.global.pan;
    }

    var body: some View {
        let ptext = inProcess ? "in-process" : "out-of-process";
        VStack {
            ParameterSlider(param: gain)
            ParameterSlider(param: pan)
            Text(ptext)
        }
    }
}
