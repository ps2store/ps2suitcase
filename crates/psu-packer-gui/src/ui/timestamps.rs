use chrono::{Local, NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use eframe::egui;
use egui_extras::DatePickerButton;

use crate::{PackerApp, TimestampStrategy, TIMESTAMP_FORMAT};

pub(crate) fn metadata_timestamp_section(app: &mut PackerApp, ui: &mut egui::Ui) {
    ui.vertical(|ui| {
        let default_timestamp = default_timestamp();
        let source_timestamp = app.source_timestamp;
        let planned_timestamp = app.planned_timestamp_for_current_source();
        let recommended_strategy = recommended_timestamp_strategy(source_timestamp, planned_timestamp);

        ui.small(
            "Deterministic timestamps ensure repeated packs produce identical archives for verification.",
        );
        ui.add_space(6.0);

        let mut strategy = app.timestamp_strategy;
        let recommended_badge = |ui: &mut egui::Ui| {
            let badge_text = egui::RichText::new("Recommended")
                .color(egui::Color32::WHITE)
                .background_color(egui::Color32::from_rgb(38, 166, 65))
                .strong();
            ui.add(egui::Label::new(badge_text))
                .on_hover_text("Best choice based on the available metadata");
        };

        ui.group(|ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    let response = ui.radio_value(
                        &mut strategy,
                        TimestampStrategy::None,
                        "No timestamp",
                    );
                    if response.changed()
                        && app.timestamp_strategy != TimestampStrategy::None
                        && strategy == TimestampStrategy::None
                    {
                        app.set_timestamp_strategy(strategy);
                    }
                });
                ui.label("• Use when verifying contents does not require metadata timestamps.");
                ui.label("• Relies on: no metadata—timestamp field will be omitted.");
            });
        });

        ui.add_space(6.0);

        ui.group(|ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    let response = ui.radio_value(
                        &mut strategy,
                        TimestampStrategy::InheritSource,
                        "Use source timestamp",
                    );
                    if recommended_strategy == Some(TimestampStrategy::InheritSource) {
                        recommended_badge(ui);
                    }
                    if response.changed()
                        && app.timestamp_strategy != TimestampStrategy::InheritSource
                        && strategy == TimestampStrategy::InheritSource
                    {
                        app.set_timestamp_strategy(strategy);
                    }
                });
                ui.label("• Use when the loaded source already contains a trusted timestamp.");
                ui.label(format!(
                    "• Relies on: Source timestamp ({}).",
                    availability_text(source_timestamp, "available", "unavailable")
                ));
                if let Some(ts) = source_timestamp {
                    ui.small(format!("  Source value: {}", ts.format(TIMESTAMP_FORMAT)));
                }
            });
        });

        ui.add_space(6.0);

        ui.group(|ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    let response = ui.radio_value(
                        &mut strategy,
                        TimestampStrategy::SasRules,
                        "Use SAS prefix rules",
                    );
                    if recommended_strategy == Some(TimestampStrategy::SasRules) {
                        recommended_badge(ui);
                    }
                    if response.changed()
                        && app.timestamp_strategy != TimestampStrategy::SasRules
                        && strategy == TimestampStrategy::SasRules
                    {
                        app.set_timestamp_strategy(strategy);
                    }
                });
                ui.label("• Use when project names follow SAS conventions for deterministic scheduling.");
                let project_name = project_name_text(app);
                ui.label(format!(
                    "• Relies on: Project name ({project_name}) and timestamp rules (planned value {}).",
                    availability_text(planned_timestamp, "available", "unavailable")
                ));
                if let Some(ts) = planned_timestamp {
                    ui.small(format!("  Planned value: {}", ts.format(TIMESTAMP_FORMAT)));
                }
            });
        });

        ui.add_space(6.0);

        let mut manual_timestamp_changed = false;

        ui.group(|ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    let response = ui.radio_value(
                        &mut strategy,
                        TimestampStrategy::Manual,
                        "Manual timestamp",
                    );
                    if recommended_strategy == Some(TimestampStrategy::Manual) {
                        recommended_badge(ui);
                    }
                    if response.changed()
                        && app.timestamp_strategy != TimestampStrategy::Manual
                        && strategy == TimestampStrategy::Manual
                    {
                        app.set_timestamp_strategy(strategy);
                    }
                });
                ui.label("• Use when you must pin the archive to an explicit, reviewer-approved timestamp.");
                ui.label("• Relies on: Manual date and time you enter here.");

                if strategy == TimestampStrategy::Manual
                    && app.manual_timestamp.is_none()
                {
                    app.manual_timestamp = Some(default_timestamp);
                    app.refresh_timestamp_from_strategy();
                }

                if strategy == TimestampStrategy::Manual {
                    let mut timestamp = app.manual_timestamp.unwrap_or(default_timestamp);
                    let mut date: NaiveDate = timestamp.date();
                    let time = timestamp.time();
                    let mut hour = time.hour();
                    let mut minute = time.minute();
                    let mut second = time.second();
                    let mut changed = false;

                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        let date_response = ui.add(
                            DatePickerButton::new(&mut date)
                                .id_source("metadata_timestamp_date_picker"),
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
                            app.manual_timestamp = Some(timestamp);
                            manual_timestamp_changed = true;
                        }
                    } else if app.manual_timestamp != Some(timestamp) {
                        app.manual_timestamp = Some(timestamp);
                        manual_timestamp_changed = true;
                    }

                    if let Some(ts) = app.manual_timestamp {
                        ui.small(format!("Selected: {}", ts.format(TIMESTAMP_FORMAT)));
                    }

                    if let Some(planned) = planned_timestamp {
                        if ui.button("Copy planned timestamp").clicked() {
                            app.manual_timestamp = Some(planned);
                            manual_timestamp_changed = true;
                        }
                    }
                }
            });
        });

        if strategy != app.timestamp_strategy {
            app.set_timestamp_strategy(strategy);
        }

        if manual_timestamp_changed {
            app.refresh_timestamp_from_strategy();
        }

        ui.add_space(8.0);

        let summary_title = current_strategy_title(app.timestamp_strategy);
        let summary_reason = current_strategy_reason(app, source_timestamp, planned_timestamp);
        let summary_text = format!("Currently using: {summary_title} because {summary_reason}.");

        ui.group(|ui| {
            ui.label(egui::RichText::new(summary_text).strong());
        });

        ui.add_space(6.0);
    });
}

fn recommended_timestamp_strategy(
    source_timestamp: Option<NaiveDateTime>,
    planned_timestamp: Option<NaiveDateTime>,
) -> Option<TimestampStrategy> {
    if source_timestamp.is_some() {
        Some(TimestampStrategy::InheritSource)
    } else if planned_timestamp.is_some() {
        Some(TimestampStrategy::SasRules)
    } else {
        Some(TimestampStrategy::Manual)
    }
}

fn availability_text(
    timestamp: Option<NaiveDateTime>,
    available_text: &str,
    unavailable_text: &str,
) -> String {
    if timestamp.is_some() {
        available_text.to_string()
    } else {
        unavailable_text.to_string()
    }
}

fn project_name_text(app: &PackerApp) -> String {
    let name = app.folder_name();
    if name.trim().is_empty() {
        "not set".to_string()
    } else {
        name
    }
}

fn current_strategy_title(strategy: TimestampStrategy) -> &'static str {
    match strategy {
        TimestampStrategy::None => "No timestamp",
        TimestampStrategy::InheritSource => "Inherited source timestamp",
        TimestampStrategy::SasRules => "SAS rules timestamp",
        TimestampStrategy::Manual => "Manual timestamp",
    }
}

fn current_strategy_reason(
    app: &PackerApp,
    source_timestamp: Option<NaiveDateTime>,
    planned_timestamp: Option<NaiveDateTime>,
) -> String {
    match app.timestamp_strategy {
        TimestampStrategy::None => {
            "timestamps are intentionally omitted from the archive".to_string()
        }
        TimestampStrategy::InheritSource => match source_timestamp {
            Some(ts) => format!(
                "the loaded source provided {} to preserve",
                ts.format(TIMESTAMP_FORMAT)
            ),
            None => "no source timestamp was found to inherit".to_string(),
        },
        TimestampStrategy::SasRules => match planned_timestamp {
            Some(ts) => format!(
                "SAS rules computed {} for {}",
                ts.format(TIMESTAMP_FORMAT),
                app.folder_name()
            ),
            None => "automatic SAS rules could not determine a timestamp".to_string(),
        },
        TimestampStrategy::Manual => match app.manual_timestamp {
            Some(ts) => format!("you entered {}", ts.format(TIMESTAMP_FORMAT)),
            None => "a manual timestamp is required until other data is provided".to_string(),
        },
    }
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
        ui.label(format!("Configuration file: {}", path.display()));
    } else {
        ui.small("Select a project folder to save these settings alongside psu.toml.");
    }

    if app.timestamp_rules_modified {
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
                    if matches!(app.timestamp_strategy, TimestampStrategy::SasRules) {
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
                if matches!(app.timestamp_strategy, TimestampStrategy::SasRules) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PackerApp, SasPrefix, TimestampStrategy};
    use chrono::{Duration, NaiveDate};
    use eframe::egui;

    #[test]
    fn summary_references_source_when_inheriting() {
        let mut app = PackerApp::default();
        let source = NaiveDate::from_ymd_opt(2024, 1, 2)
            .unwrap()
            .and_hms_opt(3, 4, 5)
            .unwrap();
        app.source_timestamp = Some(source);
        app.timestamp_strategy = TimestampStrategy::InheritSource;
        app.refresh_timestamp_from_strategy();

        let rendered = render_metadata_text(&mut app);

        assert!(rendered.contains("No timestamp"));
        assert!(rendered.contains("Source timestamp (available)"));
        assert!(rendered.contains(
            "Currently using: Inherited source timestamp because the loaded source provided 2024-01-02 03:04:05 to preserve."
        ));
        assert!(rendered.contains("Recommended"));
    }

    #[test]
    fn summary_references_planned_when_using_sas_rules() {
        let mut app = PackerApp::default();
        app.source_timestamp = None;
        app.selected_prefix = SasPrefix::App;
        app.folder_base_name = "TEST".to_string();
        app.timestamp_strategy = TimestampStrategy::SasRules;
        app.refresh_timestamp_from_strategy();

        let rendered = render_metadata_text(&mut app);

        assert!(rendered.contains("Project name (APP_TEST)"));
        assert!(rendered.contains("planned value available"));
        assert!(rendered.contains("SAS rules timestamp because SAS rules computed"));
        assert!(rendered.contains("Recommended"));
    }

    #[test]
    fn manual_summary_updates_after_manual_timestamp_change() {
        let mut app = PackerApp::default();
        app.source_timestamp = None;
        app.timestamp_strategy = TimestampStrategy::Manual;
        let initial = NaiveDate::from_ymd_opt(2024, 5, 6)
            .unwrap()
            .and_hms_opt(7, 8, 9)
            .unwrap();
        app.manual_timestamp = Some(initial);
        app.refresh_timestamp_from_strategy();

        let rendered = render_metadata_text(&mut app);
        assert!(rendered.contains(
            "Currently using: Manual timestamp because you entered 2024-05-06 07:08:09."
        ));

        let updated = initial + Duration::minutes(5);
        app.manual_timestamp = Some(updated);
        app.refresh_timestamp_from_strategy();

        let rerendered = render_metadata_text(&mut app);
        assert!(rerendered.contains(
            "Currently using: Manual timestamp because you entered 2024-05-06 07:13:09."
        ));
    }

    fn render_metadata_text(app: &mut PackerApp) -> String {
        let ctx = egui::Context::default();
        ctx.begin_frame(egui::RawInput::default());
        egui::CentralPanel::default().show(&ctx, |ui| {
            metadata_timestamp_section(app, ui);
        });
        let full_output = ctx.end_frame();
        let mut texts = Vec::new();
        collect_text_from_clipped_shapes(&full_output.shapes, &mut texts);
        texts.join("\n")
    }

    fn collect_text_from_clipped_shapes(
        shapes: &[egui::epaint::ClippedShape],
        output: &mut Vec<String>,
    ) {
        for clipped in shapes {
            collect_text_from_shape(&clipped.shape, output);
        }
    }

    fn collect_text_from_shape(shape: &egui::epaint::Shape, output: &mut Vec<String>) {
        match shape {
            egui::epaint::Shape::Vec(shapes) => {
                for nested in shapes {
                    collect_text_from_shape(nested, output);
                }
            }
            egui::epaint::Shape::Text(text_shape) => {
                output.push(text_shape.galley.text().to_string());
            }
            _ => {}
        }
    }
}
