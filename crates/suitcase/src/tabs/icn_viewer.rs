use crate::rendering::icn_renderer::ICNRenderer;
use crate::{
    components::{buttons::CustomButtons, dialogs::Dialogs},
    rendering::orbit_camera::OrbitCamera,
    tabs::Tab,
    VirtualFile,
};
use cgmath::Vector3;
use eframe::egui::load::SizedTexture;
use eframe::egui::{vec2, ColorImage, ComboBox, Grid, Id, Image, ImageData, ImageSource, Sense, Stroke, TextureId, TextureOptions, WidgetText};
use eframe::{
    egui,
    egui::{include_image, menu, Color32, Ui},
    egui_glow,
};
use egui_dock::{DockArea, DockState, NodeIndex, SurfaceIndex, TabViewer};
use image::ImageReader;
use ps2_filetypes::{color::Color, BinReader, BinWriter, ColorF, ICNWriter, IconSys, Vector, ICN};
use std::time::Instant;
use std::{
    fs::File,
    io::Write,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use relative_path::PathExt;
use crate::data::state::AppState;
use crate::tabs::PS2RgbaInterface;

enum ICNTab {
    IconProperties,
    IconSysProperties,
}

struct ICNTabViewer<'a> {
    added_tabs: &'a mut Vec<ICNTab>,
    texture: Option<TextureId>,
}

impl<'a> TabViewer for ICNTabViewer<'a> {
    type Tab = ICNTab;

    fn title(&mut self, tab: &mut Self::Tab) -> WidgetText {
        match tab {
            ICNTab::IconProperties => "Properties",
            ICNTab::IconSysProperties => "Icon.sys",
        }
        .into()
    }

    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        match tab {
            ICNTab::IconProperties => {
                Grid::new("icon_properties_grid")
                    .num_columns(2)
                    .spacing(vec2(4.0, 4.0))
                    .show(ui, |ui| {
                        ui.label("Texture");
                        if let Some(texture) = self.texture {
                            ui.image(ImageSource::Texture(SizedTexture::new(
                                texture,
                                vec2(128.0, 128.0),
                            )));
                        }
                        ui.end_row();
                        ui.label("Compression");
                        ComboBox::from_id_salt("compression").selected_text("On").show_ui(ui, |ui| {
                            ui.selectable_label(true, "On");
                        })
                    });
            }
            ICNTab::IconSysProperties => {
                ui.heading("Icon.sys");
            }
        }
    }

    fn id(&mut self, tab: &mut Self::Tab) -> Id {
        Id::new(
            "icon_".to_owned()
                + match tab {
                    ICNTab::IconProperties => "props",
                    ICNTab::IconSysProperties => "sys",
                },
        )
    }

    fn add_popup(&mut self, ui: &mut Ui, _surface: SurfaceIndex, _node: NodeIndex) {
        ui.set_min_width(120.0);
        ui.style_mut().visuals.button_frame = false;

        if ui.button("Properties").clicked() {
            self.added_tabs.push(ICNTab::IconProperties {});
        }

        if ui.button("Icon.sys").clicked() {
            self.added_tabs.push(ICNTab::IconSysProperties);
        }
    }
}

pub struct ICNViewer {
    dock_state: DockState<ICNTab>,
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
    pub texture: Option<TextureId>,
    pub background_colors: [Color32; 4],
    light_colors: [ColorF; 3],
    light_positions: [Vector; 3],
    ambient_color: ColorF,
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
    pub fn new(file: &VirtualFile, state: &AppState) -> Self {
        let mut background_colors = [Color32::DARK_GRAY; 4];
        let mut light_colors = [ColorF{r: 0.0, g: 0.0, b: 0.0, a: 0.0}; 3];
        let mut light_positions = [Vector{x: 0.0, y: 0.0, z: 0.0, w: 0.0}; 3];
        let mut ambient_color = ColorF{r: 0.1, g: 0.1, b: 0.1, a: 0.0};

        let buf = std::fs::read(&file.file_path).expect("File not found");
        let icon_sys = file.file_path.clone().parent().unwrap().join("icon.sys");

        if icon_sys.exists() {
            let icon_sys = IconSys::new(std::fs::read(icon_sys).unwrap());
            background_colors = icon_sys.background_colors.map(|c| PS2RgbaInterface::build_from_color(c).into());
            light_colors = icon_sys.light_colors;
            light_positions = icon_sys.light_directions;
        }

        let icn = ps2_filetypes::ICNParser::read(&buf.clone()).unwrap();
        let mut dock_state =
            DockState::new(vec![ICNTab::IconProperties]);

        dock_state.main_surface_mut().split_below(NodeIndex::root(), 0.5, vec![ICNTab::IconSysProperties]);

        Self {
            dock_state,
            renderer: Arc::new(Mutex::new(None)),
            file: file
                .file_path
                .relative_to(state.opened_folder.clone().unwrap())
                .unwrap()
                .to_string(),
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
            texture: None,
            background_colors,
            light_colors,
            light_positions,
            ambient_color,
        }
    }

    fn make_texture(&mut self, ui: &mut Ui) {
        let mut pixels = vec![];
        for pixel in self.icn.texture.pixels {
            let color: Color = pixel.into();
            pixels.extend_from_slice(&[color.r, color.g, color.b]);
        }

        let image_data = ImageData::from(ColorImage::from_rgb([128, 128], &pixels));

        let id = ui.ctx().tex_manager().write().alloc(
            self.file.clone(),
            image_data,
            TextureOptions::default(),
        );

        self.texture = Some(id);
    }

    fn custom_painting(&mut self, ui: &mut Ui) {
        let (rect, response) = ui.allocate_exact_size(ui.available_size(), egui::Sense::drag());
        let input = ui.input(|i| i.clone());

        let mut delta_yaw = 0.0;
        let mut delta_pitch = 0.0;
        let mut delta_zoom = 0.0;

        if response.dragged()
        {
            let delta = response.drag_delta();
            delta_yaw -= delta.x * 0.01;
            delta_pitch += delta.y * 0.01;
        }

        if response.contains_pointer() && input.raw_scroll_delta.y != 0.0 {
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
        let light_colors = self.light_colors.clone();
        let light_positions = self.light_positions.clone();
        let ambient_color = self.ambient_color.clone();

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
                    renderer.paint(painter.gl(), aspect_ratio, camera, frame, light_colors, light_positions, ambient_color);
                }
            })),
        };

        if needs_update {
            self.needs_update = false;
        }

        ui.painter().add(callback);
    }

    pub fn show(&mut self, ui: &mut Ui) {
        let mut tabs = vec![];
        if self.playing {
            let elapsed = self.start_time.elapsed().as_secs_f32();
            let loop_duration = self.icn.animation_header.frame_length as f32 / 60.0;
            let time_in_cycle = elapsed % loop_duration;
            self.frame = (time_in_cycle * 60.0).floor() as u32;
            ui.ctx().request_repaint();
        }

        if self.texture.is_none() {
            self.make_texture(ui);
        }

        egui::SidePanel::right("icn_properties").show_inside(ui, |ui| {
            DockArea::new(&mut self.dock_state)
                .id(Id::new(&self.file).with("properties"))
                .show_leaf_close_all_buttons(false)
                .show_leaf_collapse_buttons(false)
                .show_add_buttons(true)
                .show_add_popup(true)
                .show_inside(
                    ui,
                    &mut ICNTabViewer {
                        added_tabs: &mut tabs,
                        texture: self.texture,
                    },
                );
        });
        ui.vertical(|ui| {
            menu::bar(ui, |ui| {
                ui.set_height(50.0);
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

                if ui.button("Reset View").clicked() {
                    self.camera.reset_view();
                }

                ui.checkbox(&mut self.dark_mode, "Dark Mode");
            });

            let fill = if self.dark_mode {
                ui.style().visuals.code_bg_color
            } else {
                Color32::from_rgb(0xAA, 0xAA, 0xAA)
            };
            ui.vertical(|ui| {
                ui.set_height(ui.available_size_before_wrap().y - 28.0);

                egui::Frame::canvas(ui.style()).stroke(Stroke::NONE).corner_radius(0).show(ui, |ui| {
                    draw_background(ui, &self.background_colors);
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

        tabs.drain(..).for_each(|tab| {
            self.dock_state.push_to_focused_leaf(tab);
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

fn draw_background(ui: &mut Ui, colors: &[Color32; 4]) {
    let rect = ui.available_rect_before_wrap();
    let painter = ui.painter_at(rect);

    let top_left = rect.left_top();
    let top_right = rect.right_top();
    let bottom_left = rect.left_bottom();
    let bottom_right = rect.right_bottom();

    let mut mesh = egui::epaint::Mesh::default();

    let i0 = mesh.vertices.len() as u32;
    mesh.vertices.push(egui::epaint::Vertex {
        pos: top_left,
        uv: egui::epaint::WHITE_UV,
        color: colors[0],
    });
    mesh.vertices.push(egui::epaint::Vertex {
        pos: top_right,
        uv: egui::epaint::WHITE_UV,
        color: colors[1],
    });
    mesh.vertices.push(egui::epaint::Vertex {
        pos: bottom_right,
        uv: egui::epaint::WHITE_UV,
        color: colors[3],
    });
    mesh.vertices.push(egui::epaint::Vertex {
        pos: bottom_left,
        uv: egui::epaint::WHITE_UV,
        color: colors[2],
    });

    mesh.indices
        .extend_from_slice(&[i0, i0 + 1, i0 + 2, i0, i0 + 2, i0 + 3]);

    painter.add(egui::Shape::mesh(mesh));
}
