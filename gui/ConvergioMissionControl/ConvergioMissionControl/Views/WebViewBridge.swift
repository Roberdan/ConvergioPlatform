// SPDX-License-Identifier: MPL-2.0
// ConvergioMissionControl — WKWebView bridge for embedded dashboard views

import SwiftUI
import WebKit

/// Inbound message from JS via window.webkit.messageHandlers.convergio.postMessage().
struct BridgeMessage: Decodable {
    let action: String
    let payload: [String: String]?
}

enum ConnectionState {
    case connecting, connected, failed(Error)
}

// MARK: - WebViewBridge

/// NSViewRepresentable wrapping WKWebView to embed dashboard views from the daemon.
/// Bidirectional JS<->Swift communication via WKScriptMessageHandler.
struct WebViewBridge: NSViewRepresentable {
    let tab: String
    let daemonPort: Int
    var onBridgeMessage: ((BridgeMessage) -> Void)?

    init(tab: String = "brain", daemonPort: Int = 8420, onBridgeMessage: ((BridgeMessage) -> Void)? = nil) {
        self.tab = tab
        self.daemonPort = daemonPort
        self.onBridgeMessage = onBridgeMessage
    }

    func makeCoordinator() -> Coordinator {
        Coordinator(onBridgeMessage: onBridgeMessage)
    }

    func makeNSView(context: Context) -> WKWebView {
        let config = WKWebViewConfiguration()
        let contentController = WKUserContentController()

        contentController.add(context.coordinator, name: "convergio")
        let bridgeScript = WKUserScript(
            source: "window.__convergioEmbedded = true;",
            injectionTime: .atDocumentStart,
            forMainFrameOnly: true
        )
        contentController.addUserScript(bridgeScript)
        config.userContentController = contentController

        let webView = WKWebView(frame: .zero, configuration: config)
        webView.navigationDelegate = context.coordinator
        context.coordinator.webView = webView

        #if DEBUG
        webView.isInspectable = true
        #endif

        loadDashboard(webView)
        return webView
    }

    func updateNSView(_ webView: WKWebView, context: Context) {
        if context.coordinator.currentTab != tab {
            context.coordinator.currentTab = tab
            loadDashboard(webView)
        }
    }

    private func loadDashboard(_ webView: WKWebView) {
        let urlString = "http://localhost:\(daemonPort)/?mode=embedded&tab=\(tab)"
        guard let url = URL(string: urlString) else { return }
        webView.load(URLRequest(url: url))
    }
}

// MARK: - Coordinator

extension WebViewBridge {
    final class Coordinator: NSObject, WKScriptMessageHandler, WKNavigationDelegate {
        var webView: WKWebView?
        var currentTab: String = ""
        var connectionState: ConnectionState = .connecting
        private let onBridgeMessage: ((BridgeMessage) -> Void)?
        private var retryCount = 0
        private let maxRetries = 5
        private let retryBaseDelay: TimeInterval = 2.0
        private var retryTask: DispatchWorkItem?

        init(onBridgeMessage: ((BridgeMessage) -> Void)?) {
            self.onBridgeMessage = onBridgeMessage
        }

        func userContentController(
            _ userContentController: WKUserContentController,
            didReceive message: WKScriptMessage
        ) {
            guard message.name == "convergio" else { return }
            guard let body = message.body as? [String: Any] else {
                NSLog("[WebViewBridge] Unexpected message body type: \(type(of: message.body))")
                return
            }

            guard let action = body["action"] as? String else {
                NSLog("[WebViewBridge] Message missing 'action' field")
                return
            }

            let payload = body["payload"] as? [String: String]
            let bridgeMessage = BridgeMessage(action: action, payload: payload)
            onBridgeMessage?(bridgeMessage)
        }

        func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) {
            connectionState = .connected
            retryCount = 0
            cancelRetry()
        }

        func webView(
            _ webView: WKWebView,
            didFailProvisionalNavigation navigation: WKNavigation!,
            withError error: Error
        ) {
            connectionState = .failed(error)
            NSLog("[WebViewBridge] Connection failed: \(error.localizedDescription)")
            scheduleRetry(webView: webView)
        }

        func webView(_ webView: WKWebView, didFail navigation: WKNavigation!, withError error: Error) {
            connectionState = .failed(error)
            NSLog("[WebViewBridge] Navigation failed: \(error.localizedDescription)")
            scheduleRetry(webView: webView)
        }

        /// Evaluate JS in the webview context (Swift->JS direction).
        func evaluateJavaScript(_ script: String, completion: ((Any?, Error?) -> Void)? = nil) {
            webView?.evaluateJavaScript(script) { result, error in
                if let error = error {
                    NSLog("[WebViewBridge] JS eval error: \(error.localizedDescription)")
                }
                completion?(result, error)
            }
        }

        /// Sync the macOS appearance theme to the embedded dashboard.
        func syncTheme(isDark: Bool) {
            let theme = isDark ? "dark" : "light"
            evaluateJavaScript("window.__convergioSetTheme && window.__convergioSetTheme('\(theme)')")
        }

        /// Tell the embedded dashboard to switch display mode.
        func setMode(_ mode: String) {
            evaluateJavaScript("window.__convergioSetMode && window.__convergioSetMode('\(mode)')")
        }

        /// Exponential backoff retry with offline placeholder between attempts.
        private func scheduleRetry(webView: WKWebView) {
            guard retryCount < maxRetries else {
                NSLog("[WebViewBridge] Max retries (\(maxRetries)) reached, showing offline placeholder")
                loadOfflinePlaceholder(webView: webView)
                return
            }

            retryCount += 1
            let delay = retryBaseDelay * pow(2.0, Double(retryCount - 1))
            NSLog("[WebViewBridge] Retry \(retryCount)/\(maxRetries) in \(delay)s")

            loadOfflinePlaceholder(webView: webView)

            let task = DispatchWorkItem { [weak self, weak webView] in
                guard let webView = webView, let self = self else { return }
                let urlString = "http://localhost:\(self.daemonPort)/?mode=embedded&tab=\(self.currentTab)"
                guard let url = URL(string: urlString) else { return }
                webView.load(URLRequest(url: url))
            }
            retryTask = task
            DispatchQueue.main.asyncAfter(deadline: .now() + delay, execute: task)
        }

        private func cancelRetry() {
            retryTask?.cancel()
            retryTask = nil
        }

        private var daemonPort: Int {
            webView?.url?.port ?? 8420
        }

        private func loadOfflinePlaceholder(webView: WKWebView) {
            let status = retryCount < maxRetries ? "Retrying..." : "Start the daemon: ./daemon/start.sh"
            let html = """
            <!DOCTYPE html><html><head><style>
            body { font-family: -apple-system, sans-serif; display: flex; align-items: center;
              justify-content: center; height: 100vh; margin: 0; background: #1a1a1a; color: #aaa;
              flex-direction: column; gap: 12px; }
            .t { font-size: 16px; font-weight: 600; color: #ccc; }
            .d { font-size: 13px; } .r { font-size: 12px; color: #666; margin-top: 8px; }
            </style></head><body>
            <div style="font-size:48px;opacity:0.4">&#x1F50C;</div>
            <div class="t">Daemon Unavailable</div>
            <div class="d">Cannot reach localhost:\(daemonPort)</div>
            <div class="r">\(status)</div>
            </body></html>
            """
            webView.loadHTMLString(html, baseURL: nil)
        }
    }
}