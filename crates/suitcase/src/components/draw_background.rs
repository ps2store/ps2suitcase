use eframe::egui;
use eframe::egui::{Color32, Ui};

pub fn draw_background(ui: &mut Ui, colors: &[Color32; 4]) {
    let rect = ui.available_rect_before_wrap();
    let painter = ui.painter_at(rect);

    let top_left = rect.left_top();
    let top_right = rect.right_top();
    let bottom_left = rect.left_bottom();
    let bottom_right = rect.right_bottom();

    let mut mesh = egui::epaint::Mesh::default();

    let i0 = mesh.vertices.len() as u32;
    mesh.vertices.push(egui::epaint::Vertex {
        pos: top_left,
        uv: egui::epaint::WHITE_UV,
        color: colors[0],
    });
    mesh.vertices.push(egui::epaint::Vertex {
        pos: top_right,
        uv: egui::epaint::WHITE_UV,
        color: colors[1],
    });
    mesh.vertices.push(egui::epaint::Vertex {
        pos: bottom_right,
        uv: egui::epaint::WHITE_UV,
        color: colors[3],
    });
    mesh.vertices.push(egui::epaint::Vertex {
        pos: bottom_left,
        uv: egui::epaint::WHITE_UV,
        color: colors[2],
    });

    mesh.indices
        .extend_from_slice(&[i0, i0 + 1, i0 + 2, i0, i0 + 2, i0 + 3]);

    painter.add(egui::Shape::mesh(mesh));
}
