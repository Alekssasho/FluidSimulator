struct PushConstantData {
    float2 forced_velocity;
};

[[vk::push_constant]] PushConstantData g_push_data;


struct ConstantsData {
    uint2 grid_size;
};
ConstantBuffer<ConstantsData> g_constant_data : register(b0);

RWStructuredBuffer<float2> g_velocity_field : register(u1);

[numthreads(32, 1, 1)]
void cs_main(uint3 tid : SV_DispatchThreadID) {
    const uint max_grid_index = g_constant_data.grid_size.x * g_constant_data.grid_size.y;
    if(tid.x >= max_grid_index) {
        return;
    }

    g_velocity_field[tid.x] = g_push_data.forced_velocity;
}
