struct VSInput {
    float2 position : POSITION;
    uint instance_id : SV_InstanceID;
};

struct VSOutput {
    float4 position: SV_POSITION;
};

static const float PI = 3.14159265f;

struct ConstantsData {
    uint2 grid_size;
};

ConstantBuffer<ConstantsData> g_constant_data : register(b0);
StructuredBuffer<float2> g_velocity_field : register(t1);

VSOutput vs_main(VSInput input) {
    const float2 velocity = g_velocity_field[input.instance_id];
    const float velocity_magnitude = length(velocity);
    if(velocity_magnitude == 0.0) {
        // Output degenerate triangle
        VSOutput output;
        output.position = 0.0;
        return output;
    }

    const float2 velocity_direction = velocity / velocity_magnitude;

    const float2x2 rotate_matrix = float2x2(velocity_direction.y, velocity_direction.x, -velocity_direction.x, velocity_direction.y);
    const float2 vertex_rotated_position = mul(rotate_matrix, input.position * (velocity_magnitude / 1.5));

    const uint2 grid_position = uint2(input.instance_id % g_constant_data.grid_size.x, input.instance_id / g_constant_data.grid_size.y);

    const float2 grid_position_float = grid_position / float2(g_constant_data.grid_size);
    const float2 vertex_rotated_position_in_grid_space = (vertex_rotated_position + 1.0) / 2.0;
    const float2 final_vertex_position_in_grid_space = (vertex_rotated_position_in_grid_space * 1.0 / float2(g_constant_data.grid_size)) + grid_position_float;

    const float2 final_position_in_ndc_space = final_vertex_position_in_grid_space * 2.0 - 1.0;

    VSOutput output;
    output.position = float4(final_position_in_ndc_space, 0.0, 1.0);
    return output;
}

float4 ps_main(VSOutput input) : SV_Target0 {
    return float4(1.0, 0.0, 1.0, 1.0);
}
