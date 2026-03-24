import AppKit
import MetalKit
import QuartzCore
import SwiftUI

struct MetalBackdropView: NSViewRepresentable {
    let isActive: Bool

    func makeCoordinator() -> Coordinator {
        Coordinator()
    }

    func makeNSView(context: Context) -> NSView {
        guard let device = MTLCreateSystemDefaultDevice() else {
            return NSVisualEffectView()
        }

        let view = MTKView(frame: .zero, device: device)
        view.clearColor = MTLClearColor(red: 0.03, green: 0.05, blue: 0.10, alpha: 1)
        view.colorPixelFormat = .bgra8Unorm
        view.enableSetNeedsDisplay = false
        view.isPaused = !isActive
        view.preferredFramesPerSecond = 60
        view.framebufferOnly = false

        let renderer = try? BrainMetalRenderer(device: device)
        context.coordinator.renderer = renderer
        renderer?.isActive = isActive
        view.delegate = renderer
        return view
    }

    func updateNSView(_ nsView: NSView, context: Context) {
        guard let view = nsView as? MTKView else { return }
        view.isPaused = !isActive
        context.coordinator.renderer?.isActive = isActive
    }

    final class Coordinator {
        var renderer: BrainMetalRenderer?
    }
}

enum BrainMetalSupport {
    static func isShaderAvailable() -> Bool {
        guard let device = MTLCreateSystemDefaultDevice() else { return false }
        return (try? BrainMetalRenderer(device: device)) != nil
    }
}

final class BrainMetalRenderer: NSObject, MTKViewDelegate {
    private let commandQueue: MTLCommandQueue?
    private let pipelineState: MTLRenderPipelineState?
    private let startTime = CACurrentMediaTime()

    var isActive = true

    init(device: MTLDevice) throws {
        commandQueue = device.makeCommandQueue()
        let library = try device.makeLibrary(source: BrainShaderSource.source, options: nil)
        let descriptor = MTLRenderPipelineDescriptor()
        descriptor.vertexFunction = library.makeFunction(name: "brainVertex")
        descriptor.fragmentFunction = library.makeFunction(name: "brainFragment")
        descriptor.colorAttachments[0].pixelFormat = .bgra8Unorm
        pipelineState = try device.makeRenderPipelineState(descriptor: descriptor)
        super.init()
    }

    func mtkView(_ view: MTKView, drawableSizeWillChange size: CGSize) {}

    func draw(in view: MTKView) {
        guard isActive,
              let drawable = view.currentDrawable,
              let renderPassDescriptor = view.currentRenderPassDescriptor,
              let commandQueue,
              let pipelineState
        else { return }

        var time = Float(CACurrentMediaTime() - startTime)
        let commandBuffer = commandQueue.makeCommandBuffer()
        let encoder = commandBuffer?.makeRenderCommandEncoder(descriptor: renderPassDescriptor)
        encoder?.setRenderPipelineState(pipelineState)
        encoder?.setFragmentBytes(&time, length: MemoryLayout<Float>.size, index: 0)
        encoder?.drawPrimitives(type: .triangle, vertexStart: 0, vertexCount: 3)
        encoder?.endEncoding()
        commandBuffer?.present(drawable)
        commandBuffer?.commit()
    }
}
