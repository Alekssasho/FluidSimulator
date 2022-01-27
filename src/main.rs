use glam::*;
use winit::{
    self,
    event::Event,
    event::WindowEvent,
    event_loop::{ControlFlow, EventLoop},
    platform::run_return::EventLoopExtRunReturn,
    window::WindowBuilder,
};

use std::collections::VecDeque;

use kajiya::{
    backend::{
        ash::vk,
        file::{set_standard_vfs_mount_points, set_vfs_mount_point},
        vulkan::RenderBackendConfig,
        RenderBackend,
    },
    rg::{self, ImageDesc},
    ui_renderer::UiRenderer,
};

fn main() -> anyhow::Result<()> {
    set_standard_vfs_mount_points("./kajiya");
    set_vfs_mount_point("/shaders-new", "./shaders");

    //kajiya::logging::set_up_logging(builder.default_log_level)?;

    let rendering_width = 1920;
    let rendering_height = 1080;
    std::env::set_var("SMOL_THREADS", "64"); // HACK; TODO: get a real executor
    let window_builder = WindowBuilder::new()
        .with_title("hello-kajiya")
        .with_resizable(false)
        .with_inner_size(winit::dpi::LogicalSize::new(
            rendering_width as f64,
            rendering_height as f64,
        ));

    let mut event_loop = EventLoop::new();

    let window = window_builder.build(&event_loop).expect("window");

    // Physical window extent in pixels
    let swapchain_extent = [window.inner_size().width, window.inner_size().height];

    let mut render_backend = RenderBackend::new(
        &window,
        RenderBackendConfig {
            swapchain_extent,
            vsync: true,
            graphics_debugging: false,
        },
    )?;

    let mut ui_renderer = UiRenderer::default();

    let mut rg_renderer = kajiya::rg::renderer::Renderer::new(&render_backend)?;

    let mut imgui = imgui::Context::create();

    let mut imgui_backend =
        kajiya_imgui::ImGuiBackend::new(rg_renderer.device().clone(), &window, &mut imgui);

    imgui_backend.create_graphics_resources(swapchain_extent);

    let mut events = Vec::new();

    let mut last_frame_instant = std::time::Instant::now();
    let mut last_error_text = None;

    // Delta times are filtered over _this many_ frames.
    const DT_FILTER_WIDTH: usize = 10;

    // Past delta times used for filtering
    let mut dt_queue: VecDeque<f32> = VecDeque::with_capacity(DT_FILTER_WIDTH);

    // Fake the first frame's delta time. In the first frame, shaders
    // and pipelines are be compiled, so it will most likely have a spike.
    let mut fake_dt_countdown: i32 = 1;

    let mut running = true;
    while running {
        event_loop.run_return(|event, _, control_flow| {
            let _ = &render_backend;
            imgui_backend.handle_event(&window, &mut imgui, &event);

            let ui_wants_mouse = imgui.io().want_capture_mouse;

            *control_flow = ControlFlow::Poll;

            match event {
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::CloseRequested => {
                        *control_flow = ControlFlow::Exit;
                        running = false;
                    }
                    WindowEvent::CursorMoved { .. } | WindowEvent::MouseInput { .. }
                        if ui_wants_mouse => {}
                    _ => events.extend(event.to_static()),
                },
                Event::MainEventsCleared => {
                    *control_flow = ControlFlow::Exit;
                }
                _ => (),
            }
        });

        let dt_filtered = {
            let now = std::time::Instant::now();
            let dt_duration = now - last_frame_instant;
            last_frame_instant = now;

            let dt_raw = dt_duration.as_secs_f32();

            // >= because rendering (and thus the spike) happens _after_ this.
            if fake_dt_countdown >= 0 {
                // First frame. Return the fake value.
                fake_dt_countdown -= 1;
                dt_raw.min(1.0 / 60.0)
            } else {
                // Not the first frame. Start averaging.

                if dt_queue.len() >= DT_FILTER_WIDTH {
                    dt_queue.pop_front();
                }

                dt_queue.push_back(dt_raw);
                dt_queue.iter().copied().sum::<f32>() / dt_queue.len() as f32
            }
        };

        let ui = imgui_backend.prepare_frame(&window, &mut imgui, dt_filtered);
        imgui::Window::new(imgui::im_str!("Hello world"))
            .size([300.0, 110.0], imgui::Condition::FirstUseEver)
            .build(&ui, || {
                ui.text_wrapped(imgui::im_str!("Hello world!"));
                ui.text_wrapped(imgui::im_str!("こんにちは世界！"));

                ui.button(imgui::im_str!("This...is...imgui-rs!"), [10.0, 10.0]);
                ui.separator();
                let mouse_pos = ui.io().mouse_pos;
                ui.text(format!(
                    "Mouse Position: ({:.1},{:.1})",
                    mouse_pos[0], mouse_pos[1]
                ));
            });
        imgui_backend.finish_frame(ui, &window, &mut ui_renderer);

        events.clear();

        // Physical window extent in pixels
        let swapchain_extent = [window.inner_size().width, window.inner_size().height];

        let prepared_frame = {
            rg_renderer.prepare_frame(|rg| {
                //let main_img = world_renderer.prepare_render_graph(rg, &frame_desc);
                let ui_img = ui_renderer.prepare_render_graph(rg);

                let mut main_img = rg.create(ImageDesc::new_2d(
                    vk::Format::R8G8B8A8_UNORM,
                    swapchain_extent,
                ));
                rg::imageops::clear_color(rg, &mut main_img, [0.0f32; 4]);
                let mut swap_chain = rg.get_swap_chain();
                rg::SimpleRenderPass::new_compute(
                    rg.add_pass("final blit"),
                    "/shaders-new/final_blit.hlsl",
                )
                .read(&main_img)
                .read(&ui_img)
                .write(&mut swap_chain)
                .constants(([
                    swapchain_extent[0] as f32,
                    swapchain_extent[1] as f32,
                    1.0 / swapchain_extent[0] as f32,
                    1.0 / swapchain_extent[1] as f32,
                ],))
                .dispatch([swapchain_extent[0], swapchain_extent[1], 1]);
            })
        };

        match prepared_frame {
            Ok(()) => {
                rg_renderer.draw_frame(
                    |_dynamic_constants| rg::renderer::FrameConstantsLayout {
                        globals_offset: 0,
                        instance_dynamic_parameters_offset: 0,
                        triangle_lights_offset: 0,
                    },
                    &mut render_backend.swapchain,
                );
                last_error_text = None;
            }
            Err(e) => {
                let error_text = Some(format!("{:?}", e));
                if error_text != last_error_text {
                    println!("{}", error_text.as_ref().unwrap());
                    last_error_text = error_text;
                }
            }
        }
    }
    Ok(())
}
