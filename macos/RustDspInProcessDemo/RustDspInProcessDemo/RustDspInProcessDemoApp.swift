import CoreMIDI
import SwiftUI

@main
struct RustDspInProcessDemoApp: App {
    @ObservedObject private var hostModel = AudioUnitHostModel()

    var body: some Scene {
        WindowGroup {
            ContentView(hostModel: self.hostModel)
        }
    }
}
