use std::sync::Arc;

use kajiya::{
    backend::{
        ash::vk::{BufferUsageFlags, Handle},
        Device,
    },
    rg::{self, Buffer, BufferDesc, Image},
};

const GRID_SIZE_X: usize = 200;
const GRID_SIZE_Y: usize = 200;

#[derive(Clone, Copy)]
struct ConstantData {
    grid_size: glam::IVec2,
}

unsafe impl bytemuck::Pod for ConstantData {}
unsafe impl bytemuck::Zeroable for ConstantData {}
pub struct FluidSimulator {
    device: Arc<Device>,
    density_buffer: Arc<Buffer>,
    constant_data: ConstantData,
}

#[derive(Default)]
pub struct FluidSimulatorSettings {
    pub add_density_here_debug: Option<glam::IVec2>,
}

impl FluidSimulator {
    pub fn new(device: Arc<Device>) -> Self {
        let constant_data = ConstantData {
            grid_size: glam::ivec2(GRID_SIZE_X as i32, GRID_SIZE_Y as i32),
        };
        let density_buffer = Arc::new(
            device
                .create_buffer(
                    BufferDesc {
                        size: GRID_SIZE_X * GRID_SIZE_Y * std::mem::size_of::<f32>(),
                        usage: BufferUsageFlags::STORAGE_BUFFER,
                        mapped: false,
                    },
                    None
                )
                .unwrap(),
        );
        Self {
            device,
            density_buffer,
            constant_data
        }
    }

    pub fn prepare_render_graph(&self, rg: &mut rg::TemporalRenderGraph, config: &FluidSimulatorSettings) -> rg::Handle<Image> {
        let extent = [1920, 1080];
        let mut main_img = rg.create(rg::ImageDesc::new_2d(
            kajiya::backend::ash::vk::Format::R8G8B8A8_UNORM,
            extent,
        ));
        rg::imageops::clear_color(rg, &mut main_img, [0.0f32; 4]);

        let mut density_buffer = rg.import(
            self.density_buffer.clone(),
            kajiya::backend::vk_sync::AccessType::ComputeShaderWrite,
        );

        if let Some(pos) = config.add_density_here_debug {
            rg::SimpleRenderPass::new_compute(
                rg.add_pass("add force"),
                "/shaders-new/add_force.hlsl",
            )
            .constants(self.constant_data)
            .write(&mut density_buffer)
            .constants((pos, [extent[0] as f32, extent[1] as f32]))
            .dispatch([1, 1, 1]);
        }

        rg::SimpleRenderPass::new_compute(
            rg.add_pass("visualize density"),
            "/shaders-new/density_visualize.hlsl",
        )
        .constants(self.constant_data)
        .read(&density_buffer)
        .write(&mut main_img)
        .constants((
            [extent[0] as f32, extent[1] as f32],
        ))
        .dispatch([extent[0], extent[1], 1]);

        main_img
    }
}
