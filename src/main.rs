use fluid_simulator::*;

fn main() -> anyhow::Result<()> {
    set_standard_vfs_mount_points("./kajiya");
    set_vfs_mount_point("/shaders-new", "./shaders");
    let kajiya = SimpleMainLoop::builder().resolution([1920, 1080]).build(
        WindowBuilder::new()
            .with_title("hello-kajiya")
            .with_resizable(false),
    )?;

    kajiya.run(move |ctx| {
        ctx.imgui.unwrap().frame(|ui| {
            imgui::Window::new(imgui::im_str!("Hello world"))
                .size([300.0, 110.0], imgui::Condition::FirstUseEver)
                .build(ui, || {
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
        });
    })
}
