use chrono::{DateTime, Local, NaiveDateTime};
use colored::Colorize;
use ps2_filetypes::color::Color;
use ps2_filetypes::{
    ColorF, IconSys, PSUEntry, PSUEntryKind, PSUWriter, Vector, DIR_ID, FILE_ID, PSU,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
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
    use serde::{self, Deserialize, Deserializer, Serializer};

    const FORMAT: &str = "%Y-%m-%d %H:%M:%S";

    pub fn serialize<S>(value: &Option<NaiveDateTime>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(value) => serializer.serialize_some(&value.format(FORMAT).to_string()),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserialize: D) -> Result<Option<NaiveDateTime>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: Option<String> = Option::deserialize(deserialize)?;
        if let Some(s) = s {
            Ok(Some(
                NaiveDateTime::parse_from_str(&s, FORMAT).map_err(serde::de::Error::custom)?,
            ))
        } else {
            Ok(None)
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct ConfigFile {
    config: ConfigSection,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    icon_sys: Option<IconSysConfig>,
}

#[derive(Debug, Deserialize, Serialize)]
struct ConfigSection {
    name: String,
    #[serde(default, with = "date_format", skip_serializing_if = "Option::is_none")]
    timestamp: Option<NaiveDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    include: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
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

impl Config {
    pub fn to_toml_string(&self) -> Result<String, toml::ser::Error> {
        let config_section = ConfigSection {
            name: self.name.clone(),
            timestamp: self.timestamp,
            include: self.include.clone(),
            exclude: self.exclude.clone(),
        };

        let config_file = ConfigFile {
            config: config_section,
            icon_sys: self.icon_sys.clone(),
        };

        toml::to_string_pretty(&config_file)
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct IconSysConfig {
    pub flags: IconSysFlags,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub linebreak_pos: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preset: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_transparency: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_colors: Option<Vec<ColorConfig>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub light_directions: Option<Vec<VectorConfig>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub light_colors: Option<Vec<ColorFConfig>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
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

        let linebreak_pos = self.linebreak_pos.unwrap_or(DEFAULT_LINEBREAK_POS);

        Ok(IconSys {
            flags: self.flags.value(),
            linebreak_pos,
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

impl IconSysConfig {
    pub const fn default_linebreak_pos() -> u16 {
        DEFAULT_LINEBREAK_POS
    }

    pub const fn default_background_transparency() -> u32 {
        DEFAULT_BACKGROUND_TRANSPARENCY
    }

    pub const fn default_background_colors() -> [ColorConfig; 4] {
        [
            ColorConfig {
                r: DEFAULT_BACKGROUND_COLORS[0].r,
                g: DEFAULT_BACKGROUND_COLORS[0].g,
                b: DEFAULT_BACKGROUND_COLORS[0].b,
                a: DEFAULT_BACKGROUND_COLORS[0].a,
            },
            ColorConfig {
                r: DEFAULT_BACKGROUND_COLORS[1].r,
                g: DEFAULT_BACKGROUND_COLORS[1].g,
                b: DEFAULT_BACKGROUND_COLORS[1].b,
                a: DEFAULT_BACKGROUND_COLORS[1].a,
            },
            ColorConfig {
                r: DEFAULT_BACKGROUND_COLORS[2].r,
                g: DEFAULT_BACKGROUND_COLORS[2].g,
                b: DEFAULT_BACKGROUND_COLORS[2].b,
                a: DEFAULT_BACKGROUND_COLORS[2].a,
            },
            ColorConfig {
                r: DEFAULT_BACKGROUND_COLORS[3].r,
                g: DEFAULT_BACKGROUND_COLORS[3].g,
                b: DEFAULT_BACKGROUND_COLORS[3].b,
                a: DEFAULT_BACKGROUND_COLORS[3].a,
            },
        ]
    }

    pub const fn default_light_directions() -> [VectorConfig; 3] {
        [
            VectorConfig {
                x: DEFAULT_LIGHT_DIRECTIONS[0].x,
                y: DEFAULT_LIGHT_DIRECTIONS[0].y,
                z: DEFAULT_LIGHT_DIRECTIONS[0].z,
                w: DEFAULT_LIGHT_DIRECTIONS[0].w,
            },
            VectorConfig {
                x: DEFAULT_LIGHT_DIRECTIONS[1].x,
                y: DEFAULT_LIGHT_DIRECTIONS[1].y,
                z: DEFAULT_LIGHT_DIRECTIONS[1].z,
                w: DEFAULT_LIGHT_DIRECTIONS[1].w,
            },
            VectorConfig {
                x: DEFAULT_LIGHT_DIRECTIONS[2].x,
                y: DEFAULT_LIGHT_DIRECTIONS[2].y,
                z: DEFAULT_LIGHT_DIRECTIONS[2].z,
                w: DEFAULT_LIGHT_DIRECTIONS[2].w,
            },
        ]
    }

    pub const fn default_light_colors() -> [ColorFConfig; 3] {
        [
            ColorFConfig {
                r: DEFAULT_LIGHT_COLORS[0].r,
                g: DEFAULT_LIGHT_COLORS[0].g,
                b: DEFAULT_LIGHT_COLORS[0].b,
                a: DEFAULT_LIGHT_COLORS[0].a,
            },
            ColorFConfig {
                r: DEFAULT_LIGHT_COLORS[1].r,
                g: DEFAULT_LIGHT_COLORS[1].g,
                b: DEFAULT_LIGHT_COLORS[1].b,
                a: DEFAULT_LIGHT_COLORS[1].a,
            },
            ColorFConfig {
                r: DEFAULT_LIGHT_COLORS[2].r,
                g: DEFAULT_LIGHT_COLORS[2].g,
                b: DEFAULT_LIGHT_COLORS[2].b,
                a: DEFAULT_LIGHT_COLORS[2].a,
            },
        ]
    }

    pub const fn default_ambient_color() -> ColorFConfig {
        ColorFConfig {
            r: DEFAULT_AMBIENT_COLOR.r,
            g: DEFAULT_AMBIENT_COLOR.g,
            b: DEFAULT_AMBIENT_COLOR.b,
            a: DEFAULT_AMBIENT_COLOR.a,
        }
    }

    pub fn background_transparency_value(&self) -> u32 {
        self.background_transparency
            .unwrap_or(Self::default_background_transparency())
    }

    pub fn background_colors_array(&self) -> [ColorConfig; 4] {
        let mut colors = Self::default_background_colors();
        if let Some(values) = &self.background_colors {
            for (target, value) in colors.iter_mut().zip(values.iter()) {
                *target = *value;
            }
        }
        colors
    }

    pub fn light_directions_array(&self) -> [VectorConfig; 3] {
        let mut directions = Self::default_light_directions();
        if let Some(values) = &self.light_directions {
            for (target, value) in directions.iter_mut().zip(values.iter()) {
                *target = *value;
            }
        }
        directions
    }

    pub fn light_colors_array(&self) -> [ColorFConfig; 3] {
        let mut colors = Self::default_light_colors();
        if let Some(values) = &self.light_colors {
            for (target, value) in colors.iter_mut().zip(values.iter()) {
                *target = *value;
            }
        }
        colors
    }

    pub fn ambient_color_value(&self) -> ColorFConfig {
        self.ambient_color
            .unwrap_or_else(Self::default_ambient_color)
    }

    pub fn linebreak_position(&self) -> u16 {
        self.linebreak_pos.unwrap_or(Self::default_linebreak_pos())
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
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

#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
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

#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_sys_serializes_two_16_char_lines() {
        let line1 = "ABCDEFGHIJKLMNOP";
        let line2 = "QRSTUVWXYZ012345";
        let title = format!("{line1}{line2}");
        let icon_sys = IconSysConfig {
            flags: IconSysFlags::new(0),
            title: title.clone(),
            linebreak_pos: Some(line1.chars().count() as u16),
            preset: None,
            background_transparency: None,
            background_colors: None,
            light_directions: None,
            light_colors: None,
            ambient_color: None,
        };

        assert_eq!(icon_sys.linebreak_position(), line1.chars().count() as u16);

        let config = Config {
            name: "Example".to_string(),
            timestamp: None,
            include: None,
            exclude: None,
            icon_sys: Some(icon_sys.clone()),
        };

        let toml = config
            .to_toml_string()
            .expect("icon_sys config serializes to TOML");
        let parsed: toml::Value = toml::from_str(&toml).expect("parse TOML output");
        let icon_sys_table = parsed
            .get("icon_sys")
            .and_then(|value| value.as_table())
            .expect("icon_sys table present");

        assert_eq!(
            icon_sys_table.get("title").and_then(|value| value.as_str()),
            Some(title.as_str())
        );
        assert_eq!(
            icon_sys_table
                .get("linebreak_pos")
                .and_then(|value| value.as_integer()),
            Some(line1.chars().count() as i64)
        );
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

impl Serialize for IconSysFlags {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u16(self.value())
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

    let timestamp_value = timestamp.unwrap_or_default();
    add_psu_defaults(&mut psu, &name, files.len(), timestamp_value);
    add_files_to_psu(&mut psu, &files, timestamp)?;
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
            if f.file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.eq_ignore_ascii_case("psu.toml"))
                .unwrap_or(false)
            {
                None
            } else if !f.is_file() {
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

fn add_files_to_psu(
    psu: &mut PSU,
    files: &[PathBuf],
    timestamp: Option<NaiveDateTime>,
) -> Result<(), Error> {
    for file in files {
        let name = file.file_name().unwrap().to_str().unwrap();

        let f = std::fs::read(file)?;
        let (created, modified) = if let Some(timestamp) = timestamp {
            (timestamp, timestamp)
        } else {
            let stat = std::fs::metadata(file)?;
            (
                convert_timestamp(stat.created()?),
                convert_timestamp(stat.modified()?),
            )
        };

        println!("+ {} {}", "Adding", name.green());

        psu.entries.push(PSUEntry {
            id: FILE_ID,
            size: f.len() as u32,
            created,
            sector: 0,
            modified,
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
