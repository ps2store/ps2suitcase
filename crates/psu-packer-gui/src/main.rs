#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::{egui, NativeOptions, Renderer};
use psu_packer_gui::PackerApp;

trait NativeOptionsExt {
    fn with_centered(self, centered: bool) -> Self;
}

impl NativeOptionsExt for eframe::NativeOptions {
    fn with_centered(mut self, centered: bool) -> Self {
        self.centered = centered;
        self
    }
}

fn main() -> eframe::Result<()> {
    let wgpu_result = run_app(create_native_options(Renderer::Wgpu));

    match wgpu_result {
        Ok(result) => Ok(result),
        Err(wgpu_error) => {
            report_renderer_error("WGPU", &wgpu_error);

            let glow_result = run_app(create_native_options(Renderer::Glow));
            match glow_result {
                Ok(result) => Ok(result),
                Err(glow_error) => {
                    report_renderer_error("Glow", &glow_error);
                    Err(wgpu_error)
                }
            }
        }
    }
}

fn create_native_options(renderer: Renderer) -> NativeOptions {
    NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 768.0])
            .with_min_inner_size([1024.0, 768.0])
            .with_max_inner_size([1024.0, 768.0])
            .with_resizable(false),
        renderer,
        ..Default::default()
    }
    .with_centered(true)
}

fn run_app(options: NativeOptions) -> eframe::Result<()> {
    eframe::run_native(
        "PSU Packer",
        options,
        Box::new(|cc| Box::new(PackerApp::new(cc))),
    )
}

fn report_renderer_error(renderer: &str, error: &eframe::Error) {
    eprintln!("Failed to initialize {renderer} renderer: {error}");

    #[cfg(target_os = "windows")]
    {
        use rfd::MessageDialog;

        MessageDialog::new()
            .set_title("PSU Packer")
            .set_description(&format!(
                "Failed to initialize {renderer} renderer:\n{error}\n\nAttempting fallback..."
            ))
            .set_buttons(rfd::MessageButtons::Ok)
            .show();
    }
}
