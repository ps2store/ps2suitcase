use std::fs;
use std::path::Path;

use ps2_filetypes::{PSUEntryKind, PSU};
use psu_packer::{pack_with_config, Config};
use tempfile::tempdir;

fn write_file(path: &Path, contents: &[u8]) {
    fs::write(path, contents).expect("write file");
}

fn packed_psu_contains_file(output: &Path, name: &str) -> bool {
    let data = fs::read(output).expect("read packed psu");
    let archive = PSU::new(data);
    archive.entries.iter().any(|entry| {
        matches!(entry.kind, PSUEntryKind::File) && entry.name.eq_ignore_ascii_case(name)
    })
}

#[test]
fn psu_toml_is_never_packed() {
    let workspace = tempdir().expect("temp dir");
    let project = workspace.path();

    write_file(&project.join("DATA.BIN"), b"payload");
    write_file(&project.join("psu.toml"), b"[config]\nname = \"Test\"\n");

    let config_include_all = Config {
        name: "Test Save".to_string(),
        timestamp: None,
        include: None,
        exclude: None,
        icon_sys: None,
    };
    let output_include_all = project.join("include-all.psu");
    pack_with_config(project, &output_include_all, config_include_all)
        .expect("pack with automatic include");

    assert!(
        packed_psu_contains_file(&output_include_all, "DATA.BIN"),
        "expected data file to be present"
    );
    assert!(
        !packed_psu_contains_file(&output_include_all, "psu.toml"),
        "psu.toml should always be omitted"
    );

    let config_with_explicit_include = Config {
        name: "Test Save".to_string(),
        timestamp: None,
        include: Some(vec!["DATA.BIN".to_string(), "psu.toml".to_string()]),
        exclude: None,
        icon_sys: None,
    };
    let output_with_explicit = project.join("explicit.psu");
    pack_with_config(project, &output_with_explicit, config_with_explicit_include)
        .expect("pack with explicit include");

    assert!(
        !packed_psu_contains_file(&output_with_explicit, "psu.toml"),
        "psu.toml should be filtered even when explicitly included"
    );
}
