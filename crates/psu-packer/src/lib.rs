use chrono::{DateTime, Local, NaiveDateTime};
use colored::Colorize;
use ps2_filetypes::color::Color;
use ps2_filetypes::{
    ColorF, IconSys, PSUEntry, PSUEntryKind, PSUWriter, Vector, DIR_ID, FILE_ID, PSU,
};
use serde::{Deserialize, Deserializer};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug)]
pub struct Config {
    pub name: String,
    pub timestamp: Option<NaiveDateTime>,
    pub include: Option<Vec<String>>,
    pub exclude: Option<Vec<String>>,
    pub icon_sys: Option<IconSysConfig>,
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
    config: ConfigSection,
    #[serde(default)]
    icon_sys: Option<IconSysConfig>,
}

#[derive(Debug, Deserialize)]
struct ConfigSection {
    name: String,
    #[serde(default, with = "date_format")]
    timestamp: Option<NaiveDateTime>,
    include: Option<Vec<String>>,
    exclude: Option<Vec<String>>,
}

impl From<ConfigFile> for Config {
    fn from(file: ConfigFile) -> Self {
        let ConfigFile { config, icon_sys } = file;
        Self {
            name: config.name,
            timestamp: config.timestamp,
            include: config.include,
            exclude: config.exclude,
            icon_sys,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct IconSysConfig {
    pub flags: IconSysFlags,
    pub title: String,
    #[serde(default)]
    pub background_transparency: Option<u32>,
    #[serde(default)]
    pub background_colors: Option<Vec<ColorConfig>>,
    #[serde(default)]
    pub light_directions: Option<Vec<VectorConfig>>,
    #[serde(default)]
    pub light_colors: Option<Vec<ColorFConfig>>,
    #[serde(default)]
    pub ambient_color: Option<ColorFConfig>,
}

impl IconSysConfig {
    fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        let icon_sys = self.build_icon_sys()?;
        icon_sys
            .to_bytes()
            .map_err(|err| Error::ConfigError(err.to_string()))
    }

    fn build_icon_sys(&self) -> Result<IconSys, Error> {
        let mut background_colors = DEFAULT_BACKGROUND_COLORS;
        if let Some(colors) = &self.background_colors {
            if colors.len() != background_colors.len() {
                return Err(Error::ConfigError(format!(
                    "icon_sys.background_colors must contain exactly {} entries",
                    background_colors.len()
                )));
            }

            for (target, value) in background_colors.iter_mut().zip(colors.iter()) {
                *target = (*value).into();
            }
        }

        let mut light_directions = DEFAULT_LIGHT_DIRECTIONS;
        if let Some(directions) = &self.light_directions {
            if directions.len() != light_directions.len() {
                return Err(Error::ConfigError(format!(
                    "icon_sys.light_directions must contain exactly {} entries",
                    light_directions.len()
                )));
            }

            for (target, value) in light_directions.iter_mut().zip(directions.iter()) {
                *target = (*value).into();
            }
        }

        let mut light_colors = DEFAULT_LIGHT_COLORS;
        if let Some(colors) = &self.light_colors {
            if colors.len() != light_colors.len() {
                return Err(Error::ConfigError(format!(
                    "icon_sys.light_colors must contain exactly {} entries",
                    light_colors.len()
                )));
            }

            for (target, value) in light_colors.iter_mut().zip(colors.iter()) {
                *target = (*value).into();
            }
        }

        let ambient_color = self
            .ambient_color
            .map(|color| color.into())
            .unwrap_or(DEFAULT_AMBIENT_COLOR);

        let background_transparency = self
            .background_transparency
            .unwrap_or(DEFAULT_BACKGROUND_TRANSPARENCY);

        Ok(IconSys {
            flags: self.flags.value(),
            linebreak_pos: DEFAULT_LINEBREAK_POS,
            background_transparency,
            background_colors,
            light_directions,
            light_colors,
            ambient_color,
            title: self.title.clone(),
            icon_file: ICON_FILE_NAME.to_string(),
            icon_copy_file: ICON_FILE_NAME.to_string(),
            icon_delete_file: ICON_FILE_NAME.to_string(),
        })
    }
}

#[derive(Debug, Deserialize, Clone, Copy)]
pub struct ColorConfig {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl From<ColorConfig> for Color {
    fn from(value: ColorConfig) -> Self {
        Color {
            r: value.r,
            g: value.g,
            b: value.b,
            a: value.a,
        }
    }
}

#[derive(Debug, Deserialize, Clone, Copy)]
pub struct ColorFConfig {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl From<ColorFConfig> for ColorF {
    fn from(value: ColorFConfig) -> Self {
        ColorF {
            r: value.r,
            g: value.g,
            b: value.b,
            a: value.a,
        }
    }
}

#[derive(Debug, Deserialize, Clone, Copy)]
pub struct VectorConfig {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl From<VectorConfig> for Vector {
    fn from(value: VectorConfig) -> Self {
        Vector {
            x: value.x,
            y: value.y,
            z: value.z,
            w: value.w,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IconSysFlags(u16);

impl IconSysFlags {
    pub const fn new(value: u16) -> Self {
        Self(value)
    }

    pub const fn value(self) -> u16 {
        self.0
    }
}

impl From<u16> for IconSysFlags {
    fn from(value: u16) -> Self {
        Self::new(value)
    }
}

impl From<IconSysFlags> for u16 {
    fn from(value: IconSysFlags) -> Self {
        value.value()
    }
}

impl<'de> Deserialize<'de> for IconSysFlags {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct IconSysFlagsVisitor;

        impl<'de> serde::de::Visitor<'de> for IconSysFlagsVisitor {
            type Value = IconSysFlags;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("an icon.sys flag value or descriptive name")
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if value > u16::MAX as u64 {
                    return Err(E::custom("icon_sys.flags must be between 0 and 65535"));
                }
                Ok(IconSysFlags::new(value as u16))
            }

            fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if !(0..=u16::MAX as i64).contains(&value) {
                    return Err(E::custom("icon_sys.flags must be between 0 and 65535"));
                }
                Ok(IconSysFlags::new(value as u16))
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                parse_flag_string(value)
                    .map(IconSysFlags::new)
                    .map_err(E::custom)
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                self.visit_str(&value)
            }
        }

        deserializer.deserialize_any(IconSysFlagsVisitor)
    }
}

fn parse_flag_string(value: &str) -> Result<u16, String> {
    let trimmed = value.trim();
    if let Some(mapped) = parse_named_flag(trimmed) {
        return Ok(mapped);
    }

    if let Some(stripped) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        return u16::from_str_radix(stripped, 16)
            .map_err(|_| format!("Invalid hexadecimal icon_sys flag: {trimmed}"));
    }

    trimmed
        .parse::<u16>()
        .map_err(|_| format!("Invalid icon_sys flag value: {trimmed}"))
}

fn parse_named_flag(value: &str) -> Option<u16> {
    let normalized: String = value
        .to_ascii_lowercase()
        .chars()
        .filter(|c| !c.is_ascii_whitespace() && *c != '_' && *c != '(' && *c != ')')
        .collect();

    match normalized.as_str() {
        "ps2savefile" | "savefile" => Some(0),
        "softwareps2" | "software" => Some(1),
        "unrecognizeddata" | "unrecognized" | "data" => Some(2),
        "softwarepocketstation" | "pocketstation" => Some(3),
        "settingsps2" | "settings" => Some(4),
        "systemdriver" | "driver" => Some(5),
        _ => None,
    }
}

const DEFAULT_LINEBREAK_POS: u16 = 0;
const DEFAULT_BACKGROUND_TRANSPARENCY: u32 = 0;
const DEFAULT_BACKGROUND_COLORS: [Color; 4] = [
    Color {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    },
    Color {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    },
    Color {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    },
    Color {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    },
];
const DEFAULT_LIGHT_DIRECTIONS: [Vector; 3] = [
    Vector {
        x: 0.0,
        y: 0.0,
        z: 1.0,
        w: 0.0,
    },
    Vector {
        x: 0.0,
        y: 0.0,
        z: 1.0,
        w: 0.0,
    },
    Vector {
        x: 0.0,
        y: 0.0,
        z: 1.0,
        w: 0.0,
    },
];
const DEFAULT_LIGHT_COLORS: [ColorF; 3] = [
    ColorF {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    },
    ColorF {
        r: 0.5,
        g: 0.5,
        b: 0.5,
        a: 1.0,
    },
    ColorF {
        r: 0.3,
        g: 0.3,
        b: 0.3,
        a: 1.0,
    },
];
const DEFAULT_AMBIENT_COLOR: ColorF = ColorF {
    r: 0.2,
    g: 0.2,
    b: 0.2,
    a: 1.0,
};
const ICON_FILE_NAME: &str = "icon.icn";

pub fn load_config(folder: &Path) -> Result<Config, Error> {
    let config_file = folder.join("psu.toml");
    let str = std::fs::read_to_string(&config_file)?;
    let config_file =
        toml::from_str::<ConfigFile>(&str).map_err(|e| Error::ConfigError(e.to_string()))?;
    Ok(config_file.into())
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
        icon_sys,
    } = cfg;

    if !check_name(&name) {
        return Err(Error::NameError);
    }

    let mut psu = PSU::default();

    let icon_sys_path = folder.join("icon.sys");
    if let Some(icon_config) = &icon_sys {
        let bytes = icon_config.to_bytes()?;
        std::fs::write(&icon_sys_path, bytes)?;
    }

    let raw_included_files = if let Some(include) = include {
        include
            .into_iter()
            .filter_map(|file| {
                if file.contains(|c| matches!(c, '\\' | '/')) {
                    eprintln!(
                        "{} {} {}",
                        "File".dimmed(),
                        file.dimmed(),
                        "exists in subfolder, skipping".dimmed()
                    );
                    None
                } else {
                    let candidate = folder.join(&file);
                    if !candidate.exists() {
                        eprintln!(
                            "{} {} {}",
                            "File".dimmed(),
                            file.dimmed(),
                            "does not exist, skipping".dimmed()
                        );
                        None
                    } else {
                        Some(candidate)
                    }
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

    let mut files = filter_files(&raw_included_files);

    if let Some(exclude) = exclude {
        let mut exclude_set = HashSet::new();

        for file in exclude {
            if file.contains(|c| matches!(c, '\\' | '/')) {
                eprintln!(
                    "{} {} {}",
                    "File".dimmed(),
                    file.dimmed(),
                    "exists in subfolder, skipping exclude".dimmed()
                );
                continue;
            }

            let candidate = folder.join(&file);
            if !candidate.exists() {
                eprintln!(
                    "{} {} {}",
                    "File".dimmed(),
                    file.dimmed(),
                    "does not exist, skipping exclude".dimmed()
                );
                continue;
            }

            exclude_set.insert(file);
        }

        if !exclude_set.is_empty() {
            files = files
                .into_iter()
                .filter(|path| {
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .map(|name| !exclude_set.contains(name))
                        .unwrap_or(true)
                })
                .collect::<Vec<_>>();
        }
    }

    if icon_sys.is_some() {
        if !files.iter().any(|path| path == &icon_sys_path) {
            files.push(icon_sys_path);
        }
    }

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
    ConfigError(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::NameError => write!(f, "Name must match [a-zA-Z0-9._-\\s]+"),
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
