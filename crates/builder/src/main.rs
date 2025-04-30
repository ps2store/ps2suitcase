#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod rendering;
mod tabs;
mod ui;
mod utils;

use crate::tabs::{FileTree, FileTreeComponent, ICNViewer, IconSysViewer, Tab, TitleCfgViewer};
use crate::ui::{BottomBar, MenuItemComponent, TabViewer};
use crate::utils::{shortcut, Shortcut};
use eframe::egui::{
    menu, Area, Button, Color32, Context, CornerRadius, Frame, IconData, Id, KeyboardShortcut,
    Modifiers, OpenUrl, Pos2, Sense, ViewportCommand, Widget,
};
use eframe::{egui, NativeOptions};
use egui_dock::{DockArea, DockState, NodeIndex, Style, SurfaceIndex, TabIndex};
use lazy_static::lazy_static;
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
    pub file_path: PathBuf,
}

#[derive(Clone)]
pub enum AppEvent {
    OpenFile(Arc<Mutex<VirtualFile>>),
    SetTitle(String),
}

pub struct AppState {
    opened_folder: Option<PathBuf>,
    files: Vec<Arc<Mutex<VirtualFile>>>,
    events: Vec<AppEvent>,
}

impl AppState {
    pub fn open_file(&mut self, file: Arc<Mutex<VirtualFile>>) {
        self.events.push(AppEvent::OpenFile(file));
    }
    pub fn set_title(&mut self, title: String) {
        self.events.push(AppEvent::SetTitle(title));
    }
}

impl AppState {
    pub fn new() -> Self
    {
        Self {
            opened_folder: None,
            files: vec![],
            events: vec![],
        }
    }
}

struct PSUBuilderApp {
    tree: DockState<Box<dyn Tab>>,
    state: Arc<Mutex<AppState>>,
    file_tree: FileTree,
    first_render: bool,
    saving: bool,
    confirm_close: bool,
}

impl PSUBuilderApp {
    fn new() -> Self {
        let state = Arc::new(Mutex::new(AppState::new()));
        let tabs: Vec<Box<dyn Tab>> = vec![];
        let tree = DockState::new(tabs);

        Self {
            tree,
            state: state.clone(),
            file_tree: FileTree {
                state: state.clone(),
            },
            first_render: true,
            saving: false,
            confirm_close: false,
        }
    }

    fn handle_events(&mut self, ctx: &Context) {
        let events = {
            let mut state = self.state.lock().unwrap();
            state.events.drain(..).collect::<Vec<_>>()
        };

        for event in events {
            match event {
                AppEvent::OpenFile(file) => {
                    self.handle_open(file.clone());
                }
                AppEvent::SetTitle(title) => {
                    ctx.send_viewport_cmd(ViewportCommand::Title(title));
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
                self.tree.push_to_focused_leaf(editor);
                if let Some(position) = self.tree.focused_leaf() {
                    self.tree.set_focused_node_and_surface(position);
                } else {
                    self.tree
                        .set_focused_node_and_surface((SurfaceIndex::main(), NodeIndex::root()));
                }
            }
        }
    }

    fn save_file(&mut self) {
        if let Some((_, tab)) = self.tree.find_active_focused() {
            tab.save();
        }
    }

    fn open_folder(&mut self) {
       if let Some(folder) = rfd::FileDialog::new().pick_folder() {
           self.state.lock().unwrap().opened_folder = Some(folder.clone());
           self.file_tree.open(folder);
       }
    }

    fn has_unsaved_files(&self) -> bool {
        for (_, tab) in self.tree.iter_all_tabs() {
            if tab.get_modified() {
                return true;
            }
        }

        false
    }
}

impl eframe::App for PSUBuilderApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        let is_folder_open = self.state.lock().unwrap().opened_folder.is_some();
        const CTRL_OR_CMD: Modifiers = if cfg!(target_os = "macos") {
            Modifiers::MAC_CMD
        } else {
            Modifiers::CTRL
        };

        let open_folder_keyboard_shortcut = KeyboardShortcut::new(CTRL_OR_CMD, egui::Key::O);
        let export_keyboard_shortcut =
            KeyboardShortcut::new(CTRL_OR_CMD | Modifiers::SHIFT, egui::Key::S);
        let add_file_keyboard_shortcut = KeyboardShortcut::new(CTRL_OR_CMD, egui::Key::N);
        let save_keyboard_shortcut = KeyboardShortcut::new(CTRL_OR_CMD, egui::Key::S);
        let create_icn_keyboard_shortcut = KeyboardShortcut::new(CTRL_OR_CMD, egui::Key::I);

        if self.first_render {
            self.first_render = false;
        }

        if self.state.lock().unwrap().opened_folder.is_some() {
            egui::SidePanel::left("side_panel").show(ctx, |ui| {
                ui.file_tree(self.state.clone());
            });
        }
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.menu_item_shortcut("Open Folder", &open_folder_keyboard_shortcut).clicked() {
                        self.open_folder();
                    }
                    ui.add_enabled_ui(is_folder_open, |ui| {
                        if ui
                            .menu_item_shortcut("Add Files", &add_file_keyboard_shortcut)
                            .clicked()
                        {
                            println!("Add Files");
                        }
                        if ui
                            .menu_item_shortcut("Save File", &save_keyboard_shortcut)
                            .clicked()
                        {
                            self.save_file();
                        }
                        ui.separator();
                        if ui
                            .menu_item_shortcut("Create ICN", &create_icn_keyboard_shortcut)
                            .clicked()
                        {
                            println!("Create ICN");
                        }
                    });
                });
                ui.menu_button("Export", |ui| {
                    ui.add_enabled_ui(is_folder_open, |ui| {
                        if ui
                            .menu_item_shortcut("Export PSU", &export_keyboard_shortcut)
                            .clicked()
                        {
                            println!("Export PSU");
                        }
                    });
                });
                ui.menu_button("Help", |ui| {
                    ui.menu_item_link("GitHub", "https://github.com/simonhochrein/ps2-rust")
                })
            });
        });

        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.add(BottomBar {});
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

        if ctx.input_mut(|i| i.consume_shortcut(&open_folder_keyboard_shortcut)) {
            self.open_folder();
        } else if ctx.input_mut(|i| i.consume_shortcut(&export_keyboard_shortcut)) {
            self.saving = true;
        } else if ctx.input_mut(|i| i.consume_shortcut(&save_keyboard_shortcut)) {
            self.save_file();
        }

        if self.saving {
            Area::new(Id::new("export_modal"))
                .fixed_pos(Pos2::ZERO)
                .show(ctx, |ui| {
                    let screen_rect = ui.ctx().input(|i| i.screen_rect);
                    let area_response = ui.allocate_response(screen_rect.size(), Sense::click());

                    if area_response.clicked() {
                        self.saving = false;
                    }

                    ui.painter().rect_filled(
                        screen_rect,
                        CornerRadius::ZERO,
                        Color32::from_rgba_premultiplied(0, 0, 0, 100),
                    );
                });
        }

        // if ctx.input(|i| i.viewport().close_requested()) {
        //     if !self.confirm_close && self.has_unsaved_files() {
        //         ctx.send_viewport_cmd(ViewportCommand::CancelClose);
        //         self.confirm_close = true;
        //     }
        // }
        //
        // if self.confirm_close {
        //     let screen_rect = ctx.input(|i| i.screen_rect);
        //     egui::Window::new("Do you want to quit?")
        //         .collapsible(false)
        //         .resizable(false)
        //         .fixed_pos(screen_rect.center())
        //         .show(ctx, |ui| {
        //             ui.horizontal(|ui| {
        //                 if ui.button("No").clicked() {
        //                     self.confirm_close = false;
        //                 }
        //
        //                 if ui.button("Yes").clicked() {
        //                     ui.ctx().send_viewport_cmd(ViewportCommand::Close);
        //                 }
        //             });
        //         });
        // }

        self.handle_events(ctx);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {}
}
