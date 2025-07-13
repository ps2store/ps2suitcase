use crate::io::calculate_size::calc_size;
use crate::io::reveal_file_in_explorer::reveal_file_in_explorer;
use crate::AppState;
use bytesize::ByteSize;
use eframe::egui;
use eframe::egui::{
    include_image, Checkbox, Id, Image, ImageSource, Label, Layout, Ui,
};
use egui_extras::{Column, TableBuilder};
use ps2_filetypes::chrono::{DateTime, Local};
use std::path::Path;

pub struct FileTree {
    selection: std::collections::HashSet<usize>,
    show_timestamp: bool,
    show_attributes: bool,
    id: Id,
}

impl FileTree {
    pub fn new() -> Self {
        Self {
            selection: std::collections::HashSet::new(),
            show_timestamp: false,
            show_attributes: false,
            id: Id::new("file_tree"),
        }
    }

    pub fn icon(file_name: &str) -> ImageSource {
        match file_name.to_lowercase().split('.').next_back() {
            None => include_image!("../../assets/lowdpi/fm_file.png"),
            Some("elf") => include_image!("../../assets/lowdpi/fm_elf.png"),
            Some("icn") => include_image!("../../assets/lowdpi/fm_icon.png"),
            Some("sys") => include_image!("../../assets/lowdpi/fm_cfg_icon.png"),
            Some("cfg") => include_image!("../../assets/lowdpi/fm_cfg_other.png"),
            Some(_) => include_image!("../../assets/lowdpi/fm_file.png"),
        }
    }

    pub fn show(&mut self, ui: &mut Ui, app: &mut AppState) {
        let height = ui.available_height();
        ui.scope(|ui| {
            let len = app.files.len();

            let mut table = TableBuilder::new(ui)
                .id_salt(self.id.clone())
                .striped(true)
                // .resizable(true)
                .cell_layout(Layout::left_to_right(egui::Align::Center))
                .column(Column::auto().resizable(false))
                .column(Column::auto().resizable(true))
                .column(Column::auto())
                .column(Column::remainder())
                .min_scrolled_height(0.0)
                .max_scroll_height(height);

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
                        let file = app.files[row_index].clone();
                        let name = &file.name;
                        let file_path = &file.file_path;
                        let size = file.size;

                        row.set_selected(self.selection.contains(&row_index));

                        row.col(|ui| {
                            ui.add(Image::new(FileTree::icon(name)).fit_to_original_size(1.0));
                        });
                        row.col(|ui| {
                            ui.add(Label::new(name).selectable(false));
                        });

                        if self.show_timestamp {
                            row.col(|ui| {
                                if let Ok(metadata) = file_path.metadata() {
                                    if let Ok(modified) = metadata.modified() {
                                        let dt_modified: DateTime<Local> = modified.into();
                                        ui.label(
                                            dt_modified.format("%Y-%m-%d %H:%M:%S").to_string(),
                                        );
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
                            app.open_file(file.clone());
                        }
                        row.response().context_menu(|ui| {
                            if ui.button("Open").clicked() {
                                app.open_file(file.clone());
                                ui.close_menu();
                            }
                            if ui.button("Show in File Explorer").clicked() {
                                if let Some(path) = file_path.to_str() {
                                    reveal_file_in_explorer(Path::new(path));
                                    ui.close_menu();
                                }
                            }
                            if !app.pcsx2_path.is_empty() && file_path.extension().map_or(false, |ext| ext.to_ascii_lowercase() == "elf") {
                                if ui.button("Run in PCSX2").clicked() {
                                    app.start_pcsx2_elf(file.file_path.clone());
                                    ui.close_menu();
                                }
                            }
                            ui.add_enabled_ui(false, |ui| {
                                _ = ui.button("Delete");
                            });
                        });
                    })
                });
        });
    }
}
