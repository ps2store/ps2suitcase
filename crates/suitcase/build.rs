use {
    std::{env, io},
    winresource::WindowsResource,
};

fn main() -> io::Result<()> {
    let has_wgpu = env::var_os("CARGO_FEATURE_WGPU").is_some();
    let has_glow = env::var_os("CARGO_FEATURE_GLOW").is_some();

    if !has_wgpu && !has_glow {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "PS2Suitcase requires either the `wgpu` or `glow` feature to be enabled.",
        ));
    }

    if env::var_os("CARGO_CFG_WINDOWS").is_some() {
        WindowsResource::new()
            // This path can be absolute, or relative to your crate root.
            .set_icon("assets/icon.ico")
            .compile()?;
    }
    Ok(())
}
