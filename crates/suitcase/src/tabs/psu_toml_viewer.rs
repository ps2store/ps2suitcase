use crate::data::state::AppState;
use crate::tabs::Tab;
use crate::VirtualFile;
use eframe::egui::{self, menu, Color32, DragValue, ScrollArea, TextEdit, Ui};
use relative_path::PathExt;
use std::path::PathBuf;
use toml::value::Table;
use toml::Value;

pub struct PsuTomlViewer {
    file: String,
    file_path: PathBuf,
    contents: String,
    structured: Option<Value>,
    parse_error: Option<String>,
    show_raw: bool,
    modified: bool,
}

impl PsuTomlViewer {
    pub fn new(file: &VirtualFile, state: &AppState) -> Self {
        let contents = std::fs::read_to_string(&file.file_path).unwrap_or_default();
        let file_path = file.file_path.clone();
        let file = file
            .file_path
            .relative_to(state.opened_folder.clone().unwrap())
            .unwrap()
            .to_string();

        Self {
            file,
            file_path,
            contents,
            structured: None,
            parse_error: None,
            show_raw: true,
            modified: false,
        }
    }

    pub fn show(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            menu::bar(ui, |ui| {
                ui.set_height(25.0);
                if ui.button("Save").clicked() {
                    self.save();
                }
                let toggle_label = if self.show_raw {
                    "Structured View"
                } else {
                    "Raw View"
                };
                if ui.button(toggle_label).clicked() {
                    self.toggle_view();
                }
            });

            if let Some(error) = &self.parse_error {
                ui.colored_label(Color32::RED, format!("Failed to parse TOML: {error}"));
            }

            ui.separator();

            ScrollArea::vertical().show(ui, |ui| {
                if self.show_raw {
                    self.show_raw_editor(ui);
                } else {
                    self.show_structured_editor(ui);
                }
            });
        });
    }

    fn show_raw_editor(&mut self, ui: &mut Ui) {
        let response = ui.add(
            TextEdit::multiline(&mut self.contents)
                .desired_width(ui.available_width())
                .code_editor(),
        );

        if response.changed() {
            self.modified = true;
            self.structured = None;
            self.parse_error = None;
        }
    }

    fn show_structured_editor(&mut self, ui: &mut Ui) {
        if let Some(value) = &mut self.structured {
            let changed = match value {
                Value::Table(table) => Self::render_table(ui, "", table),
                Value::Array(array) => Self::render_array(ui, "", array),
                _ => Self::render_entry(ui, "", "value", value),
            };

            if changed {
                self.modified = true;
                self.contents = Self::value_to_string(value);
            }
        } else {
            ui.label("Switch back to the raw editor to resolve parsing issues.");
        }
    }

    fn toggle_view(&mut self) {
        if self.show_raw {
            match self.parse_contents() {
                Ok(value) => {
                    self.structured = Some(value);
                    self.parse_error = None;
                    self.show_raw = false;
                }
                Err(err) => {
                    self.parse_error = Some(err);
                }
            }
        } else {
            if let Some(value) = &self.structured {
                self.contents = Self::value_to_string(value);
            }
            self.show_raw = true;
            self.structured = None;
            self.parse_error = None;
        }
    }

    fn parse_contents(&self) -> Result<Value, String> {
        self.contents
            .parse::<Value>()
            .map_err(|err| err.to_string())
    }

    fn render_table(ui: &mut Ui, path: &str, table: &mut Table) -> bool {
        let mut changed = false;
        for (key, value) in table.iter_mut() {
            let child_path = Self::join_path(path, key);
            if Self::render_entry(ui, &child_path, key, value) {
                changed = true;
            }
        }
        changed
    }

    fn render_array(ui: &mut Ui, path: &str, array: &mut Vec<Value>) -> bool {
        let mut changed = false;
        for (index, value) in array.iter_mut().enumerate() {
            let label = format!("[{index}]");
            let child_path = Self::join_index(path, index);
            if Self::render_entry(ui, &child_path, &label, value) {
                changed = true;
            }
        }
        changed
    }

    fn render_entry(ui: &mut Ui, path: &str, label: &str, value: &mut Value) -> bool {
        match value {
            Value::Table(table) => egui::CollapsingHeader::new(label)
                .id_source(path.to_owned())
                .default_open(true)
                .show(ui, |ui| Self::render_table(ui, path, table))
                .inner
                .unwrap_or(false),
            Value::Array(array) => egui::CollapsingHeader::new(label)
                .id_source(path.to_owned())
                .default_open(true)
                .show(ui, |ui| Self::render_array(ui, path, array))
                .inner
                .unwrap_or(false),
            Value::String(text) => {
                ui.horizontal(|ui| {
                    ui.label(label);
                    ui.text_edit_singleline(text).changed()
                })
                .inner
            }
            Value::Integer(integer) => {
                ui.horizontal(|ui| {
                    ui.label(label);
                    let mut value = *integer;
                    let changed = ui.add(DragValue::new(&mut value)).changed();
                    if changed {
                        *integer = value;
                    }
                    changed
                })
                .inner
            }
            Value::Float(float) => {
                ui.horizontal(|ui| {
                    ui.label(label);
                    let mut value = *float;
                    let changed = ui.add(DragValue::new(&mut value).speed(0.1)).changed();
                    if changed {
                        *float = value;
                    }
                    changed
                })
                .inner
            }
            Value::Boolean(boolean) => ui.checkbox(boolean, label).changed(),
            Value::Datetime(datetime) => {
                ui.horizontal(|ui| {
                    ui.label(label);
                    let mut text = datetime.to_string();
                    let response = ui.text_edit_singleline(&mut text);
                    if response.changed() {
                        match text.parse() {
                            Ok(parsed) => {
                                *datetime = parsed;
                                true
                            }
                            Err(_) => {
                                ui.colored_label(Color32::RED, "Invalid datetime");
                                false
                            }
                        }
                    } else {
                        false
                    }
                })
                .inner
            }
        }
    }

    fn join_path(path: &str, segment: &str) -> String {
        if path.is_empty() {
            segment.to_owned()
        } else {
            format!("{path}.{segment}")
        }
    }

    fn join_index(path: &str, index: usize) -> String {
        if path.is_empty() {
            format!("[{index}]")
        } else {
            format!("{path}[{index}]")
        }
    }

    fn value_to_string(value: &Value) -> String {
        toml::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
    }
}

impl Tab for PsuTomlViewer {
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
        if !self.show_raw {
            if let Some(value) = &self.structured {
                self.contents = Self::value_to_string(value);
            }
        }

        std::fs::write(&self.file_path, self.contents.as_bytes())
            .expect("Failed to write psu.toml");
        self.modified = false;
    }
}
