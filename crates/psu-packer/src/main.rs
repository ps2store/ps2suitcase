use argh::FromArgs;
use colored::Colorize;
use std::path::PathBuf;

use psu_packer::{load_config, pack_psu, Error};

#[derive(Debug, FromArgs)]
#[argh(
    description = "Expects a folder with a psu.toml file that follows this format\n\t[config]\n\tname = \"Test PSU\"\t\t\t# Folder name on Memory Card\n\tinclude = [ \"BOOT.ELF\", \"icon.sys\" ]\t# using `exclude` will automatically include all files except the specified ones\n\ttimestamp = \"2024-10-10 10:30:00\"\t# Optional, but recommended\n"
)]
struct Args {
    /// folder to package to psu
    #[argh(positional)]
    folder: String,
    /// output path
    #[argh(option, short = 'o')]
    output: Option<String>,
}

fn main() -> Result<(), Error> {
    let args: Args = argh::from_env();
    let folder = PathBuf::from(&args.folder);

    let config = load_config(&folder)?;
    let output_file = args.output.unwrap_or(format!("{}.psu", config.name));
    let output_path = PathBuf::from(&output_file);

    pack_psu(&folder, &output_path)?;
    println!("Wrote {}! {}", output_file.green(), "".clear());

    Ok(())
}
