mod args;
mod config;

use crate::args::{Cli, Commands};
use crate::config::PsuConfig;
use chrono::{DateTime, Local, NaiveDateTime};
use clap::Parser;
use colored::Colorize;
use ps2_filetypes::{PSUWriter, PSU};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use toml::value::Datetime;

fn main() -> Result<(), Error> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Create(args) => {
            let output_filename = get_output_filename(&args.output, &args.name);
            let psu = create_psu(
                &args.name,
                output_filename,
                args.files.clone(),
                args.timestamp,
                Path::new("."),
            );
            println!("{}", psu);
        }
        Commands::Read(args) => {
            let file = fs::read(&args.file)?;
            let psu = PSU::new(file);
            println!("Reading the content of {}\n", args.file);
            println!("{}", psu);
        }
        Commands::Automate(args) => {
            let toml_path = Path::new(&args.toml);
            let raw_toml = fs::read_to_string(&args.toml)?;
            let psu_table: HashMap<String, Vec<PsuConfig>> =
                toml::from_str(&raw_toml).expect("Failed to parse config file");
            let psus: &[PsuConfig] = &psu_table["psu"];

            psus.iter().for_each(|psu| {
                let output_filename = get_output_filename(&psu.output, &psu.name);
                if !args.overwrite && fs::exists(&output_filename).unwrap() {
                    println!(
                        "{} already exists. Use --overwrite if you want to overwrite all .psu.",
                        output_filename,
                    );
                    return;
                }
                let psu = create_psu(
                    &psu.name,
                    output_filename,
                    psu.files.clone(),
                    convert_toml_datetime(psu.timestamp),
                    toml_path.parent().unwrap_or(Path::new(".")),
                );

                println!("{}\n\n{}\n", psu, "--------".dimmed());
            });
        }
        Commands::Add(args) => {
            let file = fs::read(&args.psu)?;
            let mut psu = PSU::new(file.clone());

            args.files.iter().for_each(|file| match psu.add_file(file) {
                Ok(_) => println!("+ Adding {}", file.green()),
                Err(_) => eprintln!("⚠ File {} doesn't exist. Skipping.", file.dimmed()),
            });

            fs::write(
                &args.psu,
                PSUWriter::new(psu.clone())
                    .to_bytes()
                    .expect("Couldn't generate the PSU file"),
            )
            .expect("Couldn't overwrite the PSU");

            println!("\n{}", psu);
        }
        Commands::Delete(args) => {
            let file = fs::read(&args.psu)?;
            let mut psu = PSU::new(file.clone());

            args.entries
                .iter()
                .for_each(|entry| match psu.remove_entry(entry) {
                    Ok(_) => println!("+ Removing {}", entry.green()),
                    Err(_) => eprintln!("⚠ Entry {} doesn't exist. Skipping.", entry.dimmed()),
                });

            fs::write(
                &args.psu,
                PSUWriter::new(psu.clone())
                    .to_bytes()
                    .expect("Couldn't generate the PSU file"),
            )
            .expect("Couldn't overwrite the PSU");

            println!("\n{}", psu);
        }
    }

    Ok(())
}

fn get_output_filename(output: &Option<String>, name: &String) -> String {
    output.to_owned().unwrap_or(format!("{}.psu", name))
}

fn convert_toml_datetime(time: Option<Datetime>) -> Option<NaiveDateTime> {
    match time {
        None => None,
        Some(_) => {
            let datetime_str = format!(
                "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
                time.unwrap().date.unwrap().year,
                time.unwrap().date.unwrap().month,
                time.unwrap().date.unwrap().day,
                time.unwrap().time.unwrap().hour,
                time.unwrap().time.unwrap().minute,
                time.unwrap().time.unwrap().second,
            );
            Some(
                DateTime::<Local>::from_naive_utc_and_offset(
                    (&datetime_str).parse().unwrap(),
                    *Local::now().offset(),
                )
                .naive_local(),
            )
        }
    }
}

fn create_psu(
    name: &String,
    output: String,
    files: Vec<String>,
    timestamp: Option<NaiveDateTime>,
    path_prefix: &Path,
) -> PSU {
    println!("Preparing to create {}", name);
    let mut psu = PSU::default();

    let files = files
        .iter()
        .filter_map(|file| {
            let actual_file_path = path_prefix.join(file).to_str().unwrap().to_string();
            if fs::exists(&actual_file_path).unwrap() {
                return Some(actual_file_path);
            }
            eprintln!("⚠ File {} doesn't exist. Skipping.", file.dimmed());
            None
        })
        .collect::<Vec<_>>();

    psu.add_defaults(name, timestamp.unwrap_or(Local::now().naive_local()));

    files.iter().for_each(|file| {
        psu.add_file(file);
        println!("+ Adding {}", file.green());
    });

    fs::write(
        &output,
        PSUWriter::new(psu.clone())
            .to_bytes()
            .expect("Couldn't generate the PSU file"),
    )
    .expect("Couldn't write the .psu file");
    println!("Wrote {}!\n", output.green());

    psu
}

enum Error {
    IOError(std::io::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::IOError(err) => write!(f, "{err:?}"),
        }
    }
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::IOError(err)
    }
}
