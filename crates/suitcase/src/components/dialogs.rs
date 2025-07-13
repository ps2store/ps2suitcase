use eframe::egui::Context;
use std::collections::HashMap;
use std::path::PathBuf;

pub trait Dialogs {
    fn save_as(&self, filename: impl Into<String>) -> Option<PathBuf>;
    fn open_file(&self, filters: Filters) -> Option<PathBuf>;
    fn open_file_filter(&self, extensions: &[impl ToString]) -> Option<PathBuf>;
    fn open_files(&self) -> Option<Vec<PathBuf>>;
}

pub struct Filters {
    filters: HashMap<String, Vec<String>>,
}

pub trait FilterExtensions<T, const N: usize> {
    fn as_array(&self) -> &[T; N];
}

impl<T, const N: usize> FilterExtensions<T, N> for [T; N] {
    fn as_array(&self) -> &[T; N] {
        self
    }
}

impl<T, const N: usize> FilterExtensions<T, N> for &[T; N] {
    fn as_array(&self) -> &[T; N] {
        self
    }
}

impl Filters {
    pub fn new() -> Self {
        Filters {
            filters: HashMap::new(),
        }
    }

    pub fn add_filter<A, T, const N: usize>(mut self, name: impl ToString, extensions: A) -> Self
    where
        A: FilterExtensions<T, N>,
        T: ToString,
    {
        self.filters.insert(
            name.to_string(),
            extensions
                .as_array()
                .iter()
                .map(|ext| ext.to_string())
                .collect::<Vec<String>>(),
        );
        self
    }

    fn get_filters(&self) -> &HashMap<String, Vec<String>> {
        &self.filters
    }
}

impl Dialogs for &Context {
    fn save_as(&self, filename: impl Into<String>) -> Option<PathBuf> {
        rfd::FileDialog::new().set_file_name(filename).save_file()
    }

    fn open_file(&self, filters: Filters) -> Option<PathBuf> {
        let mut dialog = rfd::FileDialog::default();
        for (name, exts) in filters.get_filters() {
            dialog = dialog.add_filter(name, &*exts);
        }
        dialog.pick_file()
    }

    fn open_file_filter(&self, extensions: &[impl ToString]) -> Option<PathBuf> {
        rfd::FileDialog::new()
            .add_filter("Extension", extensions)
            .pick_file()
    }

    fn open_files(&self) -> Option<Vec<PathBuf>> {
        rfd::FileDialog::new().pick_files()
    }
}
