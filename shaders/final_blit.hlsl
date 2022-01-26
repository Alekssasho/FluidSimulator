[[vk::binding(0)]] Texture2D<float4> main_tex;
[[vk::binding(1)]] Texture2D<float4> gui_tex;
[[vk::binding(2)]] RWTexture2D<float4> output_tex;
[[vk::binding(3)]] cbuffer _ {
    float4 output_tex_size;
};

[numthreads(8, 8, 1)]
void main(in uint2 px : SV_DispatchThreadID) {
    float3 main = main_tex[px].rgb;
    float4 gui = gui_tex[px];

    float3 result = main.rgb * (1.0 - gui.a) + gui.rgb;

    output_tex[px] = float4(result, 1);
}
