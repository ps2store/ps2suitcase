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
        self.missing_mandatory_fields().is_empty()
    }

    pub fn missing_mandatory_fields(&self) -> Vec<&'static str> {
        MANDATORY_KEYS
            .iter()
            .copied()
            .filter(|key| !self.index_map.contains_key(*key))
            .collect()
    }

    pub fn add_missing_fields(&mut self) -> &Self {
        for (_, key) in MANDATORY_KEYS.iter().enumerate() {
            if !self.index_map.contains_key(*key) {
                self.index_map.insert(key.to_string(), "".to_string());
            }
        }
        self
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
        if let Some((key, value)) = line.split_once('=') {
            index_map.insert(key.to_string(), value.to_string());
        }
    }

    index_map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_to_index_map_skips_lines_without_delimiter() {
        let contents = "title=Example Game\ninvalid_line\ndeveloper=Example Dev";

        let map = string_to_index_map(contents.to_string());

        assert_eq!(map.get("title"), Some(&"Example Game".to_string()));
        assert_eq!(map.get("developer"), Some(&"Example Dev".to_string()));
        assert!(!map.contains_key("invalid_line"));
    }

    #[test]
    fn title_cfg_handles_malformed_lines_gracefully() {
        let contents = "title=Another Game\njust_text\nboot=cdrom0:\\SLUS_123.45";

        let cfg = TitleCfg::new(contents.to_string());

        assert_eq!(
            cfg.index_map.get("title"),
            Some(&"Another Game".to_string())
        );
        assert_eq!(
            cfg.index_map.get("boot"),
            Some(&"cdrom0:\\SLUS_123.45".to_string())
        );
        assert!(!cfg.index_map.contains_key("just_text"));
    }

    #[test]
    fn reports_missing_mandatory_fields() {
        let contents = "title=Example\nDeveloper=Someone";

        let cfg = TitleCfg::new(contents.to_string());
        let mut missing = cfg.missing_mandatory_fields();
        missing.sort();

        assert!(missing.contains(&"Description"));
        assert!(missing.contains(&"Release"));
        assert!(missing.contains(&"source"));
        assert!(!cfg.has_mandatory_fields());
    }
}
