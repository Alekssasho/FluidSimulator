use glam::{vec2, Vec2};
use wgpu::{util::DeviceExt, ComputePipelineDescriptor, PushConstantRange, ShaderStages};

const GRID_SIZE_X: usize = 20;
const GRID_SIZE_Y: usize = 20;
const VELOCITY_BUFFER_SIZE: usize = GRID_SIZE_X * GRID_SIZE_Y * std::mem::size_of::<glam::Vec2>();
pub struct FluidSimulator {
    render_pipeline: wgpu::RenderPipeline,
    density_render_pipeline: wgpu::RenderPipeline,
    compute_pipeline: wgpu::ComputePipeline,
    uniform_bind_group: wgpu::BindGroup,
    compute_uniform_bind_group: wgpu::BindGroup,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    _velocity_buffer: wgpu::Buffer,
    _constants_buffer: wgpu::Buffer,

    pub forced_velocity: Vec2,
    pub forced_density: f32,
}

#[derive(Clone, Copy)]
#[repr(C)]
struct ConstantsData {
    grid_size_x: u32,
    grid_size_y: u32,
}

unsafe impl bytemuck::Pod for ConstantsData {}
unsafe impl bytemuck::Zeroable for ConstantsData {}

#[derive(Clone, Copy)]
#[repr(C)]
struct PushConstants {
    forced_velocity: Vec2,
    forced_density: f32,
}

unsafe impl bytemuck::Pod for PushConstants {}
unsafe impl bytemuck::Zeroable for PushConstants {}

impl FluidSimulator {
    fn compile_shader(
        compiler: &hassle_rs::DxcCompiler,
        library: &hassle_rs::DxcLibrary,
        blob: &hassle_rs::DxcBlobEncoding,
        entry_point: &str,
        profile: &str,
    ) -> Vec<u8> {
        let result = match compiler.compile(
            &blob,
            "Shader",
            entry_point,
            profile,
            &["-spirv"],
            None,
            &[],
        ) {
            Ok(result) => result,
            Err(result) => {
                let error_blob = result
                    .0
                    .get_error_buffer()
                    .map_err(hassle_rs::utils::HassleError::Win32Error)
                    .unwrap();
                println!("{}", library.get_blob_as_string(&error_blob));
                panic!();
            }
        };
        result.get_result().unwrap().to_vec()
    }

    pub fn new(renderer: &rend3::Renderer, surface_format: wgpu::TextureFormat) -> Self {
        let dxc = hassle_rs::Dxc::new().unwrap();
        let compiler = dxc.create_compiler().unwrap();
        let library = dxc.create_library().unwrap();
        let blob = library
            .create_blob_with_encoding_from_str(include_str!("shaders/velocity_field.hlsl"))
            .unwrap();
        let vs_code =
            FluidSimulator::compile_shader(&compiler, &library, &blob, "vs_main", "vs_6_6");
        let ps_code =
            FluidSimulator::compile_shader(&compiler, &library, &blob, "ps_main", "ps_6_6");

        let blob = library
            .create_blob_with_encoding_from_str(include_str!("shaders/velocity_calculations.hlsl"))
            .unwrap();
        let cs_code =
            FluidSimulator::compile_shader(&compiler, &library, &blob, "cs_main", "cs_6_6");

        let blob = library
            .create_blob_with_encoding_from_str(include_str!("shaders/density_visualize.hlsl"))
            .unwrap();
        let vs_density_code =
            FluidSimulator::compile_shader(&compiler, &library, &blob, "vs_main", "vs_6_6");
        let ps_density_code =
            FluidSimulator::compile_shader(&compiler, &library, &blob, "ps_main", "ps_6_6");

        let device: &wgpu::Device = &renderer.device;
        let vs_shader = wgpu::ShaderModuleDescriptor {
            label: Some("velocity_field_vs_shader"),
            source: wgpu::util::make_spirv(vs_code.as_slice()),
        };
        let vs_module = device.create_shader_module(&vs_shader);

        let ps_shader = wgpu::ShaderModuleDescriptor {
            label: Some("velocity_field_ps_shader"),
            source: wgpu::util::make_spirv(ps_code.as_slice()),
        };
        let ps_module = device.create_shader_module(&ps_shader);

        let cs_shader = wgpu::ShaderModuleDescriptor {
            label: Some("velocity_calculation_cs_shader"),
            source: wgpu::util::make_spirv(cs_code.as_slice()),
        };
        let cs_module = device.create_shader_module(&cs_shader);

        let density_vs_shader = wgpu::ShaderModuleDescriptor {
            label: Some("density_field_vs_shader"),
            source: wgpu::util::make_spirv(vs_density_code.as_slice()),
        };
        let density_vs_module = device.create_shader_module(&density_vs_shader);

        let density_ps_shader = wgpu::ShaderModuleDescriptor {
            label: Some("density_field_ps_shader"),
            source: wgpu::util::make_spirv(ps_density_code.as_slice()),
        };
        let density_ps_module = device.create_shader_module(&density_ps_shader);

        let velocity_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("velocity_field_velocity_buffer"),
            usage: wgpu::BufferUsages::STORAGE,
            size: VELOCITY_BUFFER_SIZE as u64,
            mapped_at_creation: false,
        });

        let density_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("density_field_buffer"),
            usage: wgpu::BufferUsages::STORAGE,
            size: (GRID_SIZE_X * GRID_SIZE_Y * std::mem::size_of::<f32>()) as u64,
            mapped_at_creation: false,
        });

        let constants_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("velocity_field_constants_data_buffer"),
            usage: wgpu::BufferUsages::UNIFORM,
            contents: bytemuck::cast_slice(&[ConstantsData {
                grid_size_x: GRID_SIZE_X as u32,
                grid_size_y: GRID_SIZE_Y as u32,
            }]),
        });

        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("velocity_field_bind_group_layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            has_dynamic_offset: false,
                            min_binding_size: None,
                            ty: wgpu::BufferBindingType::Uniform,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            has_dynamic_offset: false,
                            min_binding_size: None,
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            has_dynamic_offset: false,
                            min_binding_size: None,
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                        },
                        count: None,
                    },
                ],
            });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("velocity_field_bind_group"),
            layout: &uniform_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &constants_buffer,
                        offset: 0,
                        size: None,
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &velocity_buffer,
                        offset: 0,
                        size: None,
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &density_buffer,
                        offset: 0,
                        size: None,
                    }),
                },
            ],
        });

        let compute_uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("compute_velocity_field_bind_group_layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            has_dynamic_offset: false,
                            min_binding_size: None,
                            ty: wgpu::BufferBindingType::Uniform,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            has_dynamic_offset: false,
                            min_binding_size: None,
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            has_dynamic_offset: false,
                            min_binding_size: None,
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                        },
                        count: None,
                    },
                ],
            });

        let compute_uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("compute_velocity_field_bind_group"),
            layout: &compute_uniform_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &constants_buffer,
                        offset: 0,
                        size: None,
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &velocity_buffer,
                        offset: 0,
                        size: None,
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &density_buffer,
                        offset: 0,
                        size: None,
                    }),
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("velocity_field_pipeline_layout"),
            bind_group_layouts: &[&uniform_bind_group_layout],
            push_constant_ranges: &[],
        });

        let compute_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("velocity_field_compute_pipeline_layout"),
                bind_group_layouts: &[&compute_uniform_bind_group_layout],
                push_constant_ranges: &[PushConstantRange {
                    stages: ShaderStages::COMPUTE,
                    range: 0..std::mem::size_of::<PushConstants>() as u32,
                }],
            });

        let fragment_target = [wgpu::ColorTargetState {
            format: surface_format,
            blend: Some(wgpu::BlendState {
                color: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::One,
                    dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                    operation: wgpu::BlendOperation::Add,
                },
                alpha: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::OneMinusDstAlpha,
                    dst_factor: wgpu::BlendFactor::One,
                    operation: wgpu::BlendOperation::Add,
                },
            }),
            write_mask: wgpu::ColorWrites::ALL,
        }];

        let mut pipeline_description = wgpu::RenderPipelineDescriptor {
            label: Some("velocity_field_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                entry_point: "vs_main",
                module: &vs_module,
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: 2 * std::mem::size_of::<f32>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2],
                }],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                clamp_depth: false,
                conservative: false,
                cull_mode: None,
                front_face: wgpu::FrontFace::default(),
                polygon_mode: wgpu::PolygonMode::default(),
                strip_index_format: None,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                alpha_to_coverage_enabled: false,
                count: 1,
                mask: !0,
            },

            fragment: Some(wgpu::FragmentState {
                module: &ps_module,
                entry_point: "ps_main",
                targets: &fragment_target,
            }),
        };

        let render_pipeline = device.create_render_pipeline(&pipeline_description);

        pipeline_description.vertex.module = &density_vs_module;
        pipeline_description.vertex.buffers = &[];
        pipeline_description.fragment.as_mut().unwrap().module = &density_ps_module;
        let density_render_pipeline = device.create_render_pipeline(&pipeline_description);

        let compute_pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("velocity_calculcation_pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &cs_module,
            entry_point: "cs_main",
        });

        let vertex_positions = [
            vec2(-0.5, -1.0),
            vec2(0.5, -1.0),
            vec2(0.0, 1.0),
            vec2(-1.0, 0.0),
            vec2(1.0, 0.0),
            vec2(0.0, 1.0),
        ];

        let index_data: &[u32] = &[0, 1, 2, 3, 4, 5];
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("velocity_field_arrow_vertex_buffer"),
            contents: unsafe { vertex_positions.align_to::<u8>().1 },
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("velocity_field_arrow_index_buffer"),
            contents: unsafe { index_data.align_to::<u8>().1 },
            usage: wgpu::BufferUsages::INDEX,
        });

        Self {
            render_pipeline,
            density_render_pipeline,
            compute_pipeline,
            uniform_bind_group,
            compute_uniform_bind_group,
            vertex_buffer,
            index_buffer,
            _velocity_buffer: velocity_buffer,
            _constants_buffer: constants_buffer,
            forced_velocity: vec2(0.0, 0.0),
            forced_density: 0.0,
        }
    }

    pub fn add_forces_in_field_to_graph<'node>(&'node self, graph: &mut rend3::RenderGraph<'node>) {
        let mut builder = graph.add_node("fluid_simulator_add_forces_and_density");

        let _data_output = builder.add_data_output::<_, wgpu::Buffer>("Fluid Fields");

        builder.build(
            move |_pt, _renderer, encoder_or_pass, _temps, _ready, _graph_data| {
                let encoder = encoder_or_pass.get_encoder();

                let mut c_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("velocity_calculation_compute_pass"),
                });

                c_pass.push_debug_group("velocity_calculation_compute");
                c_pass.set_pipeline(&self.compute_pipeline);
                c_pass.set_bind_group(0, &self.compute_uniform_bind_group, &[]);
                c_pass.set_push_constants(0, bytemuck::cast_slice(&[PushConstants{ forced_velocity: self.forced_velocity, forced_density: self.forced_density }]));
                c_pass.dispatch((VELOCITY_BUFFER_SIZE as u32 + 31) / 32, 1, 1);
                c_pass.pop_debug_group();

                //graph_data.set_data(data_output, Some(&self._velocity_buffer));
            },
        );
    }

    pub fn add_velocity_visualization_to_graph<'node>(
        &'node self,
        graph: &mut rend3::RenderGraph<'node>,
    ) {
        let mut builder = graph.add_node("velocity_field_visualize");

        let output_handle = builder.add_surface_output();
        let _data_input_handle = builder.add_data_input::<_, wgpu::Buffer>("Fluid Fields");

        builder.build(
            move |_pt, _renderer, encoder_or_pass, _temps, _ready, graph_data| {
                let encoder = encoder_or_pass.get_encoder();

                let output = graph_data.get_render_target(output_handle);

                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    color_attachments: &[wgpu::RenderPassColorAttachment {
                        view: output,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: true,
                        },
                    }],
                    depth_stencil_attachment: None,
                    label: Some("velocity_field_visualize_render_pass"),
                });
                pass.push_debug_group("velocity_field_visualize");
                pass.set_pipeline(&self.render_pipeline);

                pass.set_bind_group(0, &self.uniform_bind_group, &[]);

                pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                pass.draw_indexed(0..6, 0, 0..(GRID_SIZE_X * GRID_SIZE_Y) as u32);

                pass.pop_debug_group();
            },
        );
    }

    pub fn add_density_visualization_to_graph<'node>(
        &'node self,
        graph: &mut rend3::RenderGraph<'node>,
    ) {
        let mut builder = graph.add_node("density_field_visualize");

        let output_handle = builder.add_surface_output();
        let _data_input_handle = builder.add_data_input::<_, wgpu::Buffer>("Fluid Fields");

        builder.build(
            move |_pt, _renderer, encoder_or_pass, _temps, _ready, graph_data| {
                let encoder = encoder_or_pass.get_encoder();

                let output = graph_data.get_render_target(output_handle);

                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    color_attachments: &[wgpu::RenderPassColorAttachment {
                        view: output,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: true,
                        },
                    }],
                    depth_stencil_attachment: None,
                    label: Some("density_field_visualize_render_pass"),
                });
                pass.push_debug_group("density_field_visualize");
                pass.set_pipeline(&self.density_render_pipeline);

                pass.set_bind_group(0, &self.uniform_bind_group, &[]);

                pass.draw(0..3, 0..1);

                pass.pop_debug_group();
            },
        );
    }
}
