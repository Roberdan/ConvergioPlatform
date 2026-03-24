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
        // surfaceSunken (#0a0a0a) — deepest nero profondo background for terminal readability
        textView.backgroundColor = NSColor(
            red: 0x0A / 255.0, green: 0x0A / 255.0, blue: 0x0A / 255.0, alpha: 1.0
        )
        // textPrimary (#fafafa) — near-white for maximum contrast on dark background
        textView.textColor = NSColor(
            red: 0xFA / 255.0, green: 0xFA / 255.0, blue: 0xFA / 255.0, alpha: 1.0
        )
        // gialloFerrari (#FFC72C) — brand cursor color
        textView.insertionPointColor = NSColor(
            red: 0xFF / 255.0, green: 0xC7 / 255.0, blue: 0x2C / 255.0, alpha: 1.0
        )
        textView.font = .monospacedSystemFont(ofSize: 13, weight: .regular)
        textView.textContainerInset = NSSize(width: 16, height: 18)

        let scrollView = NSScrollView()
        scrollView.drawsBackground = false
        scrollView.hasVerticalScroller = true
        scrollView.scrollerStyle = .overlay
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

// MARK: - Cursor blink overlay

/// Blinking cursor indicator rendered in SwiftUI on top of the terminal.
/// Uses gialloFerrari accent with easeInOut repeat for a natural blink cadence.
struct TerminalCursorView: View {
    @State private var cursorOpacity: Double = 1.0

    var body: some View {
        Rectangle()
            .fill(ConvergioTokens.Brand.gialloFerrari)
            .frame(width: 2, height: 16)
            .opacity(cursorOpacity)
            .onAppear {
                withAnimation(
                    .easeInOut(duration: 0.6).repeatForever(autoreverses: true)
                ) {
                    cursorOpacity = 0.0
                }
            }
    }
}

// MARK: - NSTextView subclass

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
