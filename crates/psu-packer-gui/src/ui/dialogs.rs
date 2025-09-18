use eframe::egui;

use crate::PackerApp;

pub(crate) fn pack_confirmation(app: &mut PackerApp, ctx: &egui::Context) {
    if let Some(missing) = app.pending_pack_missing_files() {
        let message = PackerApp::format_missing_required_files_message(missing);
        egui::Window::new("Confirm Packing")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(&message);
                ui.add_space(12.0);
                ui.label("Pack anyway?");
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Proceed").clicked() {
                        app.confirm_pending_pack_action();
                    }
                    if ui.button("Go Back").clicked() {
                        app.cancel_pending_pack_action();
                    }
                });
            });
    }
}

pub(crate) fn exit_confirmation(app: &mut PackerApp, ctx: &egui::Context) {
    if app.show_exit_confirm {
        egui::Window::new("Confirm Exit")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label("Are you sure you want to exit?");
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let yes_clicked = ui.button("Yes").clicked();
                    let no_clicked = ui.button("No").clicked();

                    if yes_clicked {
                        app.show_exit_confirm = false;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    } else if no_clicked {
                        app.show_exit_confirm = false;
                    }
                });
            });
    }
}
