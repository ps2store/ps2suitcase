#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod rendering;
mod tabs;
mod ui;

use crate::tabs::{FileTree, ICNViewer, IconSysViewer, Tab, TitleCfgViewer};
use crate::ui::{BottomBar, TabViewer};
use eframe::egui::{menu, Context, Frame, IconData, ViewportCommand, Widget};
use eframe::{egui, NativeOptions};
use egui_dock::{DockArea, DockState, NodeIndex, Style, SurfaceIndex, TabIndex};
use std::fs::{read_dir, File};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

fn main() -> eframe::Result<()> {
    let options = NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 720.0])
            .with_icon({
                let icon = include_bytes!("../ps2.png");
                let result = image::load_from_memory(icon).expect("Failed to load icon");

                let width = result.width();
                let height = result.height();

                IconData {
                    rgba: result.as_rgba8().unwrap().clone().into_raw(),
                    width,
                    height,
                }
            }),
        multisampling: 4,
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    eframe::run_native(
        "PSU Builder",
        options,
        Box::new(|_cc| Ok(Box::new(PSUBuilderApp::new()))),
    )
}

pub struct VirtualFile {
    pub name: String,
    pub file: Option<File>,
}

#[derive(Clone)]
pub enum AppEvent {
    OpenFile(Arc<Mutex<VirtualFile>>),
}

pub struct AppState {
    opened_folder: PathBuf,
    files: Vec<Arc<Mutex<VirtualFile>>>,
    events: Vec<AppEvent>,
}

impl AppState {
    pub fn open_file(&mut self, file: Arc<Mutex<VirtualFile>>) {
        self.events.push(AppEvent::OpenFile(file));
    }
}

impl AppState {
    pub fn new<T>(opened_folder: T) -> Self
    where
        T: Into<PathBuf>,
        T: Clone,
    {
        let files = read_dir(opened_folder.clone().into()).expect("Could not read directory");
        let files = files
            .into_iter()
            .flatten()
            .filter_map(|entry| {
                if entry.file_type().ok()?.is_file() {
                    Some(Arc::new(Mutex::new(VirtualFile {
                        name: entry.file_name().into_string().unwrap(),
                        file: File::open(entry.path()).ok(),
                    })))
                } else {
                    None
                }
            })
            .collect();

        Self {
            opened_folder: opened_folder.into(),
            files,
            events: vec![],
        }
    }
}

struct PSUBuilderApp {
    tree: DockState<Box<dyn Tab>>,
    state: Arc<Mutex<AppState>>,
    file_tree: FileTree,
}

impl PSUBuilderApp {
    fn new() -> Self {
        let state = Arc::new(Mutex::new(AppState::new(
            "/Users/simonhochrein/Downloads/APPS/APP_OPL",
        )));
        let tabs: Vec<Box<dyn Tab>> = vec![];
        let tree = DockState::new(tabs);

        Self {
            tree,
            state: state.clone(),
            file_tree: FileTree {
                state: state.clone(),
            },
        }
    }

    fn handle_events(&mut self) {
        let events = {
            let mut state = self.state.lock().unwrap();
            state.events.drain(..).collect::<Vec<_>>()
        };

        for event in events {
            match event {
                AppEvent::OpenFile(file) => {
                    self.handle_open(file.clone());
                }
            }
        }
    }

    fn handle_open(&mut self, file: Arc<Mutex<VirtualFile>>) {
        let mut found: Option<(SurfaceIndex, NodeIndex, TabIndex)> = None;
        for (i, ((surface, index), node)) in self.tree.iter_all_tabs().enumerate() {
            if node.get_title() == file.lock().unwrap().name {
                found = Some((surface, index, TabIndex::from(i)));
            }
        }
        if let Some(found) = found {
            self.tree.set_active_tab(found);
        } else {
            self.tab_for_file(file);
        }
    }

    fn tab_for_file(&mut self, file: Arc<Mutex<VirtualFile>>) {
        let name: PathBuf = file.lock().unwrap().name.clone().into();
        if let Some(extension) = name.extension() {
            let editor: Option<Box<dyn Tab>> = match extension.to_str().unwrap() {
                "icn" | "ico" => Some(Box::new(ICNViewer::new(self.state.clone(), file))),
                "sys" => Some(Box::new(IconSysViewer::new(self.state.clone(), file))),
                "cfg" => Some(Box::new(TitleCfgViewer::new(self.state.clone(), file))),
                _ => None,
            };

            if let Some(editor) = editor {
                self.tree.push_to_first_leaf(editor);
            }
        }
    }
}

impl eframe::App for PSUBuilderApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::left("side_panel").show(ctx, |ui| {
            self.file_tree.get_content(ui);
        });
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Add Files").clicked() {
                        println!("Add Files");
                    }
                    if ui.button("Quit").clicked() {
                        ui.ctx().send_viewport_cmd(ViewportCommand::Close);
                    }
                });
                ui.menu_button("Export", |ui| {});
                ui.menu_button("Help", |ui| {})
            });
        });

        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            BottomBar {}.ui(ui);
        });

        egui::CentralPanel::default()
            .frame(Frame::central_panel(&ctx.style()).inner_margin(0))
            .show(ctx, |ui| {
                DockArea::new(&mut self.tree)
                    .show_leaf_close_all_buttons(false)
                    .show_leaf_collapse_buttons(false)
                    .style(Style::from_egui(ctx.style().as_ref()))
                    .show_inside(ui, &mut TabViewer {});
            });

        self.handle_events();
    }
}
