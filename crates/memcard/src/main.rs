use std::io::Read;
use std::io::Seek;
use crate::dir_entry::DF_DIRECTORY;

mod fat;
mod dir_entry;

fn main() -> std::io::Result<()> {
    let data = include_bytes!("../NewCard.ps2").to_vec();

    let mut mc = fat::Memcard::new(data);

    let root = mc.get_child(mc.superblock.rootdir_cluster, 0)?;
    let folders = mc.ls(&root)?;

    for folder in folders {
        eprintln!("{}", String::from_utf8(folder.name.to_vec()).unwrap());

        if folder.mode & DF_DIRECTORY > 0 {
            for file in mc.ls(&folder)? {
                let name = String::from_utf8(file.name.to_vec()).unwrap().trim_end_matches(|c| c == '\0').to_owned();
                if name == "icon.sys" {
                    println!("Reading icon.sys");
                    std::fs::write("icon.sys", mc.read(&file, file.length as usize, 0)?)?;
                    return Ok(());
                }
                eprintln!("\t{}", name);
            }
        }
    }
    Ok(())
}
