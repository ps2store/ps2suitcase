use eframe::egui::{
    self, Color32, FontData, FontDefinitions, FontFamily, FontId, Margin, Style, Vec2,
};

pub const DISPLAY_FONT_NAME: &str = "ps2_display";

#[derive(Clone)]
pub struct Palette {
    pub background: Color32,
    pub panel: Color32,
    pub header_top: Color32,
    pub header_bottom: Color32,
    pub footer_top: Color32,
    pub footer_bottom: Color32,
    pub neon_accent: Color32,
    pub soft_accent: Color32,
    pub separator: Color32,
    pub text_primary: Color32,
}

impl Default for Palette {
    fn default() -> Self {
        Self {
            background: Color32::from_rgb(6, 8, 20),
            panel: Color32::from_rgb(18, 38, 52),
            header_top: Color32::from_rgb(12, 16, 40),
            header_bottom: Color32::from_rgb(60, 40, 120),
            footer_top: Color32::from_rgb(16, 30, 52),
            footer_bottom: Color32::from_rgb(52, 52, 112),
            neon_accent: Color32::from_rgb(150, 92, 255),
            soft_accent: Color32::from_rgb(124, 148, 220),
            separator: Color32::from_rgb(88, 68, 168),
            text_primary: Color32::from_rgb(214, 220, 240),
        }
    }
}

pub fn install(ctx: &egui::Context, palette: &Palette) {
    install_fonts(ctx);
    apply_visuals(ctx, palette);
    ctx.style_mut(|style| apply_spacing(style));
}

pub fn display_font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(DISPLAY_FONT_NAME.into()))
}

fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        DISPLAY_FONT_NAME.to_owned(),
        FontData::from_static(include_bytes!("../../assets/fonts/Orbitron-Regular.ttf")).into(),
    );

    fonts
        .families
        .entry(FontFamily::Name(DISPLAY_FONT_NAME.into()))
        .or_default()
        .insert(0, DISPLAY_FONT_NAME.to_owned());
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, DISPLAY_FONT_NAME.to_owned());

    ctx.set_fonts(fonts);
}

fn apply_visuals(ctx: &egui::Context, palette: &Palette) {
    let mut visuals = egui::Visuals::dark();
    visuals.override_text_color = Some(palette.text_primary);
    visuals.widgets.noninteractive.bg_fill = palette.panel;
    visuals.widgets.noninteractive.fg_stroke.color = palette.text_primary;
    visuals.widgets.inactive.bg_fill = palette.panel;
    visuals.widgets.inactive.fg_stroke.color = palette.text_primary;
    visuals.widgets.hovered.bg_fill = palette.soft_accent.gamma_multiply(0.2);
    visuals.widgets.active.bg_fill = palette.soft_accent.gamma_multiply(0.3);
    visuals.widgets.open.bg_fill = palette.panel;
    visuals.extreme_bg_color = palette.background;
    visuals.faint_bg_color = palette.background;
    visuals.panel_fill = palette.background;

    ctx.set_visuals(visuals);
}

fn apply_spacing(style: &mut Style) {
    style.spacing.item_spacing = Vec2::new(12.0, 8.0);
    style.spacing.button_padding = Vec2::new(14.0, 8.0);
    style.spacing.window_margin = Margin::same(14);
    style.spacing.menu_margin = Margin::same(10);
    style.spacing.indent = 20.0;
}

pub fn draw_vertical_gradient(
    painter: &egui::Painter,
    rect: egui::Rect,
    top: Color32,
    bottom: Color32,
) {
    let mid_y = rect.center().y;
    let top_rect = egui::Rect::from_min_max(rect.min, egui::pos2(rect.max.x, mid_y));
    let bottom_rect = egui::Rect::from_min_max(egui::pos2(rect.min.x, mid_y), rect.max);
    painter.rect_filled(top_rect, 0.0, top);
    painter.rect_filled(bottom_rect, 0.0, bottom);
}

pub fn draw_separator(painter: &egui::Painter, rect: egui::Rect, color: Color32) {
    painter.rect_filled(rect, 0.0, color);
}
