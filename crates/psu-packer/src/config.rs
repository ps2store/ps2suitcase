use serde::Deserialize;
use toml::value::Datetime;

#[derive(Debug, Deserialize)]
pub(crate) struct PsuConfig {
    pub(crate) name: String,
    pub(crate) files: Vec<String>,
    pub(crate) output: Option<String>,
    pub(crate) timestamp: Option<Datetime>,
}

impl std::fmt::Display for PsuConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}\n{:?}\n{:?}\n{:?}",
            self.name, self.files, self.output, self.timestamp
        )
    }
}
