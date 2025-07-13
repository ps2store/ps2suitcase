#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod components;
mod data;
mod io;
mod rendering;
mod tabs;
mod wizards;

use crate::components::dialogs::Dialogs;
use crate::components::greeting::greeting;
use crate::wizards::create_icn::create_icn_wizard;
use crate::{
    components::bottom_bar::bottom_bar,
    components::file_tree::FileTree,
    components::menu_bar::{handle_accelerators, menu_bar},
    components::tab_viewer::{TabType, TabViewer},
    components::toolbar::toolbar,
    data::state::{AppEvent, AppState},
    data::virtual_file::VirtualFile,
    io::export_psu::export_psu,
    io::read_folder::read_folder,
    tabs::{ICNViewer, IconSysViewer, TitleCfgViewer},
};
use eframe::egui::{Context, Frame, IconData, ViewportCommand};
use eframe::{egui, NativeOptions, Storage};
use egui_dock::{DockArea, DockState, NodeIndex, Style, SurfaceIndex, TabIndex};
use std::path::PathBuf;
use crate::io::file_watcher::FileWatcher;

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
        "PS2Suitcase",
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
    show_create_icn: bool,
    file_watcher: FileWatcher,
}

#[derive(Default, serde::Deserialize, serde::Serialize)]
struct WorkspaceSave {
    opened_folder: Option<PathBuf>,
}

impl PSUBuilderApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let state = AppState::new();
        let tabs: Vec<Box<TabType>> = vec![];
        let tree = DockState::new(tabs);
        let file_tree = FileTree::new();

        let mut slf = Self {
            tree,
            state,
            file_tree,
            show_create_icn: false,
            file_watcher: FileWatcher::new(),
        };

        slf.try_open_saved_folder(cc.storage);
        slf
    }

    fn try_open_saved_folder(&mut self, storage: Option<&dyn Storage>) -> Option<()> {
        let config = eframe::get_value::<WorkspaceSave>(storage?, eframe::APP_KEY)?;
        let folder = config.opened_folder?;

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
                    self.add_files(ctx).expect("Failed to add files");
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
                AppEvent::OpenSave => {
                    if let Some(_) = rfd::FileDialog::new()
                        .add_filter("PS2 Save Files", &["psu"])
                        .pick_file()
                    {}
                }
                AppEvent::CreateICN => {
                    self.show_create_icn = true;
                }
            }
        }
    }

    fn handle_fs_events(&mut self) {
        while let Ok(_event) = self.file_watcher.event_rx.try_recv() {
            self.state.files = read_folder(self.state.opened_folder.clone().unwrap()).unwrap();
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
                "icn" | "ico" => Some(TabType::ICNViewer(ICNViewer::new(&file))),
                "sys" => Some(TabType::IconSysViewer(IconSysViewer::new(&file))),
                "cfg" | "cnf" | "dat" | "txt" => {
                    Some(TabType::TitleCfgViewer(TitleCfgViewer::new(&file)))
                }
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

    fn add_files(&mut self, ctx: &Context) -> std::io::Result<()> {
        if let Some(files) = ctx.open_files() {
            let opened_folder = self
                .state
                .opened_folder
                .clone()
                .expect("Should only be called after folder is opened");

            for file in files {
                let name = file.file_name().unwrap();
                let new_path = opened_folder.join(name);
                std::fs::copy(&file, &new_path)?;

                self.state.files.add_file(new_path)?;
            }
        }
        Ok(())
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
        self.file_watcher.change_path(&folder);
        self.state.files = read_folder(folder)?;

        Ok(())
    }
}

impl eframe::App for PSUBuilderApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            menu_bar(ui, &mut self.state);
        });
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.add_space(4.0);
            toolbar(ui, &mut self.state);
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
                    greeting(ui, &mut self.state);
                }
            });

        handle_accelerators(ctx, &mut self.state);

        create_icn_wizard(ctx, &mut self.show_create_icn);
        self.handle_events(ctx);
        self.handle_fs_events();
    }

    fn save(&mut self, storage: &mut dyn Storage) {
        eframe::set_value(
            storage,
            eframe::APP_KEY,
            &WorkspaceSave {
                opened_folder: self.state.opened_folder.clone(),
            },
        );
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
    }
}
