import UIKit
import SwiftUI
import Shared

struct ComposeView: UIViewControllerRepresentable {
    func makeUIViewController(context: Context) -> UIViewController {
        MainViewControllerKt.MainViewController()
    }

    func updateUIViewController(_ uiViewController: UIViewController, context: Context) {}
}

struct ContentView: View {
    var body: some View {
        // Compose draws edge to edge and keeps its content in the safe area itself
        // (WindowInsets.safeDrawing), keyboard included; the launch ground fills the
        // window until its first frame.
        ComposeView()
            .ignoresSafeArea()
            .background(Color("LaunchBackground").ignoresSafeArea())
            // A library or unit file opened in Oscyl from Files, Mail or AirDrop.
            .onOpenURL { url in MainViewControllerKt.openFile(url: url) }
    }
}
