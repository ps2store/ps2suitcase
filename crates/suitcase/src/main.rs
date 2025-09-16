#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod components;
mod data;
mod io;
mod rendering;
mod tabs;
mod wizards;

use crate::io::validate::validate;
use crate::{
    components::bottom_bar::bottom_bar,
    components::dialogs::Dialogs,
    components::file_tree::FileTree,
    components::greeting::greeting,
    components::menu_bar::{handle_accelerators, menu_bar},
    components::tab_viewer::{TabType, TabViewer},
    components::toolbar::toolbar,
    data::state::{AppEvent, AppState},
    data::virtual_file::VirtualFile,
    io::export_psu::export_psu,
    io::file_watcher::FileWatcher,
    io::read_folder::read_folder,
    tabs::{ICNViewer, IconSysViewer, PsuTomlViewer, TitleCfgViewer},
    wizards::create_icn::create_icn_wizard,
};
use eframe::egui::{Context, Frame, IconData, Margin, ViewportCommand};
use eframe::{egui, NativeOptions, Storage};
use egui_dock::{AllowedSplits, DockArea, DockState, NodeIndex, SurfaceIndex, TabIndex};
use ps2_filetypes::TitleCfg;
use std::path::PathBuf;
use std::process::Command;

fn main() -> eframe::Result<()> {
    let options = NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1920.0, 1080.0])
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
    show_settings: bool,
    file_watcher: FileWatcher,
}

#[derive(Default, serde::Deserialize, serde::Serialize)]
struct WorkspaceSave {
    opened_folder: Option<PathBuf>,
}

impl PSUBuilderApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut state = AppState::new();
        state.pcsx2_path = cc
            .storage
            .and_then(|s| eframe::get_value::<String>(s, "pcsx2_path"))
            .unwrap_or_default();

        let mut slf = Self {
            tree: DockState::new(Vec::new()),
            state,
            file_tree: FileTree::new(),
            show_create_icn: false,
            show_settings: false,
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
                AppEvent::CreateTitleCfg => {
                    self.create_title_cfg();
                }
                AppEvent::OpenSettings => {
                    self.show_settings = true;
                }
                AppEvent::StartPCSX2 => {
                    let pcsx = if cfg!(target_os = "macos") {
                        self.state.pcsx2_path.clone() + "/Contents/MacOS/PCSX2"
                    } else {
                        self.state.pcsx2_path.clone()
                    };

                    Command::new(pcsx)
                        .arg("-bios")
                        .spawn()
                        .expect("Failed to start PCSX2");
                }
                AppEvent::StartPCSX2Elf(path) => {
                    let pcsx = if cfg!(target_os = "macos") {
                        self.state.pcsx2_path.clone() + "/Contents/MacOS/PCSX2"
                    } else {
                        self.state.pcsx2_path.clone()
                    };

                    Command::new(pcsx)
                        .arg("--")
                        .arg(path)
                        .spawn()
                        .expect("Failed to start PCSX2 with ELF");
                }
                AppEvent::Validate => {
                    validate(
                        self.state
                            .opened_folder
                            .clone()
                            .expect("No opened folder")
                            .to_str()
                            .unwrap(),
                    );
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
                "icn" | "ico" => Some(TabType::ICNViewer(ICNViewer::new(&file, &self.state))),
                "sys" => Some(TabType::IconSysViewer(IconSysViewer::new(
                    &file,
                    &self.state,
                ))),
                "cfg" | "cnf" | "dat" | "txt" => Some(TabType::TitleCfgViewer(
                    TitleCfgViewer::new(&file, &self.state),
                )),
                "toml" => Some(TabType::PsuTomlViewer(PsuTomlViewer::new(
                    &file,
                    &self.state,
                ))),
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
        self.file_tree.index_folder(&folder);
        self.state.files = read_folder(folder)?;

        Ok(())
    }

    fn create_title_cfg(&mut self) {
        if let Some(filepath) = rfd::FileDialog::new()
            .set_title("Select a folder to create title.cfg in")
            .set_file_name("title.cfg")
            .add_filter("title.cfg", &["cfg"])
            .set_directory(self.state.opened_folder.clone().unwrap())
            .save_file()
        {
            std::fs::write(
                filepath.clone(),
                TitleCfg::new("".to_string())
                    .add_missing_fields()
                    .to_string()
                    .into_bytes(),
            )
            .expect("Failed to title.cfg");
            self.handle_open(VirtualFile {
                name: filepath.file_name().unwrap().to_str().unwrap().to_string(),
                size: 0,
                file_path: filepath,
            });
            self.file_tree
                .index_folder(&self.state.opened_folder.clone().unwrap());
        }
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
                ui.set_min_width(200.0);
                self.file_tree.show(ui, &mut self.state);
            });
        }

        egui::CentralPanel::default()
            .frame(Frame::central_panel(&ctx.style()).inner_margin(0))
            .show(ctx, |ui| {
                if self.tree.iter_all_tabs().count() > 0 {
                    let mut style = egui_dock::Style::from_egui(ctx.style().as_ref());
                    style.tab.tab_body.inner_margin = Margin::same(4);
                    style.tab_bar.height = 32.0;
                    style.tab.minimum_width = Some(200.0);
                    DockArea::new(&mut self.tree)
                        .show_leaf_close_all_buttons(false)
                        .show_leaf_collapse_buttons(false)
                        .allowed_splits(AllowedSplits::None)
                        .style(style)
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

        // if self.show_settings {
        //     let rect = ctx.input(|i| i.viewport().outer_rect.unwrap_or(Rect::ZERO));
        //     let center = rect.center();
        //     let window_size = vec2(600.0, 400.0);
        //
        //     ctx.show_viewport_immediate(
        //         ViewportId::from_hash_of("settings"),
        //         ViewportBuilder::default()
        //             .with_title("Settings")
        //             .with_position(center - window_size / 2.0)
        //             .with_inner_size(window_size),
        //         |ctx, _class| {
        //             egui::CentralPanel::default().show(ctx, |ui| {
        //                 Grid::new("settings_grid").num_columns(2).show(ui, |ui| {
        //                     ui.label("PCSX2 Path:");
        //
        //                     #[cfg(target_os = "windows")]
        //                     let filters = Filters::new().add_filter("PCSX2 Executable", ["exe"]);
        //                     #[cfg(target_os = "macos")]
        //                     let filters = Filters::new().add_filter("PCSX2 Application", ["app"]);
        //                     #[cfg(target_os = "linux")]
        //                     let filters = Filters::new();
        //
        //                     ui.file_picker(
        //                         &mut self.state.pcsx2_path,
        //                         filters,
        //                     );
        //                     ui.end_row();
        //                 });
        //             });
        //
        //             if ctx.input(|i| i.viewport().close_requested()) {
        //                 self.show_settings = false;
        //             }
        //         },
        //     );
        // }

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
        eframe::set_value(storage, "pcsx2_path", &self.state.pcsx2_path);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {}
}
