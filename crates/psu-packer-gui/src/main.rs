#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::{egui, egui::IconData, NativeOptions, Renderer};
use psu_packer_gui::PackerApp;
use std::any::Any;
use std::fmt;
use std::panic::{self, AssertUnwindSafe};

fn main() -> eframe::Result<()> {
    let wgpu_result = run_app_with_renderer(Renderer::Wgpu);

    match wgpu_result {
        Ok(result) => Ok(result),
        Err(wgpu_error) => {
            report_renderer_error("WGPU", &wgpu_error);

            let glow_result = run_app_with_renderer(Renderer::Glow);
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
    let mut options = shared_native_options();
    options.renderer = renderer;
    options
}

fn shared_native_options() -> NativeOptions {
    let mut options = NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 768.0])
            .with_min_inner_size([640.0, 480.0])
            .with_icon(load_app_icon())
            .with_resizable(true),
        ..Default::default()
    };

    options.centered = true;
    options
}

fn load_app_icon() -> IconData {
    let icon = include_bytes!("../../suitcase/assets/psupackergui.ico");
    match image::load_from_memory(icon) {
        Ok(image) => {
            let image = image.into_rgba8();
            let (width, height) = image.dimensions();

            IconData {
                rgba: image.into_raw(),
                width,
                height,
            }
        }
        Err(error) => {
            eprintln!("Failed to load icon: {error}");
            IconData {
                rgba: vec![0; 4],
                width: 1,
                height: 1,
            }
        }
    }
}

fn run_app(options: NativeOptions) -> eframe::Result<()> {
    eframe::run_native(
        "PSU Packer",
        options,
        Box::new(|cc| Ok(Box::new(PackerApp::new(cc)))),
    )
}

fn run_app_with_renderer(renderer: Renderer) -> eframe::Result<()> {
    let options = create_native_options(renderer);
    match panic::catch_unwind(AssertUnwindSafe(|| run_app(options))) {
        Ok(result) => result,
        Err(payload) => Err(panic_payload_to_error(payload)),
    }
}

fn panic_payload_to_error(payload: Box<dyn Any + Send>) -> eframe::Error {
    let message = panic_message(payload);
    eframe::Error::AppCreation(Box::new(PanicAppError(message)))
}

fn panic_message(payload: Box<dyn Any + Send>) -> String {
    let payload_ref = &*payload;
    if let Some(message) = payload_ref.downcast_ref::<&str>() {
        (*message).to_owned()
    } else if let Some(message) = payload_ref.downcast_ref::<String>() {
        message.clone()
    } else {
        "Unknown panic".to_owned()
    }
}

#[derive(Debug)]
struct PanicAppError(String);

impl fmt::Display for PanicAppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for PanicAppError {}

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
