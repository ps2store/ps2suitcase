use std::fs;
use std::path::Path;

use chrono::{NaiveDate, NaiveDateTime};
use ps2_filetypes::{PSUEntryKind, PSU};
use psu_packer::{pack_with_config, Config};
use tempfile::tempdir;

fn create_sample_file(path: &Path) {
    fs::write(path, b"example").expect("write sample file");
}

#[test]
fn pack_with_or_without_timestamp_controls_entry_times() {
    let tempdir = tempdir().expect("temp dir");
    let folder = tempdir.path();
    let source_file = folder.join("DATA.BIN");
    create_sample_file(&source_file);
    let output_dir = folder.join("output");
    fs::create_dir(&output_dir).expect("create output dir");

    let timestamp = NaiveDate::from_ymd_opt(2024, 1, 2)
        .unwrap()
        .and_hms_opt(3, 4, 5)
        .unwrap();
    let config_with_timestamp = Config {
        name: "Test Save".to_string(),
        timestamp: Some(timestamp),
        include: None,
        exclude: None,
        icon_sys: None,
    };
    let output_with_timestamp = output_dir.join("with-timestamp.psu");
    pack_with_config(folder, &output_with_timestamp, config_with_timestamp)
        .expect("pack with timestamp");

    let packed_with_timestamp = PSU::new(fs::read(&output_with_timestamp).expect("read output"));
    for entry in packed_with_timestamp.entries.iter() {
        assert_eq!(
            entry.created, timestamp,
            "created timestamp should match config"
        );
        assert_eq!(
            entry.modified, timestamp,
            "modified timestamp should match config"
        );
    }

    // Legacy behaviour: omit the timestamp and expect filesystem metadata to be used for files.
    let output_without_timestamp = output_dir.join("without-timestamp.psu");
    let legacy_config = Config {
        name: "Test Save".to_string(),
        timestamp: None,
        include: None,
        exclude: None,
        icon_sys: None,
    };
    pack_with_config(folder, &output_without_timestamp, legacy_config)
        .expect("pack without timestamp");

    let packed_without_timestamp =
        PSU::new(fs::read(&output_without_timestamp).expect("read output"));
    let mut file_timestamp = None;
    for entry in packed_without_timestamp.entries.iter() {
        match entry.kind {
            PSUEntryKind::Directory => {
                assert_eq!(entry.created, NaiveDateTime::default());
                assert_eq!(entry.modified, NaiveDateTime::default());
            }
            PSUEntryKind::File => {
                file_timestamp = Some(entry.created);
            }
        }
    }

    let file_timestamp = file_timestamp.expect("file entry present");
    assert_ne!(file_timestamp, NaiveDateTime::default());
}
