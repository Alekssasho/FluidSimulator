struct VSInput {
    float2 position : POSITION;
    uint instance_id : SV_InstanceID;
};

struct VSOutput {
    float4 position: SV_POSITION;
};

static const float PI = 3.14159265f;

struct ConstantsData {
    uint grid_size_x;
    uint grid_size_y;
};

ConstantBuffer<ConstantsData> g_constant_data : register(b0);
StructuredBuffer<float2> g_velocity_field : register(t1);

VSOutput vs_main(VSInput input) {
    const uint2 grid_position = uint2(input.instance_id % g_constant_data.grid_size_x, input.instance_id / g_constant_data.grid_size_y);

    float2 grid_position_float = grid_position / float2(g_constant_data.grid_size_x, g_constant_data.grid_size_y);
    grid_position_float = grid_position_float * 2.0 - 1.0;
    const float2 half_grid_position_offset = float2(1.0 / g_constant_data.grid_size_x, 1.0 / g_constant_data.grid_size_y);


    const float2x2 rotate = float2x2(sin(PI / 4.0), cos(PI / 4.0), cos(PI / 4.0), -sin(PI / 4.0));
    const float2 position = mul(rotate, input.position) * half_grid_position_offset + half_grid_position_offset + grid_position_float;

    VSOutput output;
    output.position = float4(position, 0.0, 1.0);
    return output;
}

float4 ps_main(VSOutput input) : SV_Target0 {
    return float4(1.0, 0.0, 1.0, 1.0);
}
