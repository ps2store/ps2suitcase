use crate::rendering::icn_renderer::ICNRenderer;
use crate::{
    components::{buttons::CustomButtons, dialogs::Dialogs},
    rendering::orbit_camera::OrbitCamera,
    tabs::Tab,
    VirtualFile,
};
use cgmath::Vector3;
use eframe::{
    egui,
    egui::{include_image, menu, Color32, Ui},
    egui_glow,
};
use image::ImageReader;
use ps2_filetypes::{color::Color, BinReader, BinWriter, ICNWriter, ICN};
use std::time::Instant;
use std::{
    fs::File,
    io::Write,
    path::PathBuf,
    sync::{Arc, Mutex},
};

pub struct ICNViewer {
    renderer: Arc<Mutex<Option<ICNRenderer>>>,
    file: String,
    path: PathBuf,
    camera: OrbitCamera,
    icn: ICN,
    dark_mode: bool,
    needs_update: bool,
    modified: bool,
    pub closing: bool,
    frame: u32,
    start_time: Instant,
    pub playing: bool,
}

impl ICNViewer {
    fn replace_texture(&mut self, path: PathBuf) {
        let image = ImageReader::open(&path).unwrap().decode().unwrap();

        let image = image.to_rgba8();
        self.icn.texture.pixels = image
            .pixels()
            .map(|p| Color::new(p.0[0], p.0[1], p.0[2], 255).into())
            .collect::<Vec<u16>>()
            .try_into()
            .unwrap();
        self.needs_update = true;
        self.modified = true;
    }
}

impl ICNViewer {
    pub fn new(file: &VirtualFile) -> Self {
        let buf = std::fs::read(&file.file_path).expect("File not found");
        let icn = ps2_filetypes::ICNParser::read(&buf.clone()).unwrap();

        Self {
            renderer: Arc::new(Mutex::new(None)),
            file: file.name.clone(),
            path: file.file_path.clone(),
            camera: OrbitCamera {
                target: Vector3::new(0.0, 2.5, 0.0),
                distance: 10.0,
                yaw: 0.0,
                pitch: 0.0,
                min_distance: 1.0,
                max_distance: 50.0,
            },
            icn,
            dark_mode: true,
            needs_update: false,
            modified: false,
            closing: false,
            frame: 0,
            start_time: Instant::now(),
            playing: false,
        }
    }
    fn custom_painting(&mut self, ui: &mut Ui) {
        let (rect, _) = ui.allocate_exact_size(ui.available_size(), egui::Sense::drag());
        let input = ui.input(|i| i.clone());

        let mut delta_yaw = 0.0;
        let mut delta_pitch = 0.0;
        let mut delta_zoom = 0.0;

        if input.pointer.primary_down()
            && input
                .pointer
                .interact_pos()
                .is_some_and(|p| rect.contains(p))
        {
            let delta = input.pointer.delta();
            delta_yaw -= delta.x * 0.01;
            delta_pitch += delta.y * 0.01;
        }

        if input.raw_scroll_delta.y != 0.0 {
            delta_zoom += input.raw_scroll_delta.y * 0.1;
        }

        self.camera.update(delta_yaw, delta_pitch, delta_zoom);

        let renderer = self.renderer.clone();

        let icn = self.icn.clone();
        let needs_update = self.needs_update;
        let closing = self.closing;
        let camera = self.camera.clone();
        let aspect_ratio = rect.width() / rect.height();
        let frame = self.frame;

        let callback = egui::PaintCallback {
            rect,
            callback: Arc::new(egui_glow::CallbackFn::new(move |_, painter| {
                let mut renderer = renderer.lock().unwrap();
                let renderer = renderer.get_or_insert_with(|| {
                    ICNRenderer::new(painter.gl(), &icn).expect("Failed to create ICNRenderer")
                });
                if needs_update {
                    renderer.replace_texture(painter.gl(), &icn);
                }

                if closing {
                    renderer.drop(painter.gl())
                } else {
                    renderer.paint(painter.gl(), aspect_ratio, camera, frame);
                }
            })),
        };

        if needs_update {
            self.needs_update = false;
        }

        ui.painter().add(callback);
    }

    pub fn show(&mut self, ui: &mut Ui) {
        if self.playing {
            let elapsed = self.start_time.elapsed().as_secs_f32();
            let loop_duration = self.icn.animation_header.frame_length as f32 / 60.0;
            let time_in_cycle = elapsed % loop_duration;
            self.frame = (time_in_cycle * 60.0).floor() as u32;
            ui.ctx().request_repaint();
        }
        ui.vertical(|ui| {
            menu::bar(ui, |ui| {
                if ui
                    .icon_text_button(
                        include_image!("../../assets/icons/file-arrow-right.svg"),
                        "Export OBJ",
                    )
                    .clicked()
                {
                    if let Some(path) = ui.ctx().save_as(self.file.clone() + ".obj") {
                        File::create(path)
                            .unwrap()
                            .write_all(self.icn.export_obj().as_bytes())
                            .unwrap();
                    }
                }

                if ui
                    .icon_text_button(
                        include_image!("../../assets/icons/file-arrow-right.svg"),
                        "Export PNG",
                    )
                    .clicked()
                {
                    if let Some(path) = ui.ctx().save_as(self.file.clone() + ".png") {
                        File::create(path)
                            .unwrap()
                            .write_all(&self.icn.export_png())
                            .unwrap();
                    }
                }

                if ui
                    .icon_text_button(
                        include_image!("../../assets/icons/photo-plus.svg"),
                        "Replace Texture",
                    )
                    .clicked()
                {
                    if let Some(path) = ui.ctx().open_file_filter(&["png"]) {
                        self.replace_texture(path);
                    }
                }

                ui.checkbox(&mut self.dark_mode, "Dark Mode");
            });

            let fill = if self.dark_mode {
                ui.style().visuals.window_fill
            } else {
                Color32::from_rgb(0xAA, 0xAA, 0xAA)
            };
            ui.vertical(|ui| {
                ui.set_height(ui.available_size_before_wrap().y - 28.0);

                egui::Frame::canvas(ui.style()).fill(fill).show(ui, |ui| {
                    self.custom_painting(ui);
                });
            });

            if self.icn.animation_header.frame_length > 1 {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    if ui.button("Play/Stop").clicked() {
                        if self.playing {
                            self.playing = false;
                            self.frame = 0;
                        } else {
                            self.playing = true;
                            self.start_time = Instant::now();
                        }
                    }
                    ui.spacing_mut().slider_width =
                        ui.available_width() - ui.spacing().interact_size.x - 9.0;
                    ui.add(egui::Slider::new(
                        &mut self.frame,
                        0..=self.icn.animation_header.frame_length,
                    ));
                });
                ui.add_space(4.0);
            }
        });
    }
}

impl Tab for ICNViewer {
    fn get_id(&self) -> &str {
        &self.file
    }

    fn get_title(&self) -> String {
        self.file.clone()
    }

    fn get_modified(&self) -> bool {
        self.modified
    }

    fn save(&mut self) {
        let mut file = File::create(&self.path).expect("Failed to create file");
        let bytes = ICNWriter::new(self.icn.clone())
            .write()
            .expect("Failed to save file");
        file.write_all(&bytes).expect("Failed to write to file");
        self.modified = false;
    }
}
