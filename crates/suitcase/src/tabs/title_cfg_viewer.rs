use crate::data::state::AppState;
use crate::tabs::Tab;
use crate::VirtualFile;
use eframe::egui::{menu, CornerRadius, Id, PopupCloseBehavior, Response, TextEdit, Ui};
use ps2_filetypes::TitleCfg;
use relative_path::PathExt;
use std::ops::Add;
use std::path::PathBuf;
use toml::Value;

const MAXIMUM_DESCRIPTION_LENGTH: usize = 250;

pub struct TitleCfgViewer {
    file: String,
    file_path: PathBuf,
    title_cfg: TitleCfg,
    modified: bool,
    encoding_error: bool,
    is_raw_editor: bool,
}

impl TitleCfgViewer {
    pub fn new(file: &VirtualFile, state: &AppState) -> Self {
        let buf = std::fs::read(&file.file_path).expect("Failed to read file");

        let contents = String::from_utf8(buf).ok();
        let encoding_error = contents.is_none();

        Self {
            file: file
                .file_path
                .relative_to(state.opened_folder.clone().unwrap())
                .unwrap()
                .to_string(),
            file_path: file.file_path.clone(),
            title_cfg: TitleCfg::new(contents.unwrap_or_default()),
            encoding_error,
            modified: false,
            is_raw_editor: false,
        }
    }

    pub fn show(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            menu::bar(ui, |ui| {
                ui.set_height(25.0);
                ui.button("Save").clicked().then(|| self.save());
                ui.button("Toggle Raw Editor").clicked().then(|| {
                    self.toggle_editors();
                });
            });
            ui.separator();

            if self.is_raw_editor {
                eframe::egui::Grid::new(Id::from("TitleCfgEditor"))
                    .num_columns(1)
                    .min_col_width(ui.available_width())
                    .show(ui, |ui| {
                        ui.add(
                            TextEdit::multiline(&mut self.title_cfg.contents)
                                .desired_width(ui.available_width()),
                        )
                            .changed()
                            .then(|| self.modified = true);
                    });
            } else {
                eframe::egui::Grid::new(Id::from("TitleCfgEditor"))
                    .num_columns(3)
                    .min_col_width(200.0)
                    .max_col_width(ui.available_width())
                    .show(ui, |ui| {
                        if self.encoding_error {
                            ui.colored_label(
                                eframe::egui::Color32::RED,
                                "Encoding error, please use valid ASCII or UTF-8 encoding.",
                            );
                            return;
                        }

                        if !self.title_cfg.has_mandatory_fields() {
                            ui.colored_label(
                                eframe::egui::Color32::RED,
                                "Missing mandatory fields.",
                            );
                            ui.button("Fix").clicked().then(|| {
                                self.title_cfg.add_missing_fields();
                                self.modified = true;
                            });
                            ui.end_row();
                        }

                        for (key, value) in self.title_cfg.index_map.iter_mut() {
                            let key_helper = self.title_cfg.helper.get(key);

                            let mut tooltip_content = "".to_string();
                            if key_helper.is_some_and(|key| key.get("tooltip").is_some()) {
                                tooltip_content =
                                    key_helper.unwrap().get("tooltip").unwrap().to_string();
                            }

                            let key_label = ui.label(key.to_string());
                            if !tooltip_content.is_empty() {
                                key_label.on_hover_ui(|ui| {
                                    ui.label(tooltip_content);
                                });
                            }

                            if key == "Description" {
                                ui.add(TextEdit::multiline(value).desired_rows(6))
                                    .changed()
                                    .then(|| self.modified = true);
                                if value.len() > MAXIMUM_DESCRIPTION_LENGTH {
                                    ui.colored_label(
                                        eframe::egui::Color32::RED,
                                        format!(
                                            "Description too long, it will be truncated in OPL. {}/{}",
                                            value.len(),
                                            MAXIMUM_DESCRIPTION_LENGTH,
                                        ),
                                    );
                                }
                            } else if key_helper.is_some_and(|key| key.get("values").is_some()) {
                                value_select(
                                    ui,
                                    key,
                                    value,
                                    key_helper.unwrap().get("values").unwrap(),
                                )
                                    .changed()
                                    .then(|| self.modified = true);
                            } else {
                                ui.text_edit_singleline(value)
                                    .changed()
                                    .then(|| self.modified = true);
                            }

                            ui.end_row();
                        }
                    });
            }
        });
    }

    pub fn toggle_editors(&mut self) {
        if self.is_raw_editor {
            self.title_cfg.sync_contents_to_index_map();
        } else {
            self.title_cfg.sync_index_map_to_contents()
        }
        self.is_raw_editor ^= true;
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
        if self.is_raw_editor {
            self.title_cfg.sync_contents_to_index_map();
        }
        std::fs::write(&self.file_path, self.title_cfg.to_string().into_bytes())
            .expect("Failed to title.cfg");

        self.modified = false;
    }
}

fn set_border_radius(ui: &mut Ui, radius: CornerRadius) {
    ui.style_mut().visuals.widgets.hovered.corner_radius = radius.add(CornerRadius::same(1));
    ui.style_mut().visuals.widgets.inactive.corner_radius = radius;
    ui.style_mut().visuals.widgets.active.corner_radius = radius;
}

fn value_select(
    ui: &mut Ui,
    name: impl Into<String>,
    selected_value: &mut String,
    values: &Value,
) -> Response {
    let id = Id::from(name.into());
    let mut layout_response = ui.horizontal(|ui| {
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
        let edit_response = ui.text_edit_singleline(selected_value);

        set_border_radius(
            ui,
            CornerRadius {
                nw: 0,
                sw: 0,
                ne: 2,
                se: 2,
            },
        );
        let button_response = ui.button("🔽");
        button_response.clicked().then(|| {
            ui.memory_mut(|mem| {
                mem.toggle_popup(id);
            });
        });

        (edit_response, button_response)
    });

    // Small hack to ensure the popup is positioned correctly
    let res = Response {
        rect: layout_response.response.rect,
        ..layout_response.inner.1
    };

    let values = parse_values(values).unwrap_or_default();

    eframe::egui::popup_below_widget(ui, id, &res, PopupCloseBehavior::CloseOnClick, |ui| {
        ui.set_min_width(200.0);
        for value in values {
            ui.selectable_label(false, &value).clicked().then(|| {
                *selected_value = value;
                layout_response.inner.0.mark_changed();
            });
        }
    });

    layout_response.inner.0
}

fn parse_values(value: &Value) -> Option<Vec<String>> {
    Some(
        value
            .as_array()?
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_owned()))
            .collect(),
    )
}
