use eframe::egui::{Align, Align2, Color32, Context, Id, LayerId, Order, TextStyle};
use eframe::{egui, Frame};
// use image::{ImageBuffer, Rgb, RgbImage};
use ps2_filetypes::{PSUEntry, ICN, PSU};
use rfd::FileDialog;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

fn main() {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([640.0, 240.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };
    eframe::run_native(
        "PS2 MCM",
        options,
        Box::new(|_cc| Ok(Box::<MCM>::default())),
    )
    .unwrap()
}

#[derive(Default)]
struct MCM {
    files: Vec<PSUEntry>,
}

impl MCM {
    fn load_file<P>(&mut self, path: P)
    where
        P: AsRef<Path>,
    {
        let mut psu_raw = File::open(path).unwrap();
        let mut contents = vec![];
        psu_raw.read_to_end(&mut contents).unwrap();
        let psu = PSU::new(contents);
        self.files = psu.entries;

        // for file in self.files.iter() {
        //     if file.name == "list.icn" {
        //         let icn = ICN::new(file.contents.clone().unwrap());
        //         let mut output = File::create("/Users/<user>/Downloads/list.obj")
        //             .expect("Failed to open");
        //         output
        //             .write_all(icn.export_obj().as_bytes())
        //             .expect("Failed to write");
                // let mut image = RgbImage::new(128, 128);
                // for y in 0..128 {
                //     for x in 0..128 {
                //         let pixel = icn.texture.pixels[y * 128 + x];
                //         let r = ((pixel & 0x3F) * 255 / 31) as u8;
                //         let g = (((pixel >> 5) & 0x3F) * 255 / 31) as u8;
                //         let b = (((pixel >> 10) & 0x3F) * 255 / 31) as u8;
                //         image.put_pixel(x as u32, y as u32, Rgb([r,g,b]));
                //     }
                // }
                // image.save("/Users/<user>/Downloads/list.png").unwrap();
        //
        //         break;
        //     }
        // }
    }
}

impl eframe::App for MCM {
    fn update(&mut self, ctx: &Context, frame: &mut Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if ui.button("Load File").clicked() {
                // self.load_file("/Users/simonhochrein/Downloads/EMU_PICODRIVE-201.psu")
                self.load_file("WLE.PSU")
            }
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.with_layout(
                    egui::Layout::top_down_justified(Align::LEFT).with_cross_justify(true),
                    |ui| {
                        for file in &self.files {
                            ui.selectable_label(false, &file.name).context_menu(|ui| {
                                if let Some(contents) = &file.contents {
                                    if ui.button("Export").clicked() {
                                        if let Some(path) =
                                            FileDialog::new().set_file_name(&file.name).save_file()
                                        {
                                            let mut export_file =
                                                File::create(&path).expect("Failed to export file");
                                            export_file
                                                .write_all(contents)
                                                .expect("Failed to write file");
                                            ui.close_menu();
                                        }
                                    }
                                }
                            });
                        }
                    },
                );
            });
        });

        if !ctx.input(|i| i.raw.hovered_files.is_empty()) {
            let painter = ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("overlay")));

            let screen_rect = ctx.screen_rect();
            painter.rect_filled(screen_rect, 0.0, Color32::from_black_alpha(192));
            painter.text(
                screen_rect.center(),
                Align2::CENTER_CENTER,
                "Drop files to open",
                TextStyle::Heading.resolve(&ctx.style()),
                Color32::WHITE,
            );
        }

        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                for file in &i.raw.dropped_files {
                    let path = file.path.clone().unwrap();
                    self.load_file(path);
                }
            }
        })
    }
}
