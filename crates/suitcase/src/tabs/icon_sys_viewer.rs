use crate::tabs::Tab;
use crate::{AppState, VirtualFile};
use eframe::egui;
use eframe::egui::{
    menu, vec2, Color32, CornerRadius, Grid, Id, PopupCloseBehavior, Response, Rgba, TextEdit, Ui,
};
use ps2_filetypes::color::Color;
use ps2_filetypes::{ColorF, IconSys, Vector};
use relative_path::PathExt;
use std::ops::Add;
use std::path::PathBuf;

#[derive(Copy, Clone)]
pub struct PS2RgbaInterface {
    pub rgb: [f32; 3],
    pub alpha: f32,
}

impl PS2RgbaInterface {
    pub fn build_from_color_f(color_f: ColorF) -> Self {
        Self {
            rgb: [color_f.r, color_f.g, color_f.b],
            alpha: color_f.a,
        }
    }
    pub fn build_from_color(color: Color) -> Self {
        Self {
            rgb: [
                Self::convert_color_to_float(color.r),
                Self::convert_color_to_float(color.g),
                Self::convert_color_to_float(color.b),
            ],
            alpha: Self::convert_color_to_float(color.a),
        }
    }

    pub fn to_color_f(&self) -> ColorF {
        ColorF {
            r: self.rgb[0],
            g: self.rgb[1],
            b: self.rgb[2],
            a: self.alpha,
        }
    }

    pub fn to_color(&self) -> Color {
        Color {
            r: Self::convert_color_to_int(self.rgb[0]),
            g: Self::convert_color_to_int(self.rgb[1]),
            b: Self::convert_color_to_int(self.rgb[2]),
            a: Self::convert_color_to_int(self.alpha),
        }
    }

    pub fn convert_color_to_float(color: u8) -> f32 {
        color as f32 / 255.0
    }

    pub fn convert_color_to_int(color: f32) -> u8 {
        (color * 255.0) as u8
    }
}

impl From<PS2RgbaInterface> for Color32 {
    fn from(value: PS2RgbaInterface) -> Self {
        Rgba::from_rgb(value.rgb[0], value.rgb[1], value.rgb[2]).into()
    }
}

pub struct Light {
    pub color: PS2RgbaInterface,
    pub direction: Vector,
}

impl Light {
    pub fn new(color: ColorF, direction: Vector) -> Self {
        Self {
            color: PS2RgbaInterface::build_from_color_f(color),
            direction,
        }
    }
}

pub struct IconSysViewer {
    title_first_line: String,
    title_second_line: String,
    file: String,
    pub icon_file: String,
    pub icon_copy_file: String,
    pub icon_delete_file: String,
    pub background_transparency: u32,
    pub ambient_color: PS2RgbaInterface,
    pub background_colors: [PS2RgbaInterface; 4],
    pub lights: [Light; 3],
    pub sys: IconSys,
    pub file_path: PathBuf,
}

impl IconSysViewer {
    pub fn new(file: &VirtualFile, state: &AppState) -> Self {
        let buf = std::fs::read(&file.file_path).expect("File not found");

        let sys = IconSys::new(buf);

        let (title_first_line, title_second_line) =
            sys.title.split_at(sys.linebreak_pos as usize).to_owned();

        Self {
            title_first_line: title_first_line.to_string(),
            title_second_line: title_second_line.to_string(),
            icon_file: sys.icon_file.clone(),
            icon_copy_file: sys.icon_copy_file.clone(),
            icon_delete_file: sys.icon_delete_file.clone(),
            background_transparency: sys.background_transparency.clone(),
            ambient_color: PS2RgbaInterface::build_from_color_f(sys.ambient_color),
            background_colors: [
                PS2RgbaInterface::build_from_color(sys.background_colors[0]),
                PS2RgbaInterface::build_from_color(sys.background_colors[1]),
                PS2RgbaInterface::build_from_color(sys.background_colors[2]),
                PS2RgbaInterface::build_from_color(sys.background_colors[3]),
            ],
            lights: [
                Light::new(sys.light_colors[0], sys.light_directions[0]),
                Light::new(sys.light_colors[1], sys.light_directions[1]),
                Light::new(sys.light_colors[2], sys.light_directions[2]),
            ],
            sys,
            file_path: file.file_path.clone(),
            file: file
                .file_path
                .relative_to(state.opened_folder.clone().unwrap())
                .unwrap()
                .to_string(),
        }
    }

    pub fn show(&mut self, ui: &mut Ui, app: &mut AppState) {
        const SPACE_AROUND_HEADING: f32 = 10.0;
        let files: Vec<String> = app
            .files
            .iter()
            .filter_map(|file| {
                let name = file.name.clone();
                if matches!(
                    PathBuf::from(&name)
                        .extension()
                        .unwrap_or_default()
                        .to_str()
                        .unwrap_or(""),
                    "icn" | "ico"
                ) {
                    Some(name.clone())
                } else {
                    None
                }
            })
            .collect();

        menu::bar(ui, |ui| {
            ui.set_height(Self::TOOLBAR_HEIGHT);
            ui.add_space(Self::TOOLBAR_LEFT_MARGIN);
            ui.button("Save").clicked().then(|| self.save());
        });

        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.heading("Icon Configuration");
            ui.add_space(SPACE_AROUND_HEADING);
            Grid::new("title").num_columns(2).show(ui, |ui| {
                ui.label("Title first line");
                ui.add(TextEdit::singleline(&mut self.title_first_line));
                ui.end_row();
                ui.label("Title second line");
                ui.add(TextEdit::singleline(&mut self.title_second_line));

                length_warning(
                    ui,
                    self.title_first_line.len() + self.title_second_line.len(),
                    IconSys::MAXIMUM_TITLE_BYTE_LENGTH / 2,
                    "Title too long!",
                );
            });

            ui.add_space(SPACE_AROUND_HEADING);
            ui.separator();
            ui.heading("Icons");
            ui.add_space(SPACE_AROUND_HEADING);

            Grid::new("icons").num_columns(2).show(ui, |ui| {
                ui.label("List");
                file_select(ui, "list_icon", &mut self.icon_file, &files);
                length_warning(
                    ui,
                    self.icon_file.len(),
                    IconSys::MAXIMUM_FILENAME_BYTE_LENGTH / 2,
                    "Filename too long!",
                );
                ui.end_row();

                ui.label("Copy");
                file_select(ui, "copy_icon", &mut self.icon_copy_file, &files);
                length_warning(
                    ui,
                    self.icon_copy_file.len(),
                    IconSys::MAXIMUM_FILENAME_BYTE_LENGTH / 2,
                    "Filename too long!",
                );
                ui.end_row();

                ui.label("Delete");
                file_select(ui, "delete_icon", &mut self.icon_delete_file, &files);
                length_warning(
                    ui,
                    self.icon_delete_file.len(),
                    IconSys::MAXIMUM_FILENAME_BYTE_LENGTH / 2,
                    "Filename too long!",
                );
            });

            ui.add_space(SPACE_AROUND_HEADING);
            ui.separator();
            ui.heading("Background");
            ui.add_space(SPACE_AROUND_HEADING);

            ui.horizontal(|ui| {
                const GRADIENT_BOX_SPACING: f32 = 40.0;

                ui.add_sized(
                    vec2(GRADIENT_BOX_SPACING * 3.0, GRADIENT_BOX_SPACING * 3.0),
                    |ui: &mut Ui| {
                        draw_background(ui, &self.background_colors);
                        ui.spacing_mut().interact_size =
                            vec2(GRADIENT_BOX_SPACING, GRADIENT_BOX_SPACING);
                        ui.spacing_mut().item_spacing = vec2(0.0, 0.0);

                        ui.columns(3, |cols| {
                            egui::widgets::color_picker::color_edit_button_rgb(
                                &mut cols[0],
                                &mut self.background_colors[0].rgb,
                            );
                            cols[1].add_space(GRADIENT_BOX_SPACING);
                            egui::widgets::color_picker::color_edit_button_rgb(
                                &mut cols[2],
                                &mut self.background_colors[1].rgb,
                            );

                            cols[0].add_space(GRADIENT_BOX_SPACING);
                            cols[1].add_space(GRADIENT_BOX_SPACING);
                            cols[2].add_space(GRADIENT_BOX_SPACING);

                            egui::widgets::color_picker::color_edit_button_rgb(
                                &mut cols[0],
                                &mut self.background_colors[2].rgb,
                            );
                            cols[1].add_space(GRADIENT_BOX_SPACING);
                            egui::widgets::color_picker::color_edit_button_rgb(
                                &mut cols[2],
                                &mut self.background_colors[3].rgb,
                            );
                        });
                        ui.response()
                    },
                );

                Grid::new("background").num_columns(2).show(ui, |ui| {
                    ui.label("Background Transparency").on_hover_ui(|ui| {
                        ui.label(
                            "This is the opposite of opacity, so a value of 100 will make \
                                the background completely transparent",
                        );
                    });
                    ui.add(egui::Slider::new(
                        &mut self.background_transparency,
                        0..=100,
                    ));
                    ui.end_row();
                    ui.label("Ambient Color");
                    egui::widgets::color_picker::color_edit_button_rgb(
                        ui,
                        &mut self.ambient_color.rgb,
                    );
                    ui.end_row();
                });
            });

            ui.add_space(SPACE_AROUND_HEADING);
            ui.separator();
            ui.heading("Lights");
            ui.add_space(SPACE_AROUND_HEADING);
            Grid::new("lights")
                .num_columns(3)
                .spacing([50.0, 50.0])
                .min_col_width(40.0)
                .striped(true)
                .show(ui, |ui| {
                    for (index, light) in self.lights.iter_mut().enumerate() {
                        Grid::new(format!("light{index}"))
                            .num_columns(2)
                            .show(ui, |ui| {
                                ui.label(format!("Light {}", index + 1));
                                ui.end_row();
                                ui.label("Color");
                                egui::widgets::color_picker::color_edit_button_rgb(
                                    ui,
                                    &mut light.color.rgb,
                                );
                                ui.end_row();
                                ui.label("X");
                                ui.add(egui::Slider::new(&mut light.direction.x, 0.0..=1.0));
                                ui.end_row();
                                ui.label("Y");
                                ui.add(egui::Slider::new(&mut light.direction.y, 0.0..=1.0));
                                ui.end_row();
                                ui.label("Z");
                                ui.add(egui::Slider::new(&mut light.direction.z, 0.0..=1.0));
                            });
                    }
                });
        });
    }
}

impl Tab for IconSysViewer {
    fn get_id(&self) -> &str {
        &self.file
    }

    fn get_title(&self) -> String {
        self.file.clone()
    }

    fn get_modified(&self) -> bool {
        self.sys.title != format!("{}{}", self.title_first_line, self.title_second_line)
            || self.sys.icon_file != self.icon_file
            || self.sys.icon_copy_file != self.icon_copy_file
            || self.sys.icon_delete_file != self.icon_delete_file
    }

    fn save(&mut self) {
        let new_sys = IconSys {
            title: format!("{}{}", self.title_first_line, self.title_second_line),
            linebreak_pos: self.title_first_line.len() as u16,
            icon_file: self.icon_file.clone(),
            icon_copy_file: self.icon_copy_file.clone(),
            icon_delete_file: self.icon_delete_file.clone(),
            background_transparency: self.background_transparency,
            ambient_color: self.ambient_color.to_color_f(),
            background_colors: [
                self.background_colors[0].to_color(),
                self.background_colors[1].to_color(),
                self.background_colors[2].to_color(),
                self.background_colors[3].to_color(),
            ],
            light_colors: [
                self.lights[0].color.to_color_f(),
                self.lights[1].color.to_color_f(),
                self.lights[2].color.to_color_f(),
            ],
            light_directions: [
                self.lights[0].direction,
                self.lights[1].direction,
                self.lights[2].direction,
            ],
            ..self.sys.clone()
        };
        std::fs::write(&self.file_path, new_sys.to_bytes().unwrap()).expect("Failed to save icon");
        self.sys = new_sys;
    }
}

fn set_border_radius(ui: &mut Ui, radius: CornerRadius) {
    ui.style_mut().visuals.widgets.hovered.corner_radius = radius.add(CornerRadius::same(1));
    ui.style_mut().visuals.widgets.inactive.corner_radius = radius;
    ui.style_mut().visuals.widgets.active.corner_radius = radius;
}

fn file_select(ui: &mut Ui, name: impl Into<String>, value: &mut String, files: &[String]) {
    let id = Id::from(name.into());
    let layout_response = ui.horizontal(|ui| {
        ui.style_mut().spacing.item_spacing.x = 1.0;

        set_border_radius(
            ui,
            CornerRadius {
                nw: 2,
                sw: 2,
                ne: 0,
                se: 0,
            },
        );
        ui.text_edit_singleline(value);

        set_border_radius(
            ui,
            CornerRadius {
                nw: 0,
                sw: 0,
                ne: 2,
                se: 2,
            },
        );
        let response = ui.button("🔽");
        if response.clicked() {
            ui.memory_mut(|mem| {
                mem.toggle_popup(id);
            });
        }

        response
    });

    // Small hack to ensure the popup is positioned correctly
    let res = Response {
        rect: layout_response.response.rect,
        ..layout_response.inner
    };

    egui::popup_below_widget(ui, id, &res, PopupCloseBehavior::CloseOnClick, |ui| {
        ui.set_min_width(200.0);
        files.iter().for_each(|file| {
            if ui.selectable_label(false, file.clone()).clicked() {
                *value = file.clone();
            }
        });
    });
}

fn draw_background(ui: &mut Ui, colors: &[PS2RgbaInterface; 4]) {
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
        color: colors[0].into(),
    });
    mesh.vertices.push(egui::epaint::Vertex {
        pos: top_right,
        uv: egui::epaint::WHITE_UV,
        color: colors[1].into(),
    });
    mesh.vertices.push(egui::epaint::Vertex {
        pos: bottom_right,
        uv: egui::epaint::WHITE_UV,
        color: colors[3].into(),
    });
    mesh.vertices.push(egui::epaint::Vertex {
        pos: bottom_left,
        uv: egui::epaint::WHITE_UV,
        color: colors[2].into(),
    });

    mesh.indices
        .extend_from_slice(&[i0, i0 + 1, i0 + 2, i0, i0 + 2, i0 + 3]);

    painter.add(egui::Shape::mesh(mesh));
}

fn length_warning(ui: &mut Ui, length: usize, maximum_length: usize, message: &str) {
    if length > maximum_length {
        ui.end_row();
        ui.label("");
        ui.colored_label(Color32::RED, format!("{message} {length}/{maximum_length}"));
    }
}
