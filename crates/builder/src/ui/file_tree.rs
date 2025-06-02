use crate::{AppState, VirtualFile};
use bytesize::ByteSize;
use eframe::egui;
use eframe::egui::{include_image, Checkbox, Id, ImageSource, Label, Layout, Ui};
use egui_extras::{Column, TableBuilder};
use std::fs::read_dir;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::UNIX_EPOCH;
use ps2_filetypes::chrono::{DateTime, Local};

fn calc_size(size: u64) -> u64 {
    ((size + 1023) as i64 & -1024) as u64
}

pub struct FileTree {
    pub state: Arc<Mutex<AppState>>,
    selection: std::collections::HashSet<usize>,
    show_timestamp: bool,
    show_attributes: bool,
    id: Id
}


impl FileTree {
    pub fn new(state: Arc<Mutex<AppState>>) -> Self {
        Self {
            state,
            selection: std::collections::HashSet::new(),
            show_timestamp: false,
            show_attributes: false,
            id: Id::new("file_tree"),
        }
    }
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
                        size: entry.path().metadata().unwrap().len(),
                    })))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        files.sort_by(|a, b| {
            a.lock()
                .unwrap()
                .name
                .to_lowercase()
                .partial_cmp(&b.lock().unwrap().name.to_lowercase())
                .unwrap()
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

    pub fn icon(file_name: &str) -> ImageSource {
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
            let len = state.files.len();
            let mut table = TableBuilder::new(ui)
                .id_salt(self.id.clone())
                .striped(true)
                // .resizable(true)
                .cell_layout(Layout::left_to_right(egui::Align::Center))
                .column(Column::auto().resizable(false))
                .column(Column::auto().resizable(true))
                .column(Column::auto())
                .column(Column::remainder());

            table = table.sense(egui::Sense::click());

            table
                .header(20.0, |mut header| {
                    header.col(|_ui| {});
                    header.col(|ui| {
                        ui.add(Label::new("File").selectable(false));
                    });
                    if self.show_timestamp {
                        header.col(|ui| {
                            ui.add(Label::new("Timestamp").selectable(false));
                        });
                    }
                    header.col(|ui| {
                        ui.add(Label::new("Size").selectable(false));
                    });
                    let response = header.response();

                    response.context_menu(|ui| {
                        let mut readonly = true;
                        ui.add_enabled(false, Checkbox::new(&mut readonly, "File"));
                        ui.checkbox(&mut self.show_timestamp, "Timestamp");
                        ui.checkbox(&mut self.show_attributes, "Attributes");
                        ui.add_enabled(false, Checkbox::new(&mut readonly, "Size"));
                    });
                })
                .body(|body| {
                    body.rows(20.0, len, |mut row| {
                        let row_index = row.index();
                        let file_ref = state.files[row_index].clone();
                        let file = file_ref.lock().unwrap();
                        let name = &file.name;
                        let file_path = &file.file_path;
                        let size = file.size;

                        row.set_selected(self.selection.contains(&row_index));

                        row.col(|ui| {
                            ui.image(FileTree::icon(&name));
                        });
                        row.col(|ui| {
                            ui.add(Label::new(name).selectable(false));
                        });

                        if self.show_timestamp {
                            row.col(|ui| {
                                if let Ok(metadata) = file_path.metadata() {
                                    if let Ok(modified) = metadata.modified() {
                                        let dt_modified: DateTime<Local> = modified.into();
                                        ui.label(dt_modified.format("%Y-%m-%d %H:%M:%S").to_string());
                                    }
                                }
                            });
                        }

                        row.col(|ui| {
                            let size = ByteSize::b(calc_size(size));
                            ui.label(format!("{}", size));
                        });

                        if row.response().clicked() {
                            if self.selection.contains(&row_index) {
                                self.selection.remove(&row_index);
                            } else {
                                self.selection.clear();
                                self.selection.insert(row_index);
                            }
                        }
                        if row.response().double_clicked() {
                            state.open_file(file_ref.clone());
                        }
                    })
                });
        })
            .response
    }
}