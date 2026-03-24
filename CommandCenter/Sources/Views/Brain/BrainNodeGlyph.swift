import SwiftUI

// MARK: - Node glyph rendered on the brain graph canvas

struct BrainNodeGlyph: View {
    let node: BrainGraphNode
    let isSelected: Bool

    var body: some View {
        VStack(spacing: 6) {
            shapeView
                .frame(width: 38, height: 38)
                .overlay(
                    Text(symbol)
                        .font(.caption.weight(.bold))
                        .foregroundStyle(ConvergioTokens.Text.textPrimary)
                )
                // Colored glow matching node kind for sci-fi feel
                .shadow(color: node.color.opacity(0.6), radius: isSelected ? 14 : 8)
                .shadow(color: node.color.opacity(0.3), radius: isSelected ? 22 : 12)

            Text(node.title)
                .font(.caption2.weight(.semibold))
                .foregroundStyle(ConvergioTokens.Text.textPrimary)
                .lineLimit(2)
                .multilineTextAlignment(.center)
                .frame(width: 110)
        }
        .shadow(color: .black.opacity(0.28), radius: 10, y: 6)
    }

    @ViewBuilder
    private var shapeView: some View {
        switch node.kind {
        case .agent:
            Circle()
                .fill(fillColor)
                .overlay(Circle().stroke(isSelected ? Color.white : .clear, lineWidth: 2))
        case .task:
            RoundedRectangle(cornerRadius: 10, style: .continuous)
                .fill(fillColor)
                .overlay(
                    RoundedRectangle(cornerRadius: 10, style: .continuous)
                        .stroke(isSelected ? Color.white : .clear, lineWidth: 2)
                )
        case .plan:
            HexagonShape()
                .fill(fillColor)
                .overlay(HexagonShape().stroke(isSelected ? Color.white : .clear, lineWidth: 2))
        }
    }

    private var fillColor: Color { node.kind.color }

    private var symbol: String {
        switch node.kind {
        case .agent: "A"
        case .task:  "T"
        case .plan:  "P"
        }
    }
}

// MARK: - Hexagon insettable shape

struct HexagonShape: InsettableShape {
    var insetAmount: CGFloat = 0

    func path(in rect: CGRect) -> Path {
        let rect = rect.insetBy(dx: insetAmount, dy: insetAmount)
        let points = [
            CGPoint(x: rect.midX, y: rect.minY),
            CGPoint(x: rect.maxX, y: rect.minY + rect.height * 0.25),
            CGPoint(x: rect.maxX, y: rect.minY + rect.height * 0.75),
            CGPoint(x: rect.midX, y: rect.maxY),
            CGPoint(x: rect.minX, y: rect.minY + rect.height * 0.75),
            CGPoint(x: rect.minX, y: rect.minY + rect.height * 0.25),
        ]
        var path = Path()
        path.addLines(points)
        path.closeSubpath()
        return path
    }

    func inset(by amount: CGFloat) -> some InsettableShape {
        var shape = self
        shape.insetAmount += amount
        return shape
    }
}
