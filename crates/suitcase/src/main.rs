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
    data::files::Files,
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
use ps2_filetypes::templates::{PSU_TOML_TEMPLATE, TITLE_CFG_TEMPLATE};
use std::any::Any;
use std::fmt;
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() -> eframe::Result<()> {
    let mut attempts: Vec<(eframe::Renderer, &str, u16)> = Vec::new();

    #[cfg(feature = "wgpu")]
    {
        attempts.push((eframe::Renderer::Wgpu, "WGPU", 4));
        attempts.push((eframe::Renderer::Wgpu, "WGPU", 1));
    }

    #[cfg(feature = "glow")]
    {
        attempts.push((eframe::Renderer::Glow, "Glow", 4));
        attempts.push((eframe::Renderer::Glow, "Glow", 1));
    }

    let mut last_failure = None;
    let total_attempts = attempts.len();

    for (index, (renderer, name, multisampling)) in attempts.into_iter().enumerate() {
        match try_run_renderer(renderer, multisampling) {
            Ok(()) => return Ok(()),
            Err(failure) => {
                let has_fallback = index + 1 < total_attempts;
                report_renderer_error(name, multisampling, &failure, has_fallback);
                last_failure = Some(failure);
            }
        }
    }

    if let Some(failure) = last_failure {
        Err(failure.into_error())
    } else {
        Ok(())
    }
}

fn create_native_options(renderer: eframe::Renderer, multisampling: u16) -> NativeOptions {
    NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1920.0, 1080.0])
            .with_icon({
                let icon = include_bytes!("../assets/icon.ico");
                match image::load_from_memory(icon) {
                    Ok(image) => {
                        let image = image.into_rgba8();
                        let (width, height) = image.dimensions();

                        IconData {
                            rgba: image.into_raw(),
                            width,
                            height,
                        }
                    }
                    Err(error) => {
                        eprintln!("Failed to load icon: {error}");
                        IconData {
                            rgba: vec![0; 4],
                            width: 1,
                            height: 1,
                        }
                    }
                }
            }),
        multisampling,
        // Request a standard 24-bit depth buffer. WGPU expects at least 24 bits
        // on most platforms, and Glow gracefully ignores the request when it
        // cannot provide a depth buffer.
        depth_buffer: 24,
        renderer,
        ..Default::default()
    }
}

fn run_app(options: NativeOptions) -> eframe::Result<()> {
    eframe::run_native(
        "PS2Suitcase",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(PSUBuilderApp::new(cc)))
        }),
    )
}

fn report_renderer_error(
    renderer: &str,
    multisampling: u16,
    failure: &RendererFailure,
    has_fallback: bool,
) {
    let msaa_description = if multisampling > 1 {
        format!("{multisampling}x MSAA")
    } else {
        "no MSAA".to_owned()
    };

    let mut message =
        format!("Failed to initialize {renderer} renderer with {msaa_description}: {failure}");

    if has_fallback {
        message.push_str("\n\nAttempting fallback...");
    }

    eprintln!("{message}");

    #[cfg(target_os = "windows")]
    {
        use rfd::MessageDialog;

        MessageDialog::new()
            .set_title("PS2Suitcase")
            .set_description(&message)
            .set_buttons(rfd::MessageButtons::Ok)
            .show();
    }
}

fn try_run_renderer(renderer: eframe::Renderer, multisampling: u16) -> Result<(), RendererFailure> {
    let options = create_native_options(renderer, multisampling);
    match panic::catch_unwind(AssertUnwindSafe(|| run_app(options))) {
        Ok(Ok(())) => Ok(()),
        Ok(Err(err)) => Err(RendererFailure::Error(err)),
        Err(payload) => Err(RendererFailure::Panic(panic_message(payload))),
    }
}

fn panic_message(payload: Box<dyn Any + Send>) -> String {
    let payload_ref = &*payload;
    if let Some(message) = payload_ref.downcast_ref::<&str>() {
        (*message).to_owned()
    } else if let Some(message) = payload_ref.downcast_ref::<String>() {
        message.clone()
    } else {
        "Unknown panic".to_owned()
    }
}

enum RendererFailure {
    Panic(String),
    Error(eframe::Error),
}

impl RendererFailure {
    fn into_error(self) -> eframe::Error {
        match self {
            RendererFailure::Panic(message) => {
                eframe::Error::AppCreation(Box::new(PanicAppError(message)))
            }
            RendererFailure::Error(err) => err,
        }
    }
}

impl fmt::Display for RendererFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RendererFailure::Panic(message) => write!(f, "panic: {message}"),
            RendererFailure::Error(err) => write!(f, "{err}"),
        }
    }
}

impl fmt::Debug for RendererFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RendererFailure::Panic(message) => f.debug_tuple("Panic").field(message).finish(),
            RendererFailure::Error(err) => f.debug_tuple("Error").field(err).finish(),
        }
    }
}

#[derive(Debug)]
struct PanicAppError(String);

impl fmt::Display for PanicAppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for PanicAppError {}

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
            match self.do_open_folder(folder.clone()) {
                Ok(_) => Some(()),
                Err(err) => {
                    self.report_folder_error(&folder, &err, false);
                    None
                }
            }
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
                AppEvent::CreatePsuToml => {
                    self.create_psu_toml();
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
            let Some(folder) = self.state.opened_folder.clone() else {
                continue;
            };

            match read_folder(folder.clone()) {
                Ok(files) => {
                    self.state.files = files;
                }
                Err(err) => {
                    self.report_folder_error(&folder, &err, false);
                    self.clear_opened_folder();
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
            if let Err(err) = self.do_open_folder(folder.clone()) {
                self.report_folder_error(&folder, &err, true);
            }
        }
    }

    fn do_open_folder(&mut self, folder: PathBuf) -> std::io::Result<()> {
        let files = read_folder(folder.clone())?;
        self.file_tree.index_folder(&folder)?;

        self.state.opened_folder = Some(folder.clone());
        self.state.set_title(
            folder
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
        );
        self.file_watcher.change_path(&folder);
        self.state.files = files;

        Ok(())
    }

    fn clear_opened_folder(&mut self) {
        self.state.opened_folder = None;
        self.state.set_title("PS2Suitcase".to_owned());
        self.state.files = Files::default();
    }

    fn report_folder_error(&self, folder: &Path, err: &std::io::Error, show_dialog: bool) {
        eprintln!("Failed to load folder '{}': {err}", folder.display());

        if show_dialog {
            #[cfg(not(target_arch = "wasm32"))]
            {
                use rfd::{MessageButtons, MessageDialog};

                MessageDialog::new()
                    .set_title("PS2Suitcase")
                    .set_description(&format!(
                        "Failed to load folder '{}':\n{}",
                        folder.display(),
                        err
                    ))
                    .set_buttons(MessageButtons::Ok)
                    .show();
            }
        }
    }

    fn create_title_cfg(&mut self) {
        let Some(directory) = self.state.opened_folder.clone() else {
            return;
        };

        if let Some(filepath) = rfd::FileDialog::new()
            .set_title("Select a folder to create title.cfg in")
            .set_file_name("title.cfg")
            .add_filter("title.cfg", &["cfg"])
            .set_directory(directory)
            .save_file()
        {
            std::fs::write(&filepath, TITLE_CFG_TEMPLATE)
                .expect("Failed to write title.cfg template");
            let file_name = filepath.file_name().unwrap().to_str().unwrap().to_string();
            self.handle_open(VirtualFile {
                name: file_name,
                size: 0,
                file_path: filepath,
            });
            if let Some(folder) = &self.state.opened_folder {
                if let Err(err) = self.file_tree.index_folder(folder) {
                    self.report_folder_error(folder.as_path(), &err, false);
                }
            }
        }
    }

    fn create_psu_toml(&mut self) {
        let Some(directory) = self.state.opened_folder.clone() else {
            return;
        };

        if let Some(filepath) = rfd::FileDialog::new()
            .set_title("Select a folder to create psu.toml in")
            .set_file_name("psu.toml")
            .add_filter("psu.toml", &["toml"])
            .set_directory(directory)
            .save_file()
        {
            std::fs::write(&filepath, PSU_TOML_TEMPLATE)
                .expect("Failed to write psu.toml template");
            let file_name = filepath.file_name().unwrap().to_str().unwrap().to_string();
            self.handle_open(VirtualFile {
                name: file_name,
                size: 0,
                file_path: filepath,
            });
            if let Some(folder) = &self.state.opened_folder {
                if let Err(err) = self.file_tree.index_folder(folder) {
                    self.report_folder_error(folder.as_path(), &err, false);
                }
            }
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
