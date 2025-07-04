#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod components;
mod data;
mod io;
mod rendering;
mod tabs;
mod wizards;

use crate::components::{bottom_bar, FileTree, MenuItemComponent, TabType, TabViewer, Toolbar};
use crate::data::state::{AppEvent, AppState};
use crate::data::virtual_file::VirtualFile;
use crate::io::calculate_size::calculate_size;
use crate::io::export_psu::export_psu;
use crate::io::read_folder::read_folder;
use crate::tabs::{ICNViewer, IconSysViewer, TitleCfgViewer};
use crate::wizards::Wizards;
use eframe::egui::{
    menu, Context, Frame, IconData, KeyboardShortcut, Modifiers, ViewportCommand,
};
use eframe::{egui, NativeOptions, Storage};
use egui_dock::{DockArea, DockState, NodeIndex, Style, SurfaceIndex, TabIndex};
use std::path::PathBuf;

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

struct PSUBuilderApp {
    tree: DockState<Box<TabType>>,
    state: AppState,
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

    const OPEN_FOLDER_KEYBOARD_SHORTCUT: KeyboardShortcut =
        KeyboardShortcut::new(Self::CTRL_OR_CMD, egui::Key::O);
    const EXPORT_KEYBOARD_SHORTCUT: KeyboardShortcut =
        KeyboardShortcut::new(Self::CTRL_OR_CMD_SHIFT, egui::Key::S);
    const ADD_FILE_KEYBOARD_SHORTCUT: KeyboardShortcut =
        KeyboardShortcut::new(Self::CTRL_OR_CMD, egui::Key::N);
    const SAVE_KEYBOARD_SHORTCUT: KeyboardShortcut =
        KeyboardShortcut::new(Self::CTRL_OR_CMD, egui::Key::S);
    const CREATE_ICN_KEYBOARD_SHORTCUT: KeyboardShortcut =
        KeyboardShortcut::new(Self::CTRL_OR_CMD, egui::Key::I);

    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let state = AppState::new();
        let tabs: Vec<Box<TabType>> = vec![];
        let tree = DockState::new(tabs);
        let file_tree = FileTree::new();

        let mut slf = Self {
            tree,
            state,
            file_tree,
            first_render: true,
            saving: false,
            show_create_icn: false,
        };

        slf.try_open_saved_folder(cc.storage);
        slf
    }

    fn try_open_saved_folder(&mut self, storage: Option<&dyn Storage>) -> Option<()> {
        let folder = eframe::get_value::<WorkspaceSave>(storage?, eframe::APP_KEY)?.opened_folder?;

        if folder.exists() {
            self.do_open_folder(folder).ok()
        } else {
            None
        }
    }

    fn handle_events(&mut self, ctx: &Context) {
        let events = { self.state.events.drain(..).collect::<Vec<_>>() };

        for event in events {
            match event {
                AppEvent::OpenFile(file) => {
                    self.handle_open(file);
                }
                AppEvent::SetTitle(title) => {
                    ctx.send_viewport_cmd(ViewportCommand::Title(title));
                }
                AppEvent::AddFiles => {
                    self.add_files(ctx);
                }
                AppEvent::OpenFolder => {
                    self.open_folder();
                }
                AppEvent::ExportPSU => {
                    export_psu(&mut self.state).expect("Failed to export PSU");
                }
                AppEvent::SaveFile => {
                    self.save_file();
                }
            }
        }
    }

    fn handle_open(&mut self, file: VirtualFile) {
        let mut found: Option<(SurfaceIndex, NodeIndex, TabIndex)> = None;
        for (i, ((surface, index), node)) in self.tree.iter_all_tabs().enumerate() {
            if node.get_title() == file.name {
                found = Some((surface, index, TabIndex::from(i)));
            }
        }
        if let Some(found) = found {
            self.tree.set_active_tab(found);
        } else {
            self.tab_for_file(file);
        }
    }

    fn tab_for_file(&mut self, file: VirtualFile) {
        let name: PathBuf = file.name.clone().into();
        if let Some(extension) = name.extension() {
            let editor: Option<TabType> = match extension.to_ascii_lowercase().to_str().unwrap() {
                "icn" | "ico" => Some(TabType::ICNViewer(ICNViewer::new(file))),
                "sys" => Some(TabType::IconSysViewer(IconSysViewer::new(file))),
                "cfg" | "cnf" | "dat" | "txt" => Some(TabType::TitleCfgViewer(TitleCfgViewer::new(file))),
                _ => None,
            };

            if let Some(editor) = editor {
                self.tree.push_to_focused_leaf(Box::new(editor));
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

    fn add_files(&mut self, _ctx: &Context) {
        // if let Some(files) = ctx.open_files() {
        //     let opened_folder = self
        //         .state
        //         .opened_folder
        //         .clone()
        //         .expect("Should only be called after folder is opened");

        // for file in files {
        // let name = file.file_name().unwrap();
        // std::fs::copy(opened_folder.join(name));
        // opened_folder
        // }
        // println!("{:#?}", files);
        // }
    }

    fn open_folder(&mut self) {
        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
            self.do_open_folder(folder).expect("Failed to open folder");
        }
    }

    fn do_open_folder(&mut self, folder: PathBuf) -> std::io::Result<()> {
        self.state.opened_folder = Some(folder.clone());
        self.state.set_title(
            folder
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
        );
        self.state.files = read_folder(folder)?;
        self.state.calculated_size = calculate_size(&self.state.files)?;

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
        let is_folder_open = self.state.opened_folder.is_some();

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
                            self.state.export_psu();
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
            ui.add(Toolbar {});
            // menu::bar(components, |components| {
            //     components.set_min_size(Vec2::new(24.0, 24.0));
            //     if components.icon_button(include_image!("../assets/icons/folder.svg")).on_hover_text("Open Folder").clicked() {
            //         self.open_folder();
            //     }
            //     components.add_enabled_ui(is_folder_open, |components| {
            //         if components.icon_button(include_image!("../assets/icons/file-plus.svg")).on_hover_text("Add Files").clicked() {
            //             self.add_files(ctx);
            //         }
            //     });
            //     components.separator();
            //     components.add_enabled_ui(is_folder_open, |components| {
            //         if components.icon_button(include_image!("../assets/icons/cube-plus.svg")).on_hover_text("Create ICN").clicked() {
            //             self.show_create_icn = true;
            //         }
            //     });
            // });
            ui.add_space(4.0);
        });
        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            bottom_bar(ui, &mut self.state);
        });

        if self.state.opened_folder.is_some() {
            egui::SidePanel::left("side_panel").show(ctx, |ui| {
                self.file_tree.show(ui, &mut self.state);
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
                        .show_inside(
                            ui,
                            &mut TabViewer {
                                app: &mut self.state,
                            },
                        );
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
            self.state.open_folder();
        } else if ctx.input_mut(|i| i.consume_shortcut(&Self::EXPORT_KEYBOARD_SHORTCUT)) {
            self.saving = true;
            self.state.export_psu();
        } else if ctx.input_mut(|i| i.consume_shortcut(&Self::SAVE_KEYBOARD_SHORTCUT)) {
            self.state.save_file();
        } else if ctx.input_mut(|i| i.consume_shortcut(&Self::CREATE_ICN_KEYBOARD_SHORTCUT)) {
            self.show_create_icn = true;
        }

        ctx.create_icn_wizard(&mut self.show_create_icn);
        self.handle_events(ctx);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {}

    fn save(&mut self, storage: &mut dyn Storage) {
        eframe::set_value(
            storage,
            eframe::APP_KEY,
            &WorkspaceSave {
                opened_folder: self.state.opened_folder.clone(),
            },
        );
    }
}
