use crate::tabs::Tab;
use crate::{AppState, VirtualFile};
use eframe::egui;
use eframe::egui::{CornerRadius, Id, PopupCloseBehavior, Response, TextEdit, Ui};
use ps2_filetypes::IconSys;
use std::ops::Add;
use std::path::PathBuf;

pub struct IconSysViewer {
    title: String,
    file: String,
    pub icon_file: String,
    pub icon_copy_file: String,
    pub icon_delete_file: String,
    pub sys: IconSys,
    pub file_path: PathBuf,
}

impl IconSysViewer {
    pub fn new(file: &VirtualFile) -> Self {
        let buf = std::fs::read(&file.file_path).expect("File not found");

        let sys = IconSys::new(buf);

        Self {
            title: sys.title.clone(),
            icon_file: sys.icon_file.clone(),
            icon_copy_file: sys.icon_copy_file.clone(),
            icon_delete_file: sys.icon_delete_file.clone(),
            sys,
            file_path: file.file_path.clone(),
            file: file
                .file_path
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .to_string(),
        }
    }

    pub fn show(&mut self, ui: &mut Ui, app: &mut AppState) {
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

        ui.vertical(|ui| {
            eframe::egui::Grid::new(Id::from("IconSysEditor"))
                .num_columns(2)
                .show(ui, |ui| {
                    ui.label("Title");
                    ui.add(TextEdit::singleline(&mut self.title).desired_width(f32::INFINITY));
                    ui.end_row();
                    ui.label("Icon");
                    file_select(ui, "list_icon", &mut self.icon_file, &files);
                    ui.end_row();
                    ui.label("Copy Icon");
                    file_select(ui, "copy_icon", &mut self.icon_copy_file, &files);
                    ui.end_row();
                    ui.label("Delete Icon");
                    file_select(ui, "delete_icon", &mut self.icon_delete_file, &files);
                    ui.end_row();
                });
            ui.button("Save")
                .on_hover_text("Save changes")
                .clicked()
                .then(|| {
                    self.save();
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
        self.sys.title != self.title
            || self.sys.icon_file != self.icon_file
            || self.sys.icon_copy_file != self.icon_copy_file
            || self.sys.icon_delete_file != self.icon_delete_file
    }

    fn save(&mut self) {
        let new_sys = IconSys {
            title: self.title.clone(),
            icon_file: self.icon_file.clone(),
            icon_copy_file: self.icon_copy_file.clone(),
            icon_delete_file: self.icon_delete_file.clone(),
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
    let layout_repsonse = ui
        .horizontal(|ui| {
            ui.style_mut().spacing.item_spacing.x = 1.0;

            set_border_radius(ui, CornerRadius{nw: 2, sw: 2, ne: 0, se: 0});
            ui.text_edit_singleline(value);

            set_border_radius(ui, CornerRadius{nw: 0, sw: 0, ne: 2, se: 2});
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
        rect: layout_repsonse.response.rect,
        ..layout_repsonse.inner
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
