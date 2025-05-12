use crate::wizards::wizard::Wizard;
use eframe::egui::{Response, Ui, Widget};
use ps2_filetypes::{
    AnimationHeader,
    AnimationShape,
    ICNHeader,
    IcnTexture,
    Normal,
    Vertex,
    ICN,
    UV,
    BinWriter,
    ICNWriter,
    Color
};
use std::fs::File;
use std::hash::Hash;
use std::io::{Read, Write};
use wavefront_obj::obj::Primitive::Triangle;

pub struct CreateICN {}

impl Widget for &mut CreateICN {
    fn ui(self, ui: &mut Ui) -> Response {
        ui.vertical(|ui| {
            if ui.button("Create ICN").clicked() {
                let mut file = File::open("teapot.obj").unwrap();
                let mut data = vec![];
                file.read_to_end(&mut data).unwrap();

                let obj = wavefront_obj::obj::parse(String::from_utf8(data).unwrap()).expect("Teapot");
                assert_eq!(obj.objects.len(), 1);

                let mut vertices: AnimationShape = vec![];
                let mut normals = vec![];
                let mut uvs = vec![];
                let mut colors = vec![];

                let obj = &obj.objects[0];
                for geom in obj.geometry.iter() {
                    for shape in geom.shapes.iter() {
                        if let Triangle((x, _xt, _xn), (y, _yt, _yn), (z, _zt, _zn)) = shape.primitive {
                            let va = obj.vertices[x];
                            let vb = obj.vertices[y];
                            let vc = obj.vertices[z];

                            vertices.push(Vertex::new((va.x * 4096.0) as i16, -(va.y * 4096.0) as i16, -(va.z * 4096.0) as i16, 0));
                            vertices.push(Vertex::new((vb.x * 4096.0) as i16, -(vb.y * 4096.0) as i16, -(vb.z * 4096.0) as i16, 0));
                            vertices.push(Vertex::new((vc.x * 4096.0) as i16, -(vc.y * 4096.0) as i16, -(vc.z * 4096.0) as i16, 0));

                            for _ in 0..3 {
                                normals.push(Normal::new(0, 0, 0, 0));
                                colors.push(Color::WHITE);
                                uvs.push(UV::new(0, 0));
                            }
                        }
                    }
                }

                let icn = ICN {
                    header: ICNHeader {
                        animation_shape_count: 1,
                        vertex_count: vertices.len() as u32,
                        texture_type: 0x07,
                    },
                    animation_shapes: vec![
                        vertices
                    ],
                    normals,
                    uvs,
                    colors,
                    texture: IcnTexture { pixels: [0xFFFF; 16384] },
                    animation_header: AnimationHeader {
                        tag: 0,
                        frame_length: 0,
                        anim_speed: 0.0,
                        play_offset: 0,
                        frame_count: 0,
                    },
                    frames: vec![],
                };
                File::create("test.icn")
                    .unwrap()
                    .write_all(ICNWriter::new(icn).write().unwrap().as_slice())
                    .unwrap();
            }
        })
        .response
    }
}

impl Wizard for &mut CreateICN {
    fn get_id(&self) -> impl Hash {
        "create_icn"
    }
}
