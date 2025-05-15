use crate::ui::CustomButtons;
use crate::{AppState, VirtualFile};
use bytesize::ByteSize;
use eframe::egui;
use eframe::egui::{include_image, vec2, Align, Button, Color32, Image, ImageSource, Layout, Stroke, TextWrapMode, Ui, Vec2, WidgetText};
use std::fs::read_dir;
use std::ops::Sub;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

fn calc_size(size: u64) -> u64 {
    ((size + 1023) as i64 & -1024) as u64
}

#[derive(Default)]
pub struct FileTree {
    pub state: Arc<Mutex<AppState>>,
}

impl FileTree {
    pub fn open(&mut self, folder: PathBuf) {
        let files = read_dir(folder).expect("Could not read directory");
        let mut files = files
            .into_iter()
            .flatten()
            .filter_map(|entry| {
                if entry.file_type().ok()?.is_file() {
                    Some(Arc::new(Mutex::new(VirtualFile {
                        name: entry.file_name().into_string().unwrap(),
                        file_path: entry.path(),
                    })))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        files.sort_by(|a, b| {
            a.lock().unwrap().name.to_lowercase().partial_cmp(&b.lock().unwrap().name.to_lowercase()).unwrap()
        });

        let mut calculated_size = 512 * 3; // First 3 entries

        for file in files.iter() {
            let path = file.lock().unwrap().file_path.clone();
            if let Ok(metadata) = std::fs::metadata(&path) {
                // println!("{}", metadata.len());
                calculated_size += 512 + calc_size(metadata.len());
            } else {
                eprintln!("Could not get metadata for {:?}", path);
            }
        }

        self.state.lock().unwrap().files = files;
        self.state.lock().unwrap().calculated_size = calculated_size;
    }
    
    pub fn icon(&self, file_name: String) -> ImageSource {
        match file_name.to_lowercase().split('.').next_back() {
            None => include_image!("../../assets/icons/file.svg"),
            Some("elf") => include_image!("../../assets/icons/file-digit.svg"),
            Some("icn") => include_image!("../../assets/icons/file-3d.svg"),
            Some("sys") => include_image!("../../assets/icons/file-settings.svg"),
            Some("cfg") => include_image!("../../assets/icons/file-code.svg"),
            Some(_) => include_image!("../../assets/icons/file.svg"),
        }
    }
}

impl egui::Widget for &mut FileTree {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        ui.scope(|ui| {
            let mut state = self.state.lock().unwrap();
            ui.add_space(10.0);
            let size = ui.available_size();
            let scroll_size = size.sub(Vec2::new(0.0, 54.0));
            ui.allocate_ui(scroll_size, |ui| {
                ui.set_min_height(scroll_size.y);
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.with_layout(Layout::top_down_justified(Align::Min), |ui| {
                        let style = ui.style_mut();
                        style.spacing.button_padding = vec2(2.0, 0.0);
                        style.visuals.widgets.active.bg_stroke = Stroke::NONE;
                        style.visuals.widgets.hovered.bg_stroke = Stroke::NONE;
                        style.visuals.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
                        style.visuals.widgets.inactive.bg_stroke = Stroke::NONE;
                        style.wrap_mode = Some(TextWrapMode::Extend);

                        for file in state.files.clone().into_iter() {
                            let name = file.lock().unwrap().name.clone();
                            
                            let item = ui.icon_text_button(self.icon(name.clone()), name);

                            if item.clicked() {
                                state.open_file(file);
                            }
                        }
                    });
                });
            });
            ui.with_layout(
                Layout::default()
                    .with_cross_justify(true)
                    .with_main_align(Align::Center),
                |ui| {
                    ui.scope(|ui| {
                        ui.visuals_mut().widgets.inactive.weak_bg_fill = Color32::DARK_GREEN;
                        ui.visuals_mut().widgets.hovered.weak_bg_fill =
                            Color32::from_rgb(0, 0x6C, 0);
                        ui.visuals_mut().widgets.active.weak_bg_fill =
                            Color32::from_rgb(0, 0x68, 0);
                        ui.spacing_mut().button_padding = Vec2::splat(10.0);

                        let button = ui.add(Button::new(WidgetText::from(format!("Export PSU ({})", ByteSize::b(state.calculated_size))).heading()));
                        if button.clicked() {

                        }
                    });
                },
            );
        })
        .response
    }
}

pub trait FileTreeComponent {
    fn file_tree(&mut self, state: Arc<Mutex<AppState>>) -> egui::Response;
}

impl FileTreeComponent for Ui {
    fn file_tree(&mut self, state: Arc<Mutex<AppState>>) -> egui::Response {
        self.add(&mut FileTree { state })
    }
}
