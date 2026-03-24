// BrainShaderSource.swift — Metal backdrop for BrainVisualizationView.
// Clear color matches surfaceSunken (#0a0a0a → 0.039, 0.039, 0.039).
// Grid accent blends gialloFerrari (#FFC72C) tint subtly into the base.
enum BrainShaderSource {
    static let source = """
    #include <metal_stdlib>
    using namespace metal;

    struct VertexOut {
        float4 position [[position]];
        float2 uv;
    };

    vertex VertexOut brainVertex(uint vertexID [[vertex_id]]) {
        float2 positions[3] = {
            float2(-1.0, -1.0),
            float2( 3.0, -1.0),
            float2(-1.0,  3.0)
        };
        VertexOut out;
        out.position = float4(positions[vertexID], 0.0, 1.0);
        out.uv = positions[vertexID] * 0.5 + 0.5;
        return out;
    }

    fragment half4 brainFragment(VertexOut in [[stage_in]], constant float &time [[buffer(0)]]) {
        float wave = 0.5 + 0.5 * sin((in.uv.x + in.uv.y + time * 0.08) * 6.28318);
        float gridX = smoothstep(0.0, 0.04, abs(fract(in.uv.x * 14.0) - 0.5));
        float gridY = smoothstep(0.0, 0.04, abs(fract(in.uv.y * 10.0) - 0.5));
        float grid = max(gridX, gridY);
        // surfaceSunken base: #0a0a0a (0.039) → #0a1010 (subtle petrolio wave)
        half3 base = mix(half3(0.039, 0.039, 0.039), half3(0.039, 0.055, 0.070), half(wave));
        // gialloFerrari (#FFC72C) tint on grid lines at low intensity
        half3 accent = half3(1.0, 0.78, 0.17) * half(grid * 0.08);
        return half4(base + accent, 1.0);
    }
    """
}
