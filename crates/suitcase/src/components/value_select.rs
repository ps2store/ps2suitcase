use eframe::egui::{CornerRadius, Id, PopupCloseBehavior, Response, Ui};
use std::ops::Add;

pub fn value_select(
    ui: &mut Ui,
    name: impl Into<String>,
    selected_value: &mut String,
    values: &[String],
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

    eframe::egui::popup_below_widget(ui, id, &res, PopupCloseBehavior::CloseOnClick, |ui| {
        ui.set_min_width(200.0);
        values.iter().for_each(|value| {
            if ui.selectable_label(false, value.clone()).clicked() {
                *selected_value = value.clone();
                layout_response.inner.0.mark_changed();
            }
        });
    });

    layout_response.inner.0
}

fn set_border_radius(ui: &mut Ui, radius: CornerRadius) {
    ui.style_mut().visuals.widgets.hovered.corner_radius = radius.add(CornerRadius::same(1));
    ui.style_mut().visuals.widgets.inactive.corner_radius = radius;
    ui.style_mut().visuals.widgets.active.corner_radius = radius;
}
