[[vk::binding(0)]] StructuredBuffer<float> density_field;
[[vk::binding(1)]] RWTexture2D<float4> output_tex;
[[vk::binding(2)]] cbuffer _ {
    uint2 grid_size;
    float2 output_tex_size;
};

[numthreads(8, 8, 1)]
void main(in uint2 px : SV_DispatchThreadID) {
    uint2 position_in_grid = (output_tex_size / px) * grid_size;

    output_tex[px] = float4(density_field[position_in_grid.x + position_in_grid.y * grid_size.x], 0.0, 0.0, 1.0);
}
