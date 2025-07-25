use indexmap::IndexMap;
use std::fmt::{Display, Formatter};
use std::string::ToString;
use toml::Table;

const MANDATORY_KEYS: &[&str] = &[
    "title",
    "Description",
    "boot",
    "Release",
    "Developer",
    "source",
    "Version",
];

pub struct TitleCfg {
    pub contents: String,
    pub index_map: IndexMap<String, String>,
    pub helper: Table,
}

impl TitleCfg {
    pub fn new(contents: String) -> Self {
        let index_map = string_to_index_map(contents.clone());

        let helper = include_str!("../../title_cfg.toml")
            .parse::<Table>()
            .expect("Failed to parse title_cfg helper to toml");

        Self {
            contents,
            index_map,
            helper,
        }
    }

    pub fn sync_index_map_to_contents(&mut self) {
        self.contents = self.to_string();
    }

    pub fn sync_contents_to_index_map(&mut self) {
        self.index_map = string_to_index_map(self.contents.clone());
    }

    pub fn has_mandatory_fields(&self) -> bool {
        for (_, key) in MANDATORY_KEYS.iter().enumerate() {
            if !self.index_map.contains_key(key.to_owned()) {
                return false;
            }
        }
        true
    }

    pub fn fix_missing_fields(&mut self) {
        for (_, key) in MANDATORY_KEYS.iter().enumerate() {
            if !self.index_map.contains_key(key.to_owned()) {
                self.index_map.insert(key.to_string(), "".to_string());
            }
        }
    }
}

impl Display for TitleCfg {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut contents: String = "".to_string();
        for (key, value) in self.index_map.iter() {
            contents.push_str(format!("{key}={value}\n").to_owned().as_str());
        }
        write!(f, "{contents}")
    }
}

fn string_to_index_map(contents: String) -> IndexMap<String, String> {
    let mut index_map: IndexMap<String, String> = IndexMap::new();

    let lines = contents.lines();
    for line in lines {
        let pair = line.split('=').collect::<Vec<&str>>();
        index_map.insert(pair[0].to_string(), pair[1].to_string());
    }

    index_map
}
