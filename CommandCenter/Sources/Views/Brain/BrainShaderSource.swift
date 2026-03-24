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
        half3 base = mix(half3(0.03, 0.06, 0.12), half3(0.10, 0.20, 0.32), half(wave));
        half3 accent = half3(0.22, 0.72, 0.95) * half(grid * 0.25);
        return half4(base + accent, 1.0);
    }
    """
}
