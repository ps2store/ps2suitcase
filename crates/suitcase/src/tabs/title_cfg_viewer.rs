use crate::tabs::Tab;
use crate::VirtualFile;
use eframe::egui::{CornerRadius, Id, PopupCloseBehavior, Response, TextEdit, Ui};
use ps2_filetypes::TitleCfg;
use std::fs::File;
use std::io::Write;
use std::ops::Add;
use std::path::PathBuf;
use toml::Value;
use relative_path::PathExt;
use crate::data::state::AppState;

pub struct TitleCfgViewer {
    file: String,
    file_path: PathBuf,
    contents: String,
    title_cfg: TitleCfg,
    modified: bool,
    encoding_error: bool,
}

impl TitleCfgViewer {
    pub fn new(file: &VirtualFile, state: &AppState) -> Self {
        let buf = std::fs::read(&file.file_path)
            .expect("Failed to read file");

        let contents = String::from_utf8(buf.clone()).ok();
        let encoding_error = contents.is_none();

        Self {
            file: file
                .file_path
                .relative_to(state.opened_folder.clone().unwrap())
                .unwrap()
                .to_string(),
            file_path: file.file_path.clone(),
            contents: contents.clone().unwrap_or_default(),
            title_cfg: TitleCfg::new(contents.unwrap_or_default()),
            encoding_error,
            modified: false,
        }
    }

    pub fn show(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            eframe::egui::Grid::new(Id::from("IconSysEditor"))
                .num_columns(2)
                .show(ui, |ui| {
                    if self.encoding_error {
                        ui.colored_label(
                            eframe::egui::Color32::RED,
                            "Encoding error, please use valid ASCII or UTF-8 encoding.",
                        );
                        return;
                    }

                    if !self.title_cfg.has_mandatory_fields() {
                        ui.colored_label(eframe::egui::Color32::RED, "Missing mandatory fields.");
                        if ui.button("Fix").clicked() {
                            self.title_cfg.fix_missing_fields();
                        }
                        ui.end_row();
                    }

                    for (key, value) in self.title_cfg.index_map.iter_mut() {
                        let key_helper = self.title_cfg
                            .helper
                            .get(key);

                        let mut tooltip_content = format!("");
                        if key_helper.is_some_and(|key| key.get("tooltip").is_some()) {
                            tooltip_content = key_helper.unwrap()
                                .get("tooltip")
                                .unwrap()
                                .to_string();
                        }

                        let key_label = ui.label(format!("{key}"));
                        if !tooltip_content.is_empty() {
                            key_label.on_hover_ui(|ui| {
                                ui.label(tooltip_content);
                            });
                        }

                        if value == "Description" {
                            ui.add(TextEdit::singleline(value).desired_rows(4));
                        } else if key_helper.is_some_and(|key| key.get("values").is_some()) {
                            value_select(
                                ui,
                                format!("{key}"),
                                value,
                                key_helper.unwrap()
                                    .get("values")
                                    .unwrap(),
                            );
                        } else {
                            ui.add(TextEdit::singleline(value));
                        }

                        ui.end_row();
                    }

                    if ui.button("Save").clicked() {
                        self.save();
                    }
                });
        });
    }
}

impl Tab for TitleCfgViewer {
    fn get_id(&self) -> &str {
        &self.file
    }
    fn get_title(&self) -> String {
        self.file.to_string()
    }

    fn get_modified(&self) -> bool {
        self.modified
    }

    fn save(&mut self) {
        let mut output = File::create(&self.file_path).expect("File not found");
        output.write_all(self.contents.as_bytes()).unwrap();
        std::fs::write(&self.file_path, self.title_cfg.to_bytes()).expect("Failed to title.cfg");
        self.modified = false;
    }
}

fn set_border_radius(ui: &mut Ui, radius: CornerRadius) {
    ui.style_mut().visuals.widgets.hovered.corner_radius = radius.add(CornerRadius::same(1));
    ui.style_mut().visuals.widgets.inactive.corner_radius = radius;
    ui.style_mut().visuals.widgets.active.corner_radius = radius;
}

fn value_select(ui: &mut Ui, name: impl Into<String>, selected_value: &mut String, values: &Value) {
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
        ui.text_edit_singleline(selected_value);

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

    let values = parse_values(values).unwrap_or_default();

    eframe::egui::popup_below_widget(ui, id, &res, PopupCloseBehavior::CloseOnClick, |ui| {
        ui.set_min_width(200.0);
        for value in values {
            if ui.selectable_label(false, &value).clicked() {
                *selected_value = value;
            }
        }
    });
}

fn parse_values(value: &Value) -> Option<Vec<String>> {
    Some(
        value
            .as_array()?
            .iter()
            .map(|v| v.as_str().and_then(|s| Some(s.to_owned())))
            .flatten()
            .collect(),
    )
}
