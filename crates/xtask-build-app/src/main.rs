use std::fs::{create_dir, create_dir_all, read_dir, remove_dir_all};
use std::{env, io};
use std::io::ErrorKind;

fn cwd_to_workspace_root() -> io::Result<()> {
    let pkg_root = env!("CARGO_MANIFEST_DIR");
    let ws_root = format!("{pkg_root}/../..");
    std::env::set_current_dir(ws_root)
}

fn main() -> io::Result<()> {
    if !cfg!(target_os = "macos") {
        return Err(io::Error::new(ErrorKind::Other, "unsupported operating system"));
    }
    
    cwd_to_workspace_root()?;
    
    let cur_dir = env::current_dir()?;
    
    let contents_path = cur_dir.join("build/PSU Builder.app/Contents");
    let mac_os_path = contents_path.join("MacOS");
    let resources_path = contents_path.join("Resources");
    
    remove_dir_all(cur_dir.join("build/PSU Builder.app"))?;
    create_dir_all(&mac_os_path)?;
    create_dir_all(&resources_path)?;

    std::fs::copy("target/debug/builder", mac_os_path.join("PSU Builder"))?;
    std::fs::copy("crates/builder/assets/ps2.icns", resources_path.join("icon.icns"))?;
    std::fs::copy("crates/builder/assets/Info.plist", contents_path.join("Info.plist"))?;
    
    
    Ok(())
}
