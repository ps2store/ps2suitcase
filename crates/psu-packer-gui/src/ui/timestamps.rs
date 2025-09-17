use chrono::{Local, NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use eframe::egui;
use egui_extras::DatePickerButton;

use crate::{PackerApp, TIMESTAMP_FORMAT};

pub(crate) fn metadata_timestamp_section(app: &mut PackerApp, ui: &mut egui::Ui) {
    ui.vertical(|ui| {
        let default_timestamp = default_timestamp();
        let mut has_timestamp = app.timestamp.is_some();
        let mut new_timestamp = app.timestamp;
        let mut manual_change = false;

        if ui.checkbox(&mut has_timestamp, "Set timestamp").changed() {
            if has_timestamp {
                new_timestamp = Some(new_timestamp.unwrap_or(default_timestamp));
            } else {
                new_timestamp = None;
            }
            manual_change = true;
        }

        if !has_timestamp {
            ui.small("No timestamp will be saved.");
            new_timestamp = None;
        } else {
            let mut timestamp = new_timestamp.unwrap_or(default_timestamp);
            let mut date: NaiveDate = timestamp.date();
            let time = timestamp.time();
            let mut hour = time.hour();
            let mut minute = time.minute();
            let mut second = time.second();
            let mut changed = false;

            ui.horizontal(|ui| {
                let date_response = ui.add(
                    DatePickerButton::new(&mut date).id_source("metadata_timestamp_date_picker"),
                );
                changed |= date_response.changed();

                ui.label("Time");
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut hour)
                            .clamp_range(0..=23)
                            .suffix(" h"),
                    )
                    .changed();
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut minute)
                            .clamp_range(0..=59)
                            .suffix(" m"),
                    )
                    .changed();
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut second)
                            .clamp_range(0..=59)
                            .suffix(" s"),
                    )
                    .changed();
            });

            if changed {
                if let Some(new_time) = NaiveTime::from_hms_opt(hour, minute, second) {
                    timestamp = NaiveDateTime::new(date, new_time);
                    manual_change = true;
                }
            }

            new_timestamp = Some(timestamp);

            if let Some(ts) = new_timestamp {
                ui.small(format!("Selected: {}", ts.format(TIMESTAMP_FORMAT)));
            }
        }

        if manual_change {
            app.timestamp_from_rules = false;
        }

        if app.timestamp != new_timestamp {
            app.timestamp = new_timestamp;
            app.refresh_psu_toml_editor();
        }

        if app.folder.is_some() {
            let planned = app.planned_timestamp_for_current_folder();
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                let planned_label = planned
                    .map(|ts| ts.format(TIMESTAMP_FORMAT).to_string())
                    .unwrap_or_else(|| "Unavailable".to_string());
                ui.label(format!("Planned timestamp: {planned_label}"));
                let apply_button =
                    ui.add_enabled(planned.is_some(), egui::Button::new("Apply planned"));
                if apply_button.clicked() {
                    app.apply_planned_timestamp();
                }
            });
            if app.timestamp_from_rules {
                ui.small("Metadata timestamp follows the automatic plan.");
            }
        } else {
            app.timestamp_from_rules = false;
        }
    });
}

pub(crate) fn timestamp_rules_editor(app: &mut PackerApp, ui: &mut egui::Ui) {
    app.timestamp_rules_ui.ensure_matches(&app.timestamp_rules);

    ui.heading("Automatic timestamp rules");
    ui.small("Adjust deterministic timestamp spacing, category order, and aliases.");

    if let Some(error) = &app.timestamp_rules_error {
        ui.add_space(6.0);
        ui.colored_label(egui::Color32::YELLOW, error);
    }

    if let Some(path) = app.timestamp_rules_path() {
        ui.add_space(4.0);
        ui.label(format!("Configuration file: {}", path.display()));
    } else {
        ui.add_space(4.0);
        ui.small("Select a project folder to save these settings alongside psu.toml.");
    }

    if app.timestamp_rules_modified {
        ui.add_space(4.0);
        ui.colored_label(egui::Color32::LIGHT_YELLOW, "Unsaved changes");
    }

    ui.add_space(8.0);
    egui::Grid::new("timestamp_rules_settings")
        .num_columns(2)
        .spacing(egui::vec2(12.0, 6.0))
        .show(ui, |ui| {
            ui.label("Seconds between items");
            let mut seconds = app.timestamp_rules.seconds_between_items.max(1);
            if ui
                .add(
                    egui::DragValue::new(&mut seconds)
                        .clamp_range(1..=3600)
                        .speed(1.0),
                )
                .changed()
            {
                app.timestamp_rules.seconds_between_items = seconds.max(1);
                app.mark_timestamp_rules_modified();
            }
            ui.end_row();

            ui.label("Slots per category");
            let mut slots = app.timestamp_rules.slots_per_category.max(1);
            if ui
                .add(
                    egui::DragValue::new(&mut slots)
                        .clamp_range(1..=200_000)
                        .speed(10.0),
                )
                .changed()
            {
                app.timestamp_rules.slots_per_category = slots.max(1);
                app.mark_timestamp_rules_modified();
            }
            ui.end_row();
        });

    ui.add_space(12.0);
    ui.heading("Category order and aliases");
    ui.small("Aliases map names without prefixes to their categories (one per line).");
    ui.add_space(6.0);

    let mut move_request: Option<(usize, MoveDirection)> = None;
    let category_len = app.timestamp_rules.categories.len();

    for index in 0..category_len {
        let key = app.timestamp_rules.categories[index].key.clone();
        let alias_count = app.timestamp_rules.categories[index].aliases.len();
        let header_title = if alias_count == 1 {
            format!("{key} (1 alias)")
        } else {
            format!("{key} ({alias_count} aliases)")
        };

        egui::CollapsingHeader::new(header_title)
            .id_source(format!("timestamp_category_{index}"))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(index > 0, egui::Button::new("Move up"))
                        .clicked()
                    {
                        move_request = Some((index, MoveDirection::Up));
                    }
                    if ui
                        .add_enabled(index + 1 < category_len, egui::Button::new("Move down"))
                        .clicked()
                    {
                        move_request = Some((index, MoveDirection::Down));
                    }
                });

                ui.add_space(4.0);
                ui.label("Aliases (one per line):");
                if let Some(buffer) = app.timestamp_rules_ui.alias_texts.get_mut(index) {
                    if ui
                        .add(
                            egui::TextEdit::multiline(buffer)
                                .desired_rows(3)
                                .hint_text("Alias names without prefixes"),
                        )
                        .changed()
                    {
                        let parsed = parse_aliases(buffer, &key);
                        app.set_timestamp_aliases(index, parsed);
                    }
                }
            });
        ui.add_space(6.0);
    }

    if let Some((index, direction)) = move_request {
        match direction {
            MoveDirection::Up => app.move_timestamp_category_up(index),
            MoveDirection::Down => app.move_timestamp_category_down(index),
        }
    }

    ui.add_space(10.0);
    ui.horizontal(|ui| {
        if ui.button("Restore defaults").clicked() {
            app.reset_timestamp_rules_to_default();
        }

        let save_enabled = app.folder.is_some();
        if ui
            .add_enabled(save_enabled, egui::Button::new("Save"))
            .clicked()
        {
            match app.save_timestamp_rules() {
                Ok(path) => {
                    app.status = format!("Saved timestamp rules to {}", path.display());
                    app.clear_error_message();
                    if app.timestamp_from_rules {
                        app.apply_planned_timestamp();
                    }
                }
                Err(err) => app.set_error_message(err),
            }
        }

        if ui
            .add_enabled(save_enabled, egui::Button::new("Reload from disk"))
            .clicked()
        {
            if let Some(folder) = app.folder.clone() {
                app.load_timestamp_rules_from_folder(&folder);
                if app.timestamp_from_rules {
                    app.apply_planned_timestamp();
                }
            }
        }
    });
}

fn default_timestamp() -> NaiveDateTime {
    let now = Local::now().naive_local();
    now.with_nanosecond(0).unwrap_or(now)
}

fn parse_aliases(input: &str, key: &str) -> Vec<String> {
    let mut parsed = Vec::new();

    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let mut value = trimmed.to_ascii_uppercase();
        if key != "APPS" && key != "DEFAULT" && value.starts_with(key) {
            value = value[key.len()..].to_string();
        }
        if value.is_empty() {
            continue;
        }
        if !parsed.contains(&value) {
            parsed.push(value);
        }
    }

    parsed
}

#[derive(Clone, Copy)]
enum MoveDirection {
    Up,
    Down,
}
