pub trait Tab {
    const TOOLBAR_HEIGHT: f32 = 25.0;
    const TOOLBAR_LEFT_MARGIN: f32 = 10.0;

    fn get_id(&self) -> &str;
    fn get_title(&self) -> String;
    fn get_modified(&self) -> bool;

    fn save(&mut self);
}
