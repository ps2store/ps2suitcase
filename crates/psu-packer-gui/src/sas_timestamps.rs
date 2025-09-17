use std::path::Path;

use chrono::{
    DateTime, Duration, Local, LocalResult, NaiveDate, NaiveDateTime, NaiveTime, TimeZone,
    Timelike, Utc,
};

const SECONDS_BETWEEN_ITEMS: i64 = 2;
const SLOTS_PER_CATEGORY: i64 = 86_400;
const CATEGORY_ORDER: [&str; 13] = [
    "APP_", "APPS", "PS1_", "EMU_", "GME_", "DST_", "DBG_", "RAA_", "RTE_", "DEFAULT", "SYS_",
    "ZZY_", "ZZZ_",
];
const CATEGORY_BLOCK_SECONDS: i64 = SECONDS_BETWEEN_ITEMS * SLOTS_PER_CATEGORY;
const CHARSET: &str = " 0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ_-.";

const UNPREFIXED_MAP: [(&str, &[&str]); 12] = [
    ("APP_", &["OSDXMB", "XEBPLUS"]),
    ("APPS", &[]),
    ("PS1_", &[]),
    ("EMU_", &[]),
    ("GME_", &[]),
    ("DST_", &[]),
    ("DBG_", &[]),
    ("RAA_", &["RESTART", "POWEROFF"]),
    ("RTE_", &["NEUTRINO"]),
    ("SYS_", &["BOOT"]),
    ("ZZY_", &["EXPLOITS"]),
    ("ZZZ_", &["BM", "MATRIXTEAM", "OPL"]),
];

pub(crate) fn planned_timestamp_for_folder(path: &Path) -> Option<NaiveDateTime> {
    let name = path.file_name()?.to_str()?.trim();
    if name.is_empty() {
        return None;
    }

    let offset_seconds = deterministic_offset_seconds(name)?;
    let base = base_datetime_local_to_utc()?;
    let planned_utc = base - Duration::seconds(offset_seconds);
    let snapped = snap_even_second(planned_utc);
    let local = snapped.with_timezone(&Local);
    Some(local.naive_local())
}

fn deterministic_offset_seconds(name: &str) -> Option<i64> {
    let effective = normalize_name_for_rules(name)?;
    let category_index = category_priority_index(&effective)?;
    let slot = slot_index_within_category(&effective);
    let category_offset = category_index as i64 * CATEGORY_BLOCK_SECONDS;
    let name_offset = slot * SECONDS_BETWEEN_ITEMS;
    Some(category_offset + name_offset)
}

fn normalize_name_for_rules(name: &str) -> Option<String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return None;
    }

    let upper = trimmed.to_ascii_uppercase();

    for (category, entries) in UNPREFIXED_MAP.iter() {
        if entries.iter().any(|entry| *entry == upper) {
            if *category == "APPS" {
                return Some(String::from("APPS"));
            }
            return Some(format!("{}{}", category, upper));
        }
    }

    match upper.as_str() {
        "OSDXMB" | "XEBPLUS" => Some(format!("APP_{}", upper)),
        "RESTART" | "POWEROFF" => Some(format!("RAA_{}", upper)),
        "NEUTRINO" => Some(format!("RTE_{}", upper)),
        "BOOT" => Some(String::from("SYS_BOOT")),
        "EXPLOITS" => Some(String::from("ZZY_EXPLOITS")),
        "BM" | "MATRIXTEAM" | "OPL" => Some(format!("ZZZ_{}", upper)),
        _ => Some(upper),
    }
}

fn category_priority_index(effective: &str) -> Option<usize> {
    let key = effective_category_key(effective);
    CATEGORY_ORDER
        .iter()
        .position(|candidate| *candidate == key)
}

fn effective_category_key(effective: &str) -> &str {
    if effective.starts_with("APP_") {
        "APP_"
    } else if effective == "APPS" {
        "APPS"
    } else if effective.starts_with("PS1_") {
        "PS1_"
    } else if effective.starts_with("EMU_") {
        "EMU_"
    } else if effective.starts_with("GME_") {
        "GME_"
    } else if effective.starts_with("DST_") {
        "DST_"
    } else if effective.starts_with("DBG_") {
        "DBG_"
    } else if effective.starts_with("RAA_") {
        "RAA_"
    } else if effective.starts_with("RTE_") {
        "RTE_"
    } else if effective.starts_with("SYS_") || effective == "SYS" {
        "SYS_"
    } else if effective.starts_with("ZZY_") {
        "ZZY_"
    } else if effective.starts_with("ZZZ_") {
        "ZZZ_"
    } else {
        "DEFAULT"
    }
}

fn slot_index_within_category(effective: &str) -> i64 {
    let payload = payload_for_effective(effective);

    let mut total = 0.0f64;
    let mut scale = 1.0f64;

    for ch in payload.chars().take(128) {
        scale *= CHARSET.len() as f64;
        let index = match CHARSET.find(ch.to_ascii_uppercase()) {
            Some(idx) => idx + 1,
            None => CHARSET.len(),
        } as f64;
        total += index / scale;
    }

    let mut slot = (total * SLOTS_PER_CATEGORY as f64).floor() as i64;
    if slot >= SLOTS_PER_CATEGORY {
        slot = SLOTS_PER_CATEGORY - 1;
    }
    slot
}

fn payload_for_effective(effective: &str) -> String {
    let key = effective_category_key(effective);
    if key == "APPS" {
        "APPS".to_string()
    } else if key == "DEFAULT" {
        effective.replace('-', "")
    } else if let Some(stripped) = effective.strip_prefix(key) {
        stripped.replace('-', "")
    } else {
        effective.replace('-', "")
    }
}

fn base_datetime_local_to_utc() -> Option<DateTime<Utc>> {
    let date = NaiveDate::from_ymd_opt(2098, 12, 31)?;
    let time = NaiveTime::from_hms_opt(23, 59, 59)?;
    let naive = NaiveDateTime::new(date, time);

    let local = match Local.from_local_datetime(&naive) {
        LocalResult::Single(dt) => dt,
        LocalResult::Ambiguous(dt, alt) => dt.min(alt),
        LocalResult::None => return None,
    };

    Some(local.with_timezone(&Utc))
}

fn snap_even_second(dt: DateTime<Utc>) -> DateTime<Utc> {
    let mut snapped = dt.with_nanosecond(0).unwrap_or(dt);
    if snapped.second() % 2 == 1 {
        snapped += Duration::seconds(1);
    }
    snapped
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn produces_even_seconds() {
        let path = PathBuf::from("APP_SAMPLE");
        let timestamp = planned_timestamp_for_folder(&path).expect("timestamp");
        assert_eq!(timestamp.second() % 2, 0);
        assert_eq!(timestamp.nanosecond(), 0);
    }

    #[test]
    fn handles_aliases() {
        let path = PathBuf::from("boot");
        let ts_boot = planned_timestamp_for_folder(&path).expect("timestamp");
        let sys_path = PathBuf::from("SYS_BOOT");
        let ts_sys = planned_timestamp_for_folder(&sys_path).expect("timestamp");
        assert_eq!(ts_boot, ts_sys);
    }
}
