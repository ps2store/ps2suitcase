use eframe::egui::{self, Color32, RichText};

use crate::{
    ui::theme, IconFlagSelection, PackerApp, ICON_SYS_FLAG_OPTIONS, ICON_SYS_TITLE_CHAR_LIMIT,
};
use ps2_filetypes::sjis;
use psu_packer::{ColorConfig, ColorFConfig, IconSysConfig, VectorConfig};

const TITLE_CHAR_LIMIT: usize = ICON_SYS_TITLE_CHAR_LIMIT;
const TITLE_INPUT_WIDTH: f32 = (ICON_SYS_TITLE_CHAR_LIMIT as f32) * 9.0;

#[derive(Clone, Copy)]
struct IconSysPreset {
    id: &'static str,
    label: &'static str,
    background_transparency: u32,
    background_colors: [ColorConfig; 4],
    light_directions: [VectorConfig; 3],
    light_colors: [ColorFConfig; 3],
    ambient_color: ColorFConfig,
}

const ICON_SYS_PRESETS: &[IconSysPreset] = &[
    IconSysPreset {
        id: "default",
        label: "Standard (PS2)",
        background_transparency: IconSysConfig::default_background_transparency(),
        background_colors: IconSysConfig::default_background_colors(),
        light_directions: IconSysConfig::default_light_directions(),
        light_colors: IconSysConfig::default_light_colors(),
        ambient_color: IconSysConfig::default_ambient_color(),
    },
    IconSysPreset {
        id: "cool_blue",
        label: "Cool Blue",
        background_transparency: 0,
        background_colors: [
            ColorConfig {
                r: 0,
                g: 32,
                b: 96,
                a: 0,
            },
            ColorConfig {
                r: 0,
                g: 48,
                b: 128,
                a: 0,
            },
            ColorConfig {
                r: 0,
                g: 64,
                b: 160,
                a: 0,
            },
            ColorConfig {
                r: 0,
                g: 16,
                b: 48,
                a: 0,
            },
        ],
        light_directions: [
            VectorConfig {
                x: 0.0,
                y: 0.0,
                z: 1.0,
                w: 0.0,
            },
            VectorConfig {
                x: -0.5,
                y: -0.5,
                z: 0.5,
                w: 0.0,
            },
            VectorConfig {
                x: 0.5,
                y: -0.5,
                z: 0.5,
                w: 0.0,
            },
        ],
        light_colors: [
            ColorFConfig {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 1.0,
            },
            ColorFConfig {
                r: 0.5,
                g: 0.5,
                b: 0.6,
                a: 1.0,
            },
            ColorFConfig {
                r: 0.3,
                g: 0.3,
                b: 0.4,
                a: 1.0,
            },
        ],
        ambient_color: ColorFConfig {
            r: 0.2,
            g: 0.2,
            b: 0.2,
            a: 1.0,
        },
    },
    IconSysPreset {
        id: "warm_sunset",
        label: "Warm Sunset",
        background_transparency: 0,
        background_colors: [
            ColorConfig {
                r: 128,
                g: 48,
                b: 16,
                a: 0,
            },
            ColorConfig {
                r: 176,
                g: 72,
                b: 32,
                a: 0,
            },
            ColorConfig {
                r: 208,
                g: 112,
                b: 48,
                a: 0,
            },
            ColorConfig {
                r: 96,
                g: 32,
                b: 16,
                a: 0,
            },
        ],
        light_directions: [
            VectorConfig {
                x: -0.2,
                y: -0.4,
                z: 0.8,
                w: 0.0,
            },
            VectorConfig {
                x: 0.0,
                y: -0.6,
                z: 0.6,
                w: 0.0,
            },
            VectorConfig {
                x: 0.3,
                y: -0.5,
                z: 0.7,
                w: 0.0,
            },
        ],
        light_colors: [
            ColorFConfig {
                r: 1.0,
                g: 0.9,
                b: 0.75,
                a: 1.0,
            },
            ColorFConfig {
                r: 0.9,
                g: 0.6,
                b: 0.3,
                a: 1.0,
            },
            ColorFConfig {
                r: 0.6,
                g: 0.3,
                b: 0.2,
                a: 1.0,
            },
        ],
        ambient_color: ColorFConfig {
            r: 0.25,
            g: 0.18,
            b: 0.12,
            a: 1.0,
        },
    },
];

pub(crate) fn icon_sys_editor(app: &mut PackerApp, ui: &mut egui::Ui) {
    ui.heading(theme::display_heading_text(ui, "icon.sys metadata"));
    ui.small("Configure the save icon title, flags, and lighting.");
    ui.add_space(8.0);

    let mut config_changed = false;

    let checkbox = ui.checkbox(&mut app.icon_sys_enabled, "Enable icon.sys metadata");
    let checkbox_changed = checkbox.changed();
    checkbox
        .on_hover_text("Use an existing icon.sys file or generate a new one when packing the PSU.");

    if checkbox_changed {
        config_changed = true;
    }

    if !app.icon_sys_enabled {
        app.icon_sys_use_existing = false;
    } else if app.icon_sys_existing.is_none() {
        app.icon_sys_use_existing = false;
    }

    if app.icon_sys_enabled {
        if let Some(existing_icon) = app.icon_sys_existing.clone() {
            let previous = app.icon_sys_use_existing;
            ui.horizontal(|ui| {
                ui.label("Mode:");
                let use_existing = ui.selectable_value(
                    &mut app.icon_sys_use_existing,
                    true,
                    "Use existing icon.sys",
                );
                if use_existing.changed() {
                    config_changed = true;
                }
                let generate_new = ui.selectable_value(
                    &mut app.icon_sys_use_existing,
                    false,
                    "Generate new icon.sys",
                );
                if generate_new.changed() {
                    config_changed = true;
                }
            });

            if app.icon_sys_use_existing && !previous {
                app.apply_icon_sys_file(&existing_icon);
                config_changed = true;
            }

            if app.icon_sys_use_existing {
                ui.small(concat!(
                    "The existing icon.sys file will be packed without modification. ",
                    "Switch to \"Generate new icon.sys\" to edit metadata.",
                ));
            }
        }
    }

    ui.add_space(8.0);

    let enabled = app.icon_sys_enabled && !app.icon_sys_use_existing;
    let inner_response = ui.add_enabled_ui(enabled, |ui| {
        let mut inner_changed = false;
        inner_changed |= title_section(app, ui);
        ui.add_space(12.0);
        inner_changed |= flag_section(app, ui);
        ui.add_space(12.0);
        inner_changed |= presets_section(app, ui);
        ui.add_space(12.0);
        inner_changed |= background_section(app, ui);
        ui.add_space(12.0);
        inner_changed |= lighting_section(app, ui);
        inner_changed
    });

    if inner_response.inner {
        config_changed = true;
    }

    if config_changed {
        app.refresh_psu_toml_editor();
    }
}

fn title_section(app: &mut PackerApp, ui: &mut egui::Ui) -> bool {
    let mut changed = false;
    ui.group(|ui| {
        ui.heading(theme::display_heading_text(ui, "Title"));
        ui.small("Each line supports up to 16 characters that must round-trip through Shift-JIS");

        egui::Grid::new("icon_sys_title_grid")
            .num_columns(2)
            .spacing(egui::vec2(8.0, 4.0))
            .show(ui, |ui| {
                ui.label("Line 1");
                if title_input(
                    ui,
                    egui::Id::new("icon_sys_title_line1"),
                    &mut app.icon_sys_title_line1,
                ) {
                    changed = true;
                }
                ui.end_row();

                ui.label("Line 2");
                if title_input(
                    ui,
                    egui::Id::new("icon_sys_title_line2"),
                    &mut app.icon_sys_title_line2,
                ) {
                    changed = true;
                }
                ui.end_row();

                ui.label("Preview");
                ui.vertical(|ui| {
                    ui.monospace(format!(
                        "{:<width$}",
                        app.icon_sys_title_line1,
                        width = ICON_SYS_TITLE_CHAR_LIMIT
                    ));
                    ui.monospace(format!(
                        "{:<width$}",
                        app.icon_sys_title_line2,
                        width = ICON_SYS_TITLE_CHAR_LIMIT
                    ));

                    match sjis::encode_sjis(&app.icon_sys_title_line1) {
                        Ok(bytes) => {
                            let break_pos = bytes.len();
                            ui.small(format!("Shift-JIS byte length: {break_pos}"));
                            ui.small(format!("Line break position: {break_pos}"));
                        }
                        Err(_) => {
                            let warning = RichText::new(
                                "Shift-JIS byte length: invalid (non-encodable characters)",
                            )
                            .color(Color32::RED);
                            ui.small(warning);
                            ui.small(
                                RichText::new("Line break position: -- (invalid Shift-JIS)")
                                    .color(Color32::RED),
                            );
                        }
                    }
                });
                ui.end_row();
            });
    });
    changed
}

fn title_input(ui: &mut egui::Ui, id: egui::Id, value: &mut String) -> bool {
    let mut edit = egui::TextEdit::singleline(value)
        .char_limit(TITLE_CHAR_LIMIT)
        .desired_width(TITLE_INPUT_WIDTH);
    edit = edit.id_source(id);

    let response = ui.add(edit);
    let mut changed = false;
    if response.changed() {
        let mut sanitized = String::new();
        let mut accepted_chars = 0usize;
        for ch in value.chars() {
            if ch.is_control() {
                continue;
            }

            if accepted_chars >= TITLE_CHAR_LIMIT {
                break;
            }

            sanitized.push(ch);
            if sjis::is_roundtrip_sjis(&sanitized) {
                accepted_chars += 1;
            } else {
                sanitized.pop();
            }
        }
        if *value != sanitized {
            *value = sanitized;
        }
        changed = true;
    }

    let char_count = value.chars().count();
    ui.small(format!(
        "{char_count} / {TITLE_CHAR_LIMIT} characters (Shift-JIS compatible)"
    ));
    changed
}

fn flag_section(app: &mut PackerApp, ui: &mut egui::Ui) -> bool {
    let mut changed = false;
    ui.group(|ui| {
        ui.heading(theme::display_heading_text(ui, "Flags"));
        egui::Grid::new("icon_sys_flag_grid")
            .num_columns(2)
            .spacing(egui::vec2(8.0, 4.0))
            .show(ui, |ui| {
                ui.label("Icon type");
                ui.horizontal(|ui| {
                    egui::ComboBox::from_id_source("icon_sys_flag_combo")
                        .selected_text(app.icon_flag_label())
                        .show_ui(ui, |ui| {
                            for (idx, (_, label)) in ICON_SYS_FLAG_OPTIONS.iter().enumerate() {
                                let response = ui.selectable_value(
                                    &mut app.icon_sys_flag_selection,
                                    IconFlagSelection::Preset(idx),
                                    *label,
                                );
                                if response.changed() {
                                    changed = true;
                                }
                            }
                            let response = ui.selectable_value(
                                &mut app.icon_sys_flag_selection,
                                IconFlagSelection::Custom,
                                "Custom…",
                            );
                            if response.changed() {
                                changed = true;
                            }
                        });

                    if matches!(app.icon_sys_flag_selection, IconFlagSelection::Custom) {
                        let response = ui.add(
                            egui::DragValue::new(&mut app.icon_sys_custom_flag)
                                .clamp_range(0.0..=u16::MAX as f64)
                                .speed(1),
                        );
                        if response.changed() {
                            changed = true;
                        }
                        response.on_hover_text("Enter the raw flag value (0-65535).");
                        ui.label(format!("0x{:04X}", app.icon_sys_custom_flag));
                    }
                });
                ui.end_row();
            });
    });
    changed
}

fn presets_section(app: &mut PackerApp, ui: &mut egui::Ui) -> bool {
    let mut changed = false;
    ui.group(|ui| {
        ui.heading(theme::display_heading_text(ui, "Presets"));
        ui.small("Choose a preset to populate the colors and lights automatically.");

        let selected_label = match app.icon_sys_selected_preset.as_deref() {
            Some(id) => find_preset(id)
                .map(|preset| preset.label.to_string())
                .unwrap_or_else(|| format!("Custom ({id})")),
            None => "Manual".to_string(),
        };

        egui::ComboBox::from_id_source("icon_sys_preset_combo")
            .selected_text(selected_label)
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(app.icon_sys_selected_preset.is_none(), "Manual")
                    .clicked()
                {
                    app.clear_icon_sys_preset();
                    changed = true;
                }
                for preset in ICON_SYS_PRESETS {
                    let selected = app
                        .icon_sys_selected_preset
                        .as_deref()
                        .map(|id| id == preset.id)
                        .unwrap_or(false);
                    if ui.selectable_label(selected, preset.label).clicked() {
                        apply_preset(app, preset);
                        changed = true;
                    }
                }
            });

        ui.add_space(6.0);
        preset_preview(app, ui);
    });
    changed
}

fn preset_preview(app: &PackerApp, ui: &mut egui::Ui) {
    ui.vertical(|ui| {
        ui.label("Background gradient");
        ui.horizontal(|ui| {
            for color in app.icon_sys_background_colors {
                draw_color_swatch(ui, color32_from_color_config(color));
            }
        });

        ui.label("Light colors");
        ui.horizontal(|ui| {
            for color in app.icon_sys_light_colors {
                draw_color_swatch(ui, color32_from_color_f_config(color));
            }
        });

        ui.label("Ambient");
        draw_color_swatch(ui, color32_from_color_f_config(app.icon_sys_ambient_color));
    });
}

fn draw_color_swatch(ui: &mut egui::Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(20.0, 14.0), egui::Sense::hover());
    ui.painter().rect_filled(rect, 3.0, color);
}

fn background_section(app: &mut PackerApp, ui: &mut egui::Ui) -> bool {
    let mut changed = false;
    ui.group(|ui| {
        ui.heading(theme::display_heading_text(ui, "Background"));
        ui.small("Adjust the gradient colors and alpha layer.");

        if ui
            .add(
                egui::DragValue::new(&mut app.icon_sys_background_transparency)
                    .clamp_range(0.0..=255.0)
                    .speed(1)
                    .suffix(" α"),
            )
            .changed()
        {
            app.clear_icon_sys_preset();
            changed = true;
        }

        let mut background_changed = false;
        egui::Grid::new("icon_sys_background_grid")
            .num_columns(2)
            .spacing(egui::vec2(8.0, 4.0))
            .show(ui, |ui| {
                for (index, color) in app.icon_sys_background_colors.iter_mut().enumerate() {
                    ui.label(format!("Color {}", index + 1));
                    let mut display = color32_from_color_config(*color);
                    if ui.color_edit_button_srgba(&mut display).changed() {
                        *color = color_config_from_color32(display);
                        background_changed = true;
                    }
                    ui.end_row();
                }
            });
        if background_changed {
            app.clear_icon_sys_preset();
            changed = true;
        }
    });
    changed
}

fn lighting_section(app: &mut PackerApp, ui: &mut egui::Ui) -> bool {
    let mut changed = false;
    ui.group(|ui| {
        ui.heading(theme::display_heading_text(ui, "Lighting"));
        ui.small("Tweak light directions, colors, and the ambient glow.");

        let mut lighting_changed = false;

        for (index, (color, direction)) in app
            .icon_sys_light_colors
            .iter_mut()
            .zip(app.icon_sys_light_directions.iter_mut())
            .enumerate()
        {
            let mut light_dirty = false;
            ui.collapsing(format!("Light {}", index + 1), |ui| {
                ui.label("Color");
                let mut rgba = color_f_config_to_array(*color);
                if ui.color_edit_button_rgba_unmultiplied(&mut rgba).changed() {
                    *color = array_to_color_f_config(rgba);
                    light_dirty = true;
                }

                ui.add_space(4.0);
                ui.label("Direction");
                for (label, component) in [
                    ("x", &mut direction.x),
                    ("y", &mut direction.y),
                    ("z", &mut direction.z),
                    ("w", &mut direction.w),
                ] {
                    ui.horizontal(|ui| {
                        ui.label(label);
                        if ui
                            .add(
                                egui::DragValue::new(component)
                                    .clamp_range(-1.0..=1.0)
                                    .speed(0.01),
                            )
                            .changed()
                        {
                            light_dirty = true;
                        }
                    });
                }
            });
            if light_dirty {
                lighting_changed = true;
            }
            ui.add_space(4.0);
        }

        ui.label("Ambient color");
        let mut ambient = color_f_config_to_array(app.icon_sys_ambient_color);
        if ui
            .color_edit_button_rgba_unmultiplied(&mut ambient)
            .changed()
        {
            app.icon_sys_ambient_color = array_to_color_f_config(ambient);
            lighting_changed = true;
        }

        if lighting_changed {
            app.clear_icon_sys_preset();
            changed = true;
        }
    });
    changed
}

fn apply_preset(app: &mut PackerApp, preset: &IconSysPreset) {
    app.icon_sys_background_transparency = preset.background_transparency;
    app.icon_sys_background_colors = preset.background_colors;
    app.icon_sys_light_directions = preset.light_directions;
    app.icon_sys_light_colors = preset.light_colors;
    app.icon_sys_ambient_color = preset.ambient_color;
    app.icon_sys_selected_preset = Some(preset.id.to_string());
}

fn find_preset(id: &str) -> Option<&'static IconSysPreset> {
    ICON_SYS_PRESETS.iter().find(|preset| preset.id == id)
}

fn color32_from_color_config(color: ColorConfig) -> Color32 {
    Color32::from_rgba_unmultiplied(color.r, color.g, color.b, color.a)
}

fn color32_from_color_f_config(color: ColorFConfig) -> Color32 {
    let clamp = |value: f32| -> u8 { (value.clamp(0.0, 1.0) * 255.0).round() as u8 };
    Color32::from_rgba_unmultiplied(
        clamp(color.r),
        clamp(color.g),
        clamp(color.b),
        clamp(color.a),
    )
}

fn color_config_from_color32(color: Color32) -> ColorConfig {
    ColorConfig {
        r: color.r(),
        g: color.g(),
        b: color.b(),
        a: color.a(),
    }
}

fn color_f_config_to_array(color: ColorFConfig) -> [f32; 4] {
    [color.r, color.g, color.b, color.a]
}

fn array_to_color_f_config(color: [f32; 4]) -> ColorFConfig {
    ColorFConfig {
        r: color[0],
        g: color[1],
        b: color[2],
        a: color[3],
    }
}
