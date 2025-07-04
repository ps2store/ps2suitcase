use eframe::egui::{Button, Image, Response, Ui, WidgetText};

pub trait CustomButtons {
    fn icon_button<'a>(&mut self, image: impl Into<Image<'a>>) -> Response;
    fn icon_text_button<'a>(&mut self, image: impl Into<Image<'a>>, text: impl Into<WidgetText>) -> Response;
}

impl CustomButtons for Ui {
    #[inline]
    fn icon_button<'a>(&mut self, image: impl Into<Image<'a>>) -> Response {
        self.add(Button::image(image).image_tint_follows_text_color(true))
    }

    #[inline]
    fn icon_text_button<'a>(&mut self, image: impl Into<Image<'a>>, text: impl Into<WidgetText>) -> Response {
        self.add(Button::image_and_text(image, text).image_tint_follows_text_color(true))
    }
}