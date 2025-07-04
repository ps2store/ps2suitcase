use eframe::egui::{vec2, Area, Color32, Context, CornerRadius, Id, Pos2, Sense, Widget, Window};
use egui_dock::egui::Align2;
use std::hash::Hash;

pub trait Wizard: Widget {
    fn show_modal(self, ctx: &Context, open: &mut bool)
    where
        Self: Sized,
    {
        if *open {
            let area_resp = Area::new(Id::new(self.get_id()))
                .fixed_pos(Pos2::ZERO)
                .show(ctx, |ui| {
                    let screen_rect = ui.ctx().input(|i| i.screen_rect);
                    let area_response = ui.allocate_response(screen_rect.size(), Sense::click());
                    if area_response.clicked() {
                        *open = false;
                    }
                    ui.painter().rect_filled(
                        screen_rect,
                        CornerRadius::ZERO,
                        Color32::from_rgba_premultiplied(0, 0, 0, 100),
                    );
                });

            ctx.move_to_top(area_resp.response.layer_id);

            let window = Window::new("")
                .id(Id::new(self.get_id()).with("content"))
                .open(open)
                .title_bar(false)
                .anchor(Align2::CENTER_CENTER, [0., 0.])
                .resizable(false)
                .show(ctx, |ui| {
                    ui.set_min_size(vec2(300.0, 200.0));
                    ui.add(self);
                });

            if let Some(resp) = window {
                ctx.move_to_top(resp.response.layer_id);
            }
        }
    }

    fn get_id(&self) -> impl Hash;
}
