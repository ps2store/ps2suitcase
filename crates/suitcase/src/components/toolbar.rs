use crate::components::buttons::CustomButtons;
use crate::data::state::AppState;
use eframe::egui::{include_image, menu, ImageSource, Response, Ui};

fn toolbar_item(ui: &mut Ui, source: ImageSource, tooltip: impl Into<String>) -> Response {
    ui.icon_button(source).on_hover_ui(|ui| {
        ui.label(tooltip.into());
    })
}

pub fn toolbar(ui: &mut Ui, app: &mut AppState) -> Response {
    menu::bar(ui, |ui| {
        ui.set_min_height(32.0);
        if toolbar_item(
            ui,
            include_image!("../../assets/hidpi/main_open_dir.png"),
            "Open a directory",
        )
        .clicked()
        {
            app.open_folder();
        }
        if toolbar_item(
            ui,
            include_image!("../../assets/hidpi/main_open_sav.png"),
            "Open a save file",
        )
        .clicked()
        {
            app.open_save();
        }
        ui.add_enabled_ui(false, |ui| {
            toolbar_item(
                ui,
                include_image!("../../assets/hidpi/main_open_vmc.png"),
                "Open a virtual memory card",
            );
        });
        toolbar_item(
            ui,
            include_image!("../../assets/hidpi/main_mk_titlecfg.png"),
            "Make title configuration",
        )
        .clicked()
        .then(|| app.create_title_cfg());
        if toolbar_item(
            ui,
            include_image!("../../assets/hidpi/main_mk_iconsys.png"),
            "Create ICN file",
        )
        .clicked()
        {
            app.create_icn();
        }
        toolbar_item(
            ui,
            include_image!("../../assets/hidpi/main_sav_meta.png"),
            "Save metadata",
        );
        toolbar_item(
            ui,
            include_image!("../../assets/hidpi/main_extract_all.png"),
            "Extract all saves",
        );
        toolbar_item(
            ui,
            include_image!("../../assets/hidpi/main_mk_sav.png"),
            "Make save file",
        );
        toolbar_item(
            ui,
            include_image!("../../assets/hidpi/main_mk_vmc.png"),
            "Make virtual memory card",
        );
        if toolbar_item(
            ui,
            include_image!("../../assets/hidpi/main_valid_ok.png"),
            "Validate save file",
        )
        .clicked()
        {
            app.validate();
        }

        if !app.pcsx2_path.is_empty() {
            if toolbar_item(
                ui,
                include_image!("../../assets/hidpi/main_emu_osdsys.png"),
                "Boot OSDSYS",
            )
            .clicked()
            {
                app.start_pcsx2();
            }
        }
    })
    .response
}
