struct ConstantsData {
    uint2 grid_size;
};
[[vk::binding(0)]] ConstantBuffer<ConstantsData> c;
[[vk::binding(1)]] StructuredBuffer<float> density_field;
[[vk::binding(2)]] RWTexture2D<float4> output_tex;
[[vk::binding(3)]] cbuffer _ {
    float2 output_tex_size;
};

[numthreads(8, 8, 1)]
void main(in uint2 px : SV_DispatchThreadID) {
    uint2 position_in_grid = (px / output_tex_size) * c.grid_size;

    output_tex[px] = float4(density_field[position_in_grid.x + position_in_grid.y * c.grid_size.x], 0.0, 0.0, 1.0);
}
