use crate::data::virtual_file::VirtualFile;
use crate::AppState;
use eframe::egui::collapsing_header::CollapsingState;
use eframe::egui::{
    include_image, vec2, Align, Button, Color32, Id, ImageSource, Layout, ScrollArea, Stroke,
    Style, TextWrapMode, Ui,
};
use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

fn is_symlink_or_junction(file_type: &std::fs::FileType) -> bool {
    if file_type.is_symlink() {
        true
    } else {
        #[cfg(windows)]
        {
            use std::os::windows::fs::FileTypeExt;
            file_type.is_symlink_dir() || file_type.is_symlink_file()
        }
        #[cfg(not(windows))]
        {
            false
        }
    }
}

pub struct FileTree {
    show_timestamp: bool,
    show_attributes: bool,
    id: Id,
    expanded: HashMap<PathBuf, bool>,
    dir_cache: HashMap<PathBuf, Vec<PathBuf>>,
    visited_dirs: HashSet<PathBuf>,
}

fn set_menu_style(style: &mut Style) {
    style.spacing.button_padding = vec2(2.0, 0.0);
    style.visuals.widgets.active.bg_stroke = Stroke::NONE;
    style.visuals.widgets.hovered.bg_stroke = Stroke::NONE;
    style.visuals.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
    style.visuals.widgets.inactive.bg_stroke = Stroke::NONE;
}

impl FileTree {
    pub fn new() -> Self {
        Self {
            show_timestamp: false,
            show_attributes: false,
            id: Id::new("file_tree"),
            expanded: HashMap::new(),
            dir_cache: HashMap::new(),
            visited_dirs: HashSet::new(),
        }
    }

    pub fn icon(file_name: &str) -> ImageSource {
        match file_name.to_lowercase().split('.').next_back() {
            None => include_image!("../../assets/hidpi/fm_file.png"),
            Some("elf") => include_image!("../../assets/hidpi/fm_elf.png"),
            Some("icn") => include_image!("../../assets/hidpi/fm_icon.png"),
            Some("sys") => include_image!("../../assets/hidpi/fm_cfg_icon.png"),
            Some("cfg") => include_image!("../../assets/hidpi/fm_cfg_other.png"),
            Some(_) => include_image!("../../assets/hidpi/fm_file.png"),
        }
    }

    pub fn show(&mut self, ui: &mut Ui, state: &mut AppState) {
        set_menu_style(ui.style_mut());
        if let Some(folder) = state.opened_folder.as_ref() {
            ScrollArea::new([true, true]).show(ui, |ui| {
                ui.with_layout(
                    Layout::top_down(Align::Min).with_cross_justify(true),
                    |ui| {
                        self.show_folder(ui, folder.clone(), state);
                    },
                );
            });
        }
    }

    pub fn show_folder(&mut self, ui: &mut Ui, path: PathBuf, state: &mut AppState) {
        let file_name = Self::display_name(&path);

        let (_, response, _) =
            CollapsingState::load_with_default_open(ui.ctx(), Id::new(&path), false)
                .show_header(ui, |ui| {
                    ui.with_layout(
                        Layout::top_down(Align::Min).with_cross_justify(true),
                        |ui| {
                            ui.add(
                                // Button::image_and_text(
                                //     include_image!("../../assets/hidpi/fm_file.png"),
                                //     file_name,
                                // )
                                Button::new(file_name).wrap_mode(TextWrapMode::Extend),
                            )
                        },
                    )
                    .inner
                })
                .body(|ui| {
                    let children = self.dir_cache.get(&path).cloned().unwrap_or(vec![]);

                    for child in children {
                        if self.dir_cache.contains_key(&child) {
                            self.show_folder(ui, child.clone(), state);
                        } else {
                            self.show_file(ui, child.clone(), state);
                        }
                    }
                });

        response.inner.context_menu(|ui| {
            ui.set_min_width(100.0);
            ui.button("Export");
            ui.separator();
            ui.button("Delete");
        });
    }

    pub fn show_file(&mut self, ui: &mut Ui, path: PathBuf, state: &mut AppState) {
        let file_name = Self::display_name(&path);

        let response = ui.add(
            Button::image_and_text(Self::icon(&file_name), file_name.clone())
                .wrap_mode(TextWrapMode::Extend),
        );

        if response.double_clicked() {
            state.open_file(VirtualFile {
                name: file_name.clone(),
                size: 0,
                file_path: path.clone(),
            });
        }
    }

    fn display_name(path: &Path) -> String {
        let file_name = path.file_name();

        if let Some(valid) = file_name.and_then(OsStr::to_str) {
            valid.to_owned()
        } else if let Some(name) = file_name {
            name.to_string_lossy().into_owned()
        } else {
            path.display().to_string()
        }
    }

    fn index_folder_internal(&mut self, root: &Path) -> std::io::Result<()> {
        let canonical_root = match root.canonicalize() {
            Ok(path) => path,
            Err(err) => {
                eprintln!(
                    "Failed to canonicalize directory '{}': {err}",
                    root.display()
                );
                return Err(err);
            }
        };

        if !self.visited_dirs.insert(canonical_root.clone()) {
            return Ok(());
        }

        let mut folders = Vec::new();
        let mut files = Vec::new();
        let children = match std::fs::read_dir(root) {
            Ok(children) => children,
            Err(err) => {
                eprintln!("Failed to read contents of '{}': {err}", root.display());
                self.visited_dirs.remove(&canonical_root);
                return Err(err);
            }
        };

        for entry in children {
            match entry {
                Ok(entry) => {
                    let path = entry.path();
                    let file_type = match entry.file_type() {
                        Ok(file_type) => file_type,
                        Err(err) => {
                            eprintln!(
                                "Failed to read metadata for '{}': {err}",
                                path.display()
                            );
                            continue;
                        }
                    };

                    if is_symlink_or_junction(&file_type) {
                        continue;
                    }

                    if file_type.is_dir() {
                        match self.index_folder_internal(&path) {
                            Ok(()) => {
                                if self.dir_cache.contains_key(&path) {
                                    folders.push(path);
                                }
                            }
                            Err(err) => {
                                eprintln!(
                                    "Skipping subdirectory '{}' due to error: {err}",
                                    path.display()
                                );
                            }
                        }
                    } else {
                        files.push(path);
                    }
                }
                Err(err) => {
                    eprintln!(
                        "Failed to access directory entry in '{}': {err}",
                        root.display()
                    );
                }
            }
        }

        self.dir_cache
            .insert(root.to_path_buf(), [folders, files].concat());

        Ok(())
    }

    pub fn index_folder(&mut self, root: &PathBuf) -> std::io::Result<()> {
        self.dir_cache.clear();
        self.visited_dirs.clear();
        match self.index_folder_internal(root.as_path()) {
            Ok(()) => Ok(()),
            Err(err) => {
                self.dir_cache.clear();
                self.visited_dirs.clear();
                Err(err)
            }
        }
    }

    // pub fn show(&mut self, ui: &mut Ui, app: &mut AppState) {
    //     let height = ui.available_height();
    //     ui.scope(|ui| {
    //         let len = app.files.len();
    //
    //         let mut table = TableBuilder::new(ui)
    //             .id_salt(self.id.clone())
    //             .striped(true)
    //             // .resizable(true)
    //             .cell_layout(Layout::left_to_right(egui::Align::Center))
    //             .column(Column::auto().resizable(false))
    //             .column(Column::auto().resizable(true))
    //             .column(Column::auto())
    //             .column(Column::remainder())
    //             .min_scrolled_height(0.0)
    //             .max_scroll_height(height);
    //
    //         table = table.sense(egui::Sense::click());
    //
    //         table
    //             .header(20.0, |mut header| {
    //                 header.col(|_ui| {});
    //                 header.col(|ui| {
    //                     ui.add(Label::new("File").selectable(false));
    //                 });
    //                 if self.show_timestamp {
    //                     header.col(|ui| {
    //                         ui.add(Label::new("Timestamp").selectable(false));
    //                     });
    //                 }
    //                 header.col(|ui| {
    //                     ui.add(Label::new("Size").selectable(false));
    //                 });
    //                 let response = header.response();
    //
    //                 response.context_menu(|ui| {
    //                     let mut readonly = true;
    //                     ui.add_enabled(false, Checkbox::new(&mut readonly, "File"));
    //                     ui.checkbox(&mut self.show_timestamp, "Timestamp");
    //                     ui.checkbox(&mut self.show_attributes, "Attributes");
    //                     ui.add_enabled(false, Checkbox::new(&mut readonly, "Size"));
    //                 });
    //             })
    //             .body(|body| {
    //                 body.rows(32.0, len, |mut row| {
    //                     let row_index = row.index();
    //                     let file = app.files[row_index].clone();
    //                     let name = &file.name;
    //                     let file_path = &file.file_path;
    //                     let size = file.size;
    //
    //                     row.set_selected(self.selection.contains(&row_index));
    //
    //                     row.col(|ui| {
    //                         ui.add(Image::new(FileTree::icon(name)).fit_to_original_size(1.0));
    //                     });
    //                     row.col(|ui| {
    //                         ui.add(Label::new(name).selectable(false));
    //                     });
    //
    //                     if self.show_timestamp {
    //                         row.col(|ui| {
    //                             if let Ok(metadata) = file_path.metadata() {
    //                                 if let Ok(modified) = metadata.modified() {
    //                                     let dt_modified: DateTime<Local> = modified.into();
    //                                     ui.label(
    //                                         dt_modified.format("%Y-%m-%d %H:%M:%S").to_string(),
    //                                     );
    //                                 }
    //                             }
    //                         });
    //                     }
    //
    //                     row.col(|ui| {
    //                         let size = ByteSize::b(calc_size(size));
    //                         ui.label(format!("{}", size));
    //                     });
    //
    //                     if row.response().clicked() {
    //                         if self.selection.contains(&row_index) {
    //                             self.selection.remove(&row_index);
    //                         } else {
    //                             self.selection.clear();
    //                             self.selection.insert(row_index);
    //                         }
    //                     }
    //                     if row.response().double_clicked() {
    //                         app.open_file(file.clone());
    //                     }
    //                     row.response().context_menu(|ui| {
    //                         if ui.button("Open").clicked() {
    //                             app.open_file(file.clone());
    //                             ui.close_menu();
    //                         }
    //                         if ui.button("Show in File Explorer").clicked() {
    //                             if let Some(path) = file_path.to_str() {
    //                                 reveal_file_in_explorer(Path::new(path));
    //                                 ui.close_menu();
    //                             }
    //                         }
    //                         if !app.pcsx2_path.is_empty() && file_path.extension().map_or(false, |ext| ext.to_ascii_lowercase() == "elf") {
    //                             if ui.button("Run in PCSX2").clicked() {
    //                                 app.start_pcsx2_elf(file.file_path.clone());
    //                                 ui.close_menu();
    //                             }
    //                         }
    //                         ui.add_enabled_ui(false, |ui| {
    //                             _ = ui.button("Delete");
    //                         });
    //                     });
    //                 })
    //             });
    //     });
    // }
}
