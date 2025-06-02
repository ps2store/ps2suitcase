#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod rendering;
mod tabs;
mod ui;
mod wizards;

use std::fs::File;
use std::io::{Read, Write};
use std::ops::Sub;
use crate::tabs::{ICNViewer, IconSysViewer, Tab, TitleCfgViewer};
use crate::ui::{BottomBar, CustomButtons, Dialogs, FileTree, MenuItemComponent, TabViewer};
use crate::wizards::Wizards;
use eframe::egui::{include_image, menu, Context, Frame, IconData, KeyboardShortcut, Modifiers, Rect, UiBuilder, Vec2, ViewportCommand, Widget};
use eframe::{egui, NativeOptions, Storage};
use egui_dock::{DockArea, DockState, NodeIndex, Style, SurfaceIndex, TabIndex};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use ps2_filetypes::{BinWriter, PSUEntry, PSUEntryKind, PSUWriter, DIR_ID, FILE_ID, PSU};
use ps2_filetypes::chrono::{DateTime, Utc};

fn main() -> eframe::Result<()> {
    let options = NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 720.0])
            .with_icon({
                let icon = include_bytes!("../assets/ps2.png");
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
        depth_buffer: 1,
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    eframe::run_native(
        "PSU Builder",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(PSUBuilderApp::new(cc)))
        }),
    )
}

pub struct VirtualFile {
    pub name: String,
    pub file_path: PathBuf,
    pub size: u64,
}

#[derive(Clone)]
pub enum AppEvent {
    OpenFile(Arc<Mutex<VirtualFile>>),
    SetTitle(String),
    AddFiles,
}

pub struct AppState {
    opened_folder: Option<PathBuf>,
    files: Vec<Arc<Mutex<VirtualFile>>>,
    events: Vec<AppEvent>,
    pub calculated_size: u64,
}

impl AppState {
    pub fn open_file(&mut self, file: Arc<Mutex<VirtualFile>>) {
        self.events.push(AppEvent::OpenFile(file));
    }
    pub fn set_title(&mut self, title: String) {
        self.events.push(AppEvent::SetTitle(title));
    }
    pub fn add_files(&mut self) {
        self.events.push(AppEvent::AddFiles);
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        Self {
            opened_folder: None,
            files: vec![],
            events: vec![],
            calculated_size: 0,
        }
    }
}

struct PSUBuilderApp {
    tree: DockState<Box<dyn Tab>>,
    state: Arc<Mutex<AppState>>,
    file_tree: FileTree,
    first_render: bool,
    saving: bool,
    show_create_icn: bool,
}

#[derive(Default, serde::Deserialize, serde::Serialize)]
struct WorkspaceSave {
    opened_folder: Option<PathBuf>,
}

impl PSUBuilderApp {
    const CTRL_OR_CMD: Modifiers = if cfg!(target_os = "macos") {
        Modifiers::MAC_CMD
    } else {
        Modifiers::CTRL
    };
    const CTRL_OR_CMD_SHIFT: Modifiers = if cfg!(target_os = "macos") {
       Modifiers {
           alt: false,
           ctrl: false,
           shift: true,
           mac_cmd: true,
           command: false,
       }
    } else {
        Modifiers {
            alt: false,
            ctrl: true,
            shift: true,
            mac_cmd: false,
            command: false,
        }
    };

    const OPEN_FOLDER_KEYBOARD_SHORTCUT: KeyboardShortcut = KeyboardShortcut::new(Self::CTRL_OR_CMD, egui::Key::O);
    const EXPORT_KEYBOARD_SHORTCUT: KeyboardShortcut = KeyboardShortcut::new(Self::CTRL_OR_CMD_SHIFT, egui::Key::S);
    const ADD_FILE_KEYBOARD_SHORTCUT: KeyboardShortcut = KeyboardShortcut::new(Self::CTRL_OR_CMD, egui::Key::N);
    const SAVE_KEYBOARD_SHORTCUT: KeyboardShortcut = KeyboardShortcut::new(Self::CTRL_OR_CMD, egui::Key::S);
    const CREATE_ICN_KEYBOARD_SHORTCUT: KeyboardShortcut = KeyboardShortcut::new(Self::CTRL_OR_CMD, egui::Key::I);

    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let state = Arc::new(Mutex::new(AppState::new()));
        let tabs: Vec<Box<dyn Tab>> = vec![];
        let tree = DockState::new(tabs);
        let file_tree = FileTree::new(state.clone());

        let mut slf = Self {
            tree,
            state,
            file_tree,
            first_render: true,
            saving: false,
            show_create_icn: false,
        };

        if let Some(storage) = cc.storage {
            if let Some(save) = eframe::get_value::<WorkspaceSave>(storage, eframe::APP_KEY) {
                if let Some(folder) = save.opened_folder {
                    if folder.exists() {
                        slf.do_open_folder(folder);
                    }
                }
            }
        }

       slf
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
                AppEvent::AddFiles => {
                    self.add_files(ctx);
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
                "cfg" | "txt" => Some(Box::new(TitleCfgViewer::new(self.state.clone(), file))),
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

    fn add_files(&mut self, ctx: &Context) {
        if let Some(files) = ctx.open_files() {
            let opened_folder = self.state.lock().unwrap().opened_folder.clone().expect("Should only be called after folder is opened");

            for file in files {
                let name = file.file_name().unwrap();
                // std::fs::copy(opened_folder.join(name));
                // opened_folder
            }
            // println!("{:#?}", files);
        }
    }

    fn open_folder(&mut self) {
        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
            self.do_open_folder(folder);
        }
    }

    fn do_open_folder(&mut self, folder: PathBuf) {
        self.state.lock().unwrap().opened_folder = Some(folder.clone());
        self.state.lock().unwrap().set_title(
            folder
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
        );
        self.file_tree.open(folder);
    }

    fn export_psu(&mut self) -> std::io::Result<()> {
        let folder_name = self.state
            .lock()
            .unwrap()
            .opened_folder
            .clone()
            .unwrap()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();

        let target_filename = folder_name.to_owned()
            + ".psu";

        if let Some(filename) = rfd::FileDialog::new()
            .set_file_name(target_filename)
            .save_file()
        {
            let mut psu = PSU::default();

            let root = PSUEntry {
                id: DIR_ID,
                size:  self.state.lock().unwrap().files.len() as u32 + 2,
                created: Utc::now().naive_utc(),
                sector: 0,
                modified: Utc::now().naive_utc(),
                name: folder_name,
                kind: PSUEntryKind::Directory,
                contents: None,
            };
            let cur = PSUEntry {
                id: FILE_ID,
                size: 0,
                created: Utc::now().naive_utc(),
                sector: 0,
                modified: Utc::now().naive_utc(),
                name: ".".to_string(),
                kind: PSUEntryKind::File,
                contents: Some(vec![]),
            };
            let parent = PSUEntry {
                id: FILE_ID,
                size: 0,
                created: Utc::now().naive_utc(),
                sector: 0,
                modified: Utc::now().naive_utc(),
                name: "..".to_string(),
                kind: PSUEntryKind::File,
                contents: Some(vec![]),
            };

            psu.entries.push(root);
            psu.entries.push(cur);
            psu.entries.push(parent);

            for file in self.state.lock().unwrap().files.iter() {
                let file_path = file.lock().unwrap().file_path.clone();
                let name = file.lock().unwrap().name.clone();
                let mut file = File::open(file_path)?;
                let metadata = file.metadata()?;
                let mut contents = vec![0u8; metadata.len() as usize];
                let size = file.read(&mut contents)? as u32;

                let created_at: DateTime<Utc> = metadata.modified()?.into();
                let modified_at: DateTime<Utc> = metadata.modified()?.into();

                psu.entries.push(PSUEntry  {
                    id: FILE_ID,
                    size,
                    sector: 0,
                    contents: Some(contents),
                    name,
                    created: created_at.naive_local(),
                    modified: modified_at.naive_local(),
                    kind: PSUEntryKind::File,
                });
            }
            let data = PSUWriter::new(psu).write()?;
            File::create(&filename)?.write_all(&data)?;
        }

        Ok(())
    }

    // fn has_unsaved_files(&self) -> bool {
    //     for (_, tab) in self.tree.iter_all_tabs() {
    //         if tab.get_modified() {
    //             return true;
    //         }
    //     }
    //
    //     false
    // }
}

impl eframe::App for PSUBuilderApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        let is_folder_open = self.state.lock().unwrap().opened_folder.is_some();

        if self.first_render {
            self.first_render = false;
        }

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui
                        .menu_item_shortcut("Open Folder", &Self::OPEN_FOLDER_KEYBOARD_SHORTCUT)
                        .clicked()
                    {
                        self.open_folder();
                    }
                    ui.add_enabled_ui(is_folder_open, |ui| {
                        if ui
                            .menu_item_shortcut("Add Files", &Self::ADD_FILE_KEYBOARD_SHORTCUT)
                            .clicked()
                        {
                            self.add_files(ctx);
                        }
                        if ui
                            .menu_item_shortcut("Save File", &Self::SAVE_KEYBOARD_SHORTCUT)
                            .clicked()
                        {
                            self.save_file();
                        }
                        ui.separator();
                        if ui
                            .menu_item_shortcut("Create ICN", &Self::CREATE_ICN_KEYBOARD_SHORTCUT)
                            .clicked()
                        {
                            self.show_create_icn = true;
                        }
                    });
                });
                ui.menu_button("Export", |ui| {
                    ui.add_enabled_ui(is_folder_open, |ui| {
                        if ui
                            .menu_item_shortcut("Export PSU", &Self::EXPORT_KEYBOARD_SHORTCUT)
                            .clicked()
                        {
                            self.export_psu().expect("Failed to export PSU");
                        }
                    });
                });
                ui.menu_button("Help", |ui| {
                    ui.menu_item_link("GitHub", "https://github.com/simonhochrein/ps2-rust")
                })
            });
        });
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.add_space(4.0);
            menu::bar(ui, |ui| {
                ui.set_min_size(Vec2::new(24.0, 24.0));
                if ui.icon_button(include_image!("../assets/icons/folder.svg")).on_hover_text("Open Folder").clicked() {
                    self.open_folder();
                }
                ui.add_enabled_ui(is_folder_open, |ui| {
                    if ui.icon_button(include_image!("../assets/icons/file-plus.svg")).on_hover_text("Add Files").clicked() {
                        self.add_files(ctx);
                    }
                });
                ui.separator();
                ui.add_enabled_ui(is_folder_open, |ui| {
                    if ui.icon_button(include_image!("../assets/icons/cube-plus.svg")).on_hover_text("Create ICN").clicked() {
                        self.show_create_icn = true;
                    }
                });
            });
            ui.add_space(4.0);
        });
        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.add(BottomBar {});
        });

        if self.state.lock().unwrap().opened_folder.is_some() {
            egui::SidePanel::left("side_panel").show(ctx, |ui| {
                self.file_tree.ui(ui);
            });
        }


        egui::CentralPanel::default()
            .frame(Frame::central_panel(&ctx.style()).inner_margin(0))
            .show(ctx, |ui| {
                if self.tree.iter_all_tabs().count() > 0 {
                    DockArea::new(&mut self.tree)
                        .show_leaf_close_all_buttons(false)
                        .show_leaf_collapse_buttons(false)
                        .style(Style::from_egui(ctx.style().as_ref()))
                        .show_inside(ui, &mut TabViewer {});
                } else {
                    ui.centered_and_justified(|ui| {
                        if !is_folder_open {
                            ui.heading(format!(
                                "Open a folder to get started ({})",
                                &ctx.format_shortcut(&Self::OPEN_FOLDER_KEYBOARD_SHORTCUT)
                            ));
                        } else {
                            ui.heading("No open editors");
                        }
                    });
                }
            });

        if ctx.input_mut(|i| i.consume_shortcut(&Self::OPEN_FOLDER_KEYBOARD_SHORTCUT)) {
            self.open_folder();
        } else if ctx.input_mut(|i| i.consume_shortcut(&Self::EXPORT_KEYBOARD_SHORTCUT)) {
            self.saving = true;
            self.export_psu().expect("Failed to export psu");
        } else if ctx.input_mut(|i| i.consume_shortcut(&Self::SAVE_KEYBOARD_SHORTCUT)) {
            self.save_file();
        } else if ctx.input_mut(|i| i.consume_shortcut(&Self::CREATE_ICN_KEYBOARD_SHORTCUT)) {
            self.show_create_icn = true;
        }

        ctx.create_icn_wizard(&mut self.show_create_icn);
        self.handle_events(ctx);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {}

    fn save(&mut self, storage: &mut dyn Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &WorkspaceSave {
            opened_folder: self.state.lock().unwrap().opened_folder.clone(),
        });
    }
}

