use std::{sync::Arc, time::Instant};

use egui_winit_platform::{Platform, PlatformDescriptor};
use glam::UVec2;
use winit::{event::Event::*, event_loop::ControlFlow};

mod velocity_field_routine;

fn main() {
    // Create event loop and window
    let event_loop = winit::event_loop::EventLoop::new();
    let window = {
        let mut builder = winit::window::WindowBuilder::new();
        builder = builder.with_title("Fluid Simulator");
        builder.build(&event_loop).expect("Could not build window")
    };

    let window_size = window.inner_size();

    // Create the Instance, Adapter, and Device. We can specify preferred backend, device name, or rendering mode. In this case we let rend3 choose for us.
    let iad = futures::executor::block_on(rend3::create_iad(None, None, None)).unwrap();

    // The one line of unsafe needed. We just need to guarentee that the window outlives the use of the surface.
    let surface = unsafe { Arc::new(iad.instance.create_surface(&window)) };
    // Get the preferred format for the surface.
    let format = surface.get_preferred_format(&iad.adapter).unwrap();
    // Configure the surface to be ready for rendering.
    rend3::configure_surface(
        &surface,
        &iad.device,
        format,
        UVec2::new(window_size.width, window_size.height),
        rend3::types::PresentMode::Mailbox,
    );

    // Make us a renderer.
    let renderer = rend3::Renderer::new(
        iad,
        Some(window_size.width as f32 / window_size.height as f32),
    )
    .unwrap();

    // Create the egui render egui_routine
    let mut egui_routine = rend3_egui::EguiRenderRoutine::new(
        &renderer,
        format,
        1, // For now this has to be 1, until rendergraphs support multisampling
        window_size.width,
        window_size.height,
        window.scale_factor() as f32,
    );

    let mut velocity_field_routine =
        velocity_field_routine::VelocityFieldRoutine::new(&renderer, format);

    let camera_pitch = std::f32::consts::FRAC_PI_4;
    let camera_yaw = -std::f32::consts::FRAC_PI_4;
    // These values may seem arbitrary, but they center the camera on the cube in the scene
    let camera_location = glam::Vec3A::new(5.0, 7.5, -5.0);
    let view = glam::Mat4::from_euler(glam::EulerRot::XYZ, -camera_pitch, -camera_yaw, 0.0);
    let view = view * glam::Mat4::from_translation((-camera_location).into());

    // Set camera location data
    renderer.set_camera_data(rend3::types::Camera {
        projection: rend3::types::CameraProjection::Perspective {
            vfov: 60.0,
            near: 0.1,
        },
        view,
    });

    // We use the egui_winit_platform crate as the platform.
    let mut platform = Platform::new(PlatformDescriptor {
        physical_width: window_size.width as u32,
        physical_height: window_size.height as u32,
        scale_factor: window.scale_factor(),
        font_definitions: egui::FontDefinitions::default(),
        style: Default::default(),
    });

    let start_time = Instant::now();

    event_loop.run(move |event, _, control_flow| {
        // Pass the winit events to the platform integration.
        platform.handle_event(&event);

        match event {
            RedrawRequested(..) => {
                platform.update_time(start_time.elapsed().as_secs_f64());
                platform.begin_frame();

                // Insert egui commands here
                let ctx = platform.context();
                egui::Window::new("Settings")
                    .resizable(true)
                    .show(&ctx, |_ui| {});

                // End the UI frame. Now let's draw the UI with our Backend, we could also handle the output here
                let (_output, paint_commands) = platform.end_frame(Some(&window));
                let paint_jobs = platform.context().tessellate(paint_commands);

                let input = rend3_egui::Input {
                    clipped_meshes: &paint_jobs,
                    context: platform.context(),
                };

                // Get a frame
                let frame = rend3::util::output::OutputFrame::Surface {
                    surface: Arc::clone(&surface),
                };

                // Ready up the renderer
                let (cmd_bufs, ready) = renderer.ready();

                // Build a rendergraph
                let mut graph = rend3::RenderGraph::new();

                velocity_field_routine.add_to_graph(&mut graph);

                // Add egui on top of all the other passes
                egui_routine.add_to_graph(&mut graph, input);

                // Dispatch a render using the built up rendergraph!
                graph.execute(&renderer, frame, cmd_bufs, &ready);

                *control_flow = ControlFlow::Poll;
            }
            MainEventsCleared => {
                window.request_redraw();
            }
            WindowEvent { event, .. } => match event {
                winit::event::WindowEvent::Resized(size) => {
                    let size = UVec2::new(size.width, size.height);
                    // Reconfigure the surface for the new size.
                    rend3::configure_surface(
                        &surface,
                        &renderer.device,
                        format,
                        UVec2::new(size.x, size.y),
                        rend3::types::PresentMode::Mailbox,
                    );

                    renderer.set_aspect_ratio(size.x as f32 / size.y as f32);

                    egui_routine.resize(size.x, size.y, window.scale_factor() as f32);
                }
                winit::event::WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit;
                }
                _ => {}
            },
            _ => {}
        }
    });
}
