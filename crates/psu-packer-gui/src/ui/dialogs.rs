use eframe::egui;

use crate::PackerApp;

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
