struct ConstantsData {
    uint2 grid_size;
};
[[vk::binding(0)]] ConstantBuffer<ConstantsData> c;
[[vk::binding(1)]] RWStructuredBuffer<float> density_field;
[[vk::binding(2)]] cbuffer _ {
    uint2 screen_position;
    float2 output_tex_size;
};

[numthreads(1, 1, 1)]
void main(uint3 tid : SV_DispatchThreadID) {
    uint2 position_in_grid = (screen_position / output_tex_size) * c.grid_size;
    density_field[position_in_grid.x + position_in_grid.y * c.grid_size.x] += 0.01;
}
