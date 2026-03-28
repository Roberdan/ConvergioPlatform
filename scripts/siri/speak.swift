#!/usr/bin/env swift
// speak.swift — Use Siri neural voices via AVSpeechSynthesizer
// Usage: swift speak.swift "Testo da parlare"
// Uses the best available Italian Siri voice.

import AVFoundation
import Foundation

let text = CommandLine.arguments.dropFirst().joined(separator: " ")
guard !text.isEmpty else {
    fputs("Usage: swift speak.swift \"text to speak\"\n", stderr)
    exit(1)
}

let utterance = AVSpeechUtterance(string: text)
utterance.rate = AVSpeechUtteranceDefaultSpeechRate

// Find the best Italian voice — prefer Siri/Premium voices
let italianVoices = AVSpeechSynthesisVoice.speechVoices()
    .filter { $0.language.hasPrefix("it") }
    .sorted { v1, v2 in
        // Higher quality first
        v1.quality.rawValue > v2.quality.rawValue
    }

if let bestVoice = italianVoices.first {
    utterance.voice = bestVoice
    fputs("Using voice: \(bestVoice.name) (quality: \(bestVoice.quality.rawValue))\n", stderr)
} else {
    utterance.voice = AVSpeechSynthesisVoice(language: "it-IT")
}

let synth = AVSpeechSynthesizer()

// Synchronous playback — wait until done
let semaphore = DispatchSemaphore(value: 0)

class Delegate: NSObject, AVSpeechSynthesizerDelegate {
    let semaphore: DispatchSemaphore
    init(_ s: DispatchSemaphore) { self.semaphore = s }
    func speechSynthesizer(_ synth: AVSpeechSynthesizer, didFinish utterance: AVSpeechUtterance) {
        semaphore.signal()
    }
}

let delegate = Delegate(semaphore)
synth.delegate = delegate
synth.speak(utterance)
semaphore.wait()
