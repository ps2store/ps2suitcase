use chrono::{DateTime, Local, NaiveDateTime};
use colored::Colorize;
use ps2_filetypes::{PSUEntry, PSUEntryKind, PSUWriter, DIR_ID, FILE_ID, PSU};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub name: String,
    #[serde(default, with = "date_format")]
    pub timestamp: Option<NaiveDateTime>,
    pub include: Option<Vec<String>>,
    pub exclude: Option<Vec<String>>,
}

mod date_format {
    use chrono::NaiveDateTime;
    use serde::{self, Deserialize, Deserializer};

    pub fn deserialize<'de, D>(deserialize: D) -> Result<Option<NaiveDateTime>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: Option<String> = Option::deserialize(deserialize)?;
        if let Some(s) = s {
            Ok(Some(
                NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S")
                    .map_err(serde::de::Error::custom)?,
            ))
        } else {
            Ok(None)
        }
    }
}

#[derive(Debug, Deserialize)]
struct ConfigFile {
    config: Config,
}

pub fn load_config(folder: &Path) -> Result<Config, Error> {
    let config_file = folder.join("psu.toml");
    let str = std::fs::read_to_string(&config_file)?;
    let config = toml::from_str::<ConfigFile>(&str)
        .map_err(|e| Error::ConfigError(e.to_string()))?
        .config;
    Ok(config)
}

pub fn pack_psu(folder: &Path, output: &Path) -> Result<(), Error> {
    let config = load_config(folder)?;
    pack_with_config(folder, output, config)
}

pub fn pack_with_config(folder: &Path, output: &Path, cfg: Config) -> Result<(), Error> {
    let Config {
        name,
        timestamp,
        include,
        exclude,
    } = cfg;

    if !check_name(&name) {
        return Err(Error::NameError);
    }

    if include.is_some() && exclude.is_some() {
        return Err(Error::IncludeExcludeError);
    }

    let mut psu = PSU::default();

    let files = if let Some(include) = include {
        include
            .iter()
            .filter_map(|file| {
                if file.contains(|c| matches!(c, '\\' | '/')) {
                    eprintln!(
                        "{} {} {}",
                        "File".dimmed(),
                        file.dimmed(),
                        "exists in subfolder, skipping".dimmed()
                    );
                    None
                } else if !folder.join(file).exists() {
                    eprintln!(
                        "{} {} {}",
                        "File".dimmed(),
                        file.dimmed(),
                        "does not exist, skipping".dimmed()
                    );
                    None
                } else {
                    Some(folder.join(file))
                }
            })
            .collect::<Vec<_>>()
    } else if let Some(exclude) = exclude {
        std::fs::read_dir(folder)?
            .into_iter()
            .flatten()
            .filter_map(|d| {
                if !exclude.contains(&d.file_name().to_str().unwrap().to_string()) {
                    Some(d.path())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
    } else {
        std::fs::read_dir(folder)?
            .into_iter()
            .flatten()
            .map(|d| d.path())
            .collect::<Vec<_>>()
    };
    let files = filter_files(&files);
    add_psu_defaults(&mut psu, &name, files.len(), timestamp.unwrap_or_default());
    add_files_to_psu(&mut psu, &files)?;
    std::fs::write(output, PSUWriter::new(psu).to_bytes()?)?;
    Ok(())
}

fn check_name(name: &str) -> bool {
    for c in name.chars() {
        if !matches!(c, 'a'..'z'|'A'..'Z'|'0'..'9'|'_'|'-'|' ') {
            return false;
        }
    }
    true
}

fn filter_files(files: &[PathBuf]) -> Vec<PathBuf> {
    files
        .iter()
        .filter_map(|f| {
            if !f.is_file() {
                println!(
                    "{} {}",
                    f.display().to_string().dimmed(),
                    "is not a file, skipping".dimmed()
                );
                None
            } else {
                Some(f.to_owned())
            }
        })
        .collect()
}

fn add_psu_defaults(psu: &mut PSU, name: &str, file_count: usize, timestamp: NaiveDateTime) {
    psu.entries.push(PSUEntry {
        id: DIR_ID,
        size: file_count as u32 + 2,
        created: timestamp,
        sector: 0,
        modified: timestamp,
        name: name.to_owned(),
        kind: PSUEntryKind::Directory,
        contents: None,
    });
    psu.entries.push(PSUEntry {
        id: DIR_ID,
        size: 0,
        created: timestamp,
        sector: 0,
        modified: timestamp,
        name: ".".to_string(),
        kind: PSUEntryKind::Directory,
        contents: None,
    });
    psu.entries.push(PSUEntry {
        id: DIR_ID,
        size: 0,
        created: timestamp,
        sector: 0,
        modified: timestamp,
        name: "..".to_string(),
        kind: PSUEntryKind::Directory,
        contents: None,
    });
}

fn add_files_to_psu(psu: &mut PSU, files: &[PathBuf]) -> Result<(), Error> {
    for file in files {
        let name = file.file_name().unwrap().to_str().unwrap();

        let f = std::fs::read(file)?;
        let stat = std::fs::metadata(file)?;

        println!("+ {} {}", "Adding", name.green());

        psu.entries.push(PSUEntry {
            id: FILE_ID,
            size: f.len() as u32,
            created: convert_timestamp(stat.created()?),
            sector: 0,
            modified: convert_timestamp(stat.modified()?),
            name: name.to_owned(),
            kind: PSUEntryKind::File,
            contents: Some(f),
        })
    }

    Ok(())
}

fn convert_timestamp(time: SystemTime) -> NaiveDateTime {
    let duration = time.duration_since(UNIX_EPOCH).unwrap();
    DateTime::from_timestamp(duration.as_secs() as i64, duration.subsec_nanos())
        .unwrap()
        .with_timezone(&Local)
        .naive_local()
}

#[derive(Debug)]
pub enum Error {
    NameError,
    IOError(std::io::Error),
    IncludeExcludeError,
    ConfigError(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::NameError => write!(f, "Name must match [a-zA-Z0-9._-\\s]+"),
            Error::IncludeExcludeError => write!(f, "Exclude cannot be used in include mode"),
            Error::IOError(err) => write!(f, "{err:?}"),
            Error::ConfigError(err) => write!(f, "{err}"),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::IOError(err)
    }
}
