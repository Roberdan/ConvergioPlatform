import AppKit
import SwiftUI

struct TerminalOutputView: NSViewRepresentable {
    let text: String
    let onInput: (Data) -> Void

    func makeCoordinator() -> Coordinator {
        Coordinator(onInput: onInput)
    }

    func makeNSView(context: Context) -> NSScrollView {
        let textView = TerminalTextView()
        textView.onInput = context.coordinator.handleInput
        textView.isEditable = false
        textView.isSelectable = true
        textView.drawsBackground = true
        textView.backgroundColor = NSColor(calibratedWhite: 0.08, alpha: 1)
        textView.textColor = NSColor(calibratedWhite: 0.92, alpha: 1)
        textView.insertionPointColor = .white
        textView.font = .monospacedSystemFont(ofSize: 13, weight: .regular)
        textView.textContainerInset = NSSize(width: 12, height: 14)

        let scrollView = NSScrollView()
        scrollView.drawsBackground = false
        scrollView.hasVerticalScroller = true
        scrollView.borderType = .noBorder
        scrollView.documentView = textView
        return scrollView
    }

    func updateNSView(_ scrollView: NSScrollView, context: Context) {
        guard let textView = scrollView.documentView as? TerminalTextView else { return }
        textView.onInput = context.coordinator.handleInput
        if textView.string != text {
            textView.string = text
            textView.scrollToEndOfDocument(nil)
        }
    }

    final class Coordinator {
        private let onInput: (Data) -> Void

        init(onInput: @escaping (Data) -> Void) {
            self.onInput = onInput
        }

        func handleInput(_ data: Data) {
            onInput(data)
        }
    }
}

private final class TerminalTextView: NSTextView {
    var onInput: (Data) -> Void = { _ in }

    override var acceptsFirstResponder: Bool { true }

    override func keyDown(with event: NSEvent) {
        if event.modifierFlags.contains(.command) {
            super.keyDown(with: event)
            return
        }
        if let data = specialKeyData(for: event) ?? event.characters?.data(using: .utf8) {
            onInput(data)
        }
    }

    override func paste(_ sender: Any?) {
        let text = NSPasteboard.general.string(forType: .string) ?? ""
        if let data = text.data(using: .utf8), !data.isEmpty {
            onInput(data)
        }
    }

    private func specialKeyData(for event: NSEvent) -> Data? {
        switch event.keyCode {
        case 36: return "\r".data(using: .utf8)
        case 48: return "\t".data(using: .utf8)
        case 51: return "\u{7f}".data(using: .utf8)
        case 53: return "\u{1B}".data(using: .utf8)
        case 123: return "\u{1B}[D".data(using: .utf8)
        case 124: return "\u{1B}[C".data(using: .utf8)
        case 125: return "\u{1B}[B".data(using: .utf8)
        case 126: return "\u{1B}[A".data(using: .utf8)
        default: return nil
        }
    }
}
