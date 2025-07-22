mod dir_entry;
mod fat;

fn main() -> std::io::Result<()> {
    let data = std::fs::read("../NewCard.ps2").expect("cannot read file");

    let mut mc = fat::Memcard::new(data);

    let folders = mc.read_entry_cluster(mc.rootdir_cluster as u32);
    let root = folders[0];

    eprintln!("{:#?}", root);

    mc.print_allocation_table_recursive();
    // let folders = mc.find_sub_entries(&root);

    // for folder in folders {
    //     let str = String::from_utf8(folder.name.to_vec())
    //         .unwrap()
    //         .trim_end_matches('\0')
    //         .to_string();
    //
    //     if str == "BISLPM-65880DMC3" {
    //         for file in mc.find_sub_entries(&folder) {
    //             let str = String::from_utf8(file.name.to_vec())
    //                 .unwrap()
    //                 .trim_end_matches('\0')
    //                 .to_string();
    //
    //             if str == "icon00.ico" {
    //                 let data = mc.read_data_cluster(&file);
    //                 std::fs::write("icon00.ico", data).expect("Unable to write file");
    //             }
    //         }
    //     }
    // }
    Ok(())
}
