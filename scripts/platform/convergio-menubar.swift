#!/usr/bin/env swift
// convergio-menubar.swift — macOS menu bar status icon for Convergio daemon
// Shows daemon status, active agents, quick actions
// Build: swiftc convergio-menubar.swift -o convergio-menubar -framework Cocoa
// Run:   ./convergio-menubar &

import Cocoa

class ConvergioMenuBar: NSObject, NSApplicationDelegate {
    var statusItem: NSStatusItem!
    var timer: Timer?
    let daemonURL = ProcessInfo.processInfo.environment["CONVERGIO_DAEMON_URL"] ?? "http://localhost:8420"
    let platformDir = ProcessInfo.processInfo.environment["CONVERGIO_PLATFORM_DIR"] ?? "\(NSHomeDirectory())/GitHub/ConvergioPlatform"

    func applicationDidFinishLaunching(_ notification: Notification) {
        statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)
        updateIcon(running: false)
        updateMenu()
        timer = Timer.scheduledTimer(withTimeInterval: 10, repeats: true) { [weak self] _ in
            self?.refresh()
        }
        refresh()
    }

    func updateIcon(running: Bool) {
        if let button = statusItem.button {
            button.title = running ? "◉" : "◎"
            button.toolTip = running ? "Convergio: running" : "Convergio: stopped"
        }
    }

    func updateMenu() {
        let menu = NSMenu()

        menu.addItem(NSMenuItem(title: "Convergio Daemon", action: nil, keyEquivalent: ""))
        menu.addItem(NSMenuItem.separator())

        let statusItem = NSMenuItem(title: "Checking...", action: nil, keyEquivalent: "")
        statusItem.tag = 100
        menu.addItem(statusItem)

        let agentsItem = NSMenuItem(title: "Agents: —", action: nil, keyEquivalent: "")
        agentsItem.tag = 101
        menu.addItem(agentsItem)

        menu.addItem(NSMenuItem.separator())

        menu.addItem(NSMenuItem(title: "Start Daemon", action: #selector(startDaemon), keyEquivalent: "s"))
        menu.addItem(NSMenuItem(title: "Stop Daemon", action: #selector(stopDaemon), keyEquivalent: "x"))
        menu.addItem(NSMenuItem(title: "Open Dashboard", action: #selector(openDashboard), keyEquivalent: "d"))

        menu.addItem(NSMenuItem.separator())

        menu.addItem(NSMenuItem(title: "Open Terminal", action: #selector(openTerminal), keyEquivalent: "t"))
        menu.addItem(NSMenuItem(title: "Quit", action: #selector(quitApp), keyEquivalent: "q"))

        self.statusItem.menu = menu
    }

    func refresh() {
        let url = URL(string: "\(daemonURL)/api/ipc/status")!
        let task = URLSession.shared.dataTask(with: url) { [weak self] data, response, error in
            DispatchQueue.main.async {
                if let data = data, let _ = try? JSONSerialization.jsonObject(with: data) as? [String: Any] {
                    self?.updateIcon(running: true)
                    self?.statusItem.menu?.item(withTag: 100)?.title = "Status: running"

                    // Get agent count
                    self?.fetchAgentCount()
                } else {
                    self?.updateIcon(running: false)
                    self?.statusItem.menu?.item(withTag: 100)?.title = "Status: stopped"
                    self?.statusItem.menu?.item(withTag: 101)?.title = "Agents: —"
                }
            }
        }
        task.resume()
    }

    func fetchAgentCount() {
        let url = URL(string: "\(daemonURL)/api/ipc/agents")!
        let task = URLSession.shared.dataTask(with: url) { [weak self] data, _, _ in
            DispatchQueue.main.async {
                if let data = data,
                   let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
                   let agents = json["agents"] as? [[String: Any]] {
                    self?.statusItem.menu?.item(withTag: 101)?.title = "Agents: \(agents.count) active"
                    if !agents.isEmpty {
                        self?.statusItem.button?.title = "◉ \(agents.count)"
                    }
                }
            }
        }
        task.resume()
    }

    @objc func startDaemon() {
        Process.launchedProcess(launchPath: "/bin/bash", arguments: ["\(platformDir)/daemon/start.sh"])
        DispatchQueue.main.asyncAfter(deadline: .now() + 3) { [weak self] in self?.refresh() }
    }

    @objc func stopDaemon() {
        Process.launchedProcess(launchPath: "/usr/bin/pkill", arguments: ["-f", "claude-core"])
        DispatchQueue.main.asyncAfter(deadline: .now() + 1) { [weak self] in self?.refresh() }
    }

    @objc func openDashboard() {
        NSWorkspace.shared.open(URL(string: daemonURL)!)
    }

    @objc func openTerminal() {
        let script = "tell application \"Terminal\" to do script \"cd \(platformDir) && convergio status\""
        if let appleScript = NSAppleScript(source: script) {
            appleScript.executeAndReturnError(nil)
        }
    }

    @objc func quitApp() {
        NSApp.terminate(nil)
    }
}

let app = NSApplication.shared
let delegate = ConvergioMenuBar()
app.delegate = delegate
app.setActivationPolicy(.accessory)
app.run()
