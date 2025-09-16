//! Shared configuration templates bundled with the workspace.

/// Template `title.cfg` file with the mandatory keys pre-populated.
pub const TITLE_CFG_TEMPLATE: &str = include_str!("../../../assets/templates/title.cfg");

/// Template `psu.toml` file with minimal project metadata.
pub const PSU_TOML_TEMPLATE: &str = include_str!("../../../assets/templates/psu.toml");
