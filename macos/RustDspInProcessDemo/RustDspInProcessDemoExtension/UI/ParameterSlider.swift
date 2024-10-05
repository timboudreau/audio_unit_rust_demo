import SwiftUI

extension Float {
    func toDb() -> Self {
        if self == 0 {
            return Float(-160.0)
        } else {
            return log10(abs(self)) * 20
        }
    }
}

struct ParameterSlider: View {
    @ObservedObject var param: ObservableAUParameter

    func labelFor(_ value : AUValue) -> String {
        switch param.address {
        case RustDspInProcessDemoExtensionParameterAddress.gain.rawValue:
            return String(format: "%.2f dB", value.toDb())
        case RustDspInProcessDemoExtensionParameterAddress.pan.rawValue:
            let ival = Int(value * 100);
            return "\(ival) %"
        default :
            return "\(value)"
        }
    }

    var body: some View {
        VStack {
            Slider(
                value: $param.value,
                in: param.min...param.max,
                onEditingChanged: param.onEditingChanged,
                minimumValueLabel: Text(labelFor(param.min)),
                maximumValueLabel: Text(labelFor(param.max))
            ) {
                EmptyView()
            }
            Text("\(param.displayName): \(labelFor(param.value))")
        }
        .padding()
    }
}

