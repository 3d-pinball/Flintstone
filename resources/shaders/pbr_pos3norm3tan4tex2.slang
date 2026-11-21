struct VertexInput {
    float3 pos;
    float3 normal;
    float4 tangent;
    float2 tex;
};

struct VertexOutput {
    float4 sv_position : SV_Position;
    float3 norm;
    float4 tangent;
    float2 tex;
};

struct PushData {
    float4x4 model;
}

struct VertexTarget {
    float4 pos;
    float4 normal;
    float4 tangent;
}

[[vk::push_constant]]
ConstantBuffer<PushData> push_data;

[[vk::binding(0, 0)]]
ConstantBuffer<float4x4> vp;

[[vk::binding(0, 1)]] //set 1 bind 0
ConstantBuffer<Sampler2D> texture_color;

[[vk::binding(1, 1)]]
ConstantBuffer<Sampler2D> texture_metallic_roughness;

[[vk::binding(2, 1)]]
ConstantBuffer<Sampler2D> texture_normal;

[[vk::binding(3, 1)]]
ConstantBuffer<Sampler2D> texture_occlusion;

[[vk::binding(4, 1)]]
ConstantBuffer<Sampler2D> texture_emissive;

[[vk::binding(5, 1)]]
ConstantBuffer<float4> color_factor;

[[vk::binding(6, 1)]]
ConstantBuffer<float2> metallic_roughness_factor;

[[vk::binding(7, 1)]]
ConstantBuffer<float> normal_scale;

[[vk::binding(8, 1)]]
ConstantBuffer<float> occlusion_strength;

[shader("vertex")]
VertexOutput vertMain(VertexInput input, uint vertex_id : SV_VertexID) {
    float3 pos = input.pos;
    return VertexOutput(mul(vp, mul(push_data.model, float4(pos, 1.0))), input.normal, input.tangent, input.tex);
}

[shader("fragment")]
float4 fragMain(VertexOutput vert) : SV_Target {
    return texture_color.Sample(vert.tex) * color_factor;
}
