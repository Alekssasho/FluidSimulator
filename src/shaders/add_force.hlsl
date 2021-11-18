struct PushConstantData {
    float2 screen_position;
};

[[vk::push_constant]] PushConstantData g_push_data;

struct ConstantsData {
    uint2 grid_size;
};
ConstantBuffer<ConstantsData> g_constant_data : register(b0);

RWStructuredBuffer<float2> g_velocity_field : register(u1);
RWStructuredBuffer<float> g_density_field : register(u2);

[numthreads(1, 1, 1)]
void cs_main(uint3 tid : SV_DispatchThreadID) {
    uint2 position_in_grid = g_push_data.screen_position * g_constant_data.grid_size;
    g_density_field[position_in_grid.x + position_in_grid.y * g_constant_data.grid_size.x] += 0.01;
}
