use std::string::ToString;
use indexmap::IndexMap;
use toml::Table;

const MANDATORY_KEYS: &'static [&'static str] = &[
    "title",
    "Description",
    "boot",
    "Release",
    "Developer",
    "source",
    "Version",
];

pub struct TitleCfg {
    pub index_map: IndexMap<String, String>,
    pub helper: Table,
}

impl TitleCfg {
    pub fn new(contents: String) -> Self {
        let mut index_map: IndexMap<String, String> = IndexMap::new();

        let lines = contents.lines();
        for line in lines {
            let pair = line.split('=').collect::<Vec<&str>>();
            index_map.insert(pair[0].to_string(), pair[1].to_string());
        }

        let helper = include_str!("../../title_cfg.toml")
            .parse::<Table>()
            .expect("Failed to parse title_cfg helper to toml");

        Self { index_map, helper }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut contents: String = "".to_string();
        for (key, value) in self.index_map.iter() {
            contents.push_str(format!("{key}={value}\n").to_owned().as_str());
        }
        contents.into_bytes()
    }
    
    pub fn has_mandatory_fields(&self) -> bool {
        for (_, key) in MANDATORY_KEYS.iter().enumerate() {
            if !self.index_map.contains_key(key.to_owned()) {
                return false
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

