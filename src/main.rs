mod app;
mod parser;
mod playback;
mod renderer;
mod ui;

fn main() -> eframe::Result<()> {
    let wgpu_options = egui_wgpu::WgpuConfiguration {
        wgpu_setup: egui_wgpu::WgpuSetup::CreateNew(
            egui_wgpu::WgpuSetupCreateNew {
                device_descriptor: std::sync::Arc::new(|_adapter| {
                    wgpu::DeviceDescriptor {
                        required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
                        ..Default::default()
                    }
                }),
                ..egui_wgpu::WgpuSetupCreateNew::without_display_handle()
            },
        ),
        ..Default::default()
    };
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1400.0, 900.0])
            .with_drag_and_drop(true),
        wgpu_options,
        ..Default::default()
    };
    eframe::run_native(
        "Visual G-Code",
        options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
}
