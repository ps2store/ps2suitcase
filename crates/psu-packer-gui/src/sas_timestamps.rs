use std::{collections::HashSet, path::Path};

use chrono::{
    DateTime, Duration, Local, LocalResult, NaiveDate, NaiveDateTime, NaiveTime, TimeZone,
    Timelike, Utc,
};
use serde::{Deserialize, Serialize};

const CHARSET: &str = " 0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ_-.";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct TimestampRules {
    #[serde(default = "TimestampRules::default_seconds_between_items")]
    pub(crate) seconds_between_items: u32,
    #[serde(default = "TimestampRules::default_slots_per_category")]
    pub(crate) slots_per_category: u32,
    #[serde(default = "TimestampRules::default_categories")]
    pub(crate) categories: Vec<CategoryRule>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct CategoryRule {
    pub(crate) key: String,
    #[serde(default)]
    pub(crate) aliases: Vec<String>,
}

impl CategoryRule {
    fn new(key: &'static str) -> Self {
        Self {
            key: key.to_string(),
            aliases: Vec::new(),
        }
    }

    fn with_aliases(mut self, aliases: &'static [&'static str]) -> Self {
        self.aliases = aliases.iter().map(|alias| alias.to_string()).collect();
        self
    }
}

impl TimestampRules {
    const fn default_seconds_between_items() -> u32 {
        2
    }

    const fn default_slots_per_category() -> u32 {
        86_400
    }

    fn default_categories() -> Vec<CategoryRule> {
        vec![
            CategoryRule::new("APP_").with_aliases(&["OSDXMB", "XEBPLUS"]),
            CategoryRule::new("APPS"),
            CategoryRule::new("PS1_"),
            CategoryRule::new("EMU_"),
            CategoryRule::new("GME_"),
            CategoryRule::new("DST_"),
            CategoryRule::new("DBG_"),
            CategoryRule::new("RAA_").with_aliases(&["RESTART", "POWEROFF"]),
            CategoryRule::new("RTE_").with_aliases(&["NEUTRINO"]),
            CategoryRule::new("DEFAULT"),
            CategoryRule::new("SYS_").with_aliases(&["BOOT"]),
            CategoryRule::new("ZZY_").with_aliases(&["EXPLOITS"]),
            CategoryRule::new("ZZZ_").with_aliases(&["BM", "MATRIXTEAM", "OPL"]),
        ]
    }

    pub(crate) fn sanitize(&mut self) {
        if self.seconds_between_items == 0 {
            self.seconds_between_items = Self::default_seconds_between_items();
        }
        if self.slots_per_category == 0 {
            self.slots_per_category = Self::default_slots_per_category();
        }

        if self.categories.is_empty() {
            *self = Self::default();
            return;
        }

        let mut sanitized = Vec::with_capacity(self.categories.len());
        let mut seen_keys: HashSet<String> = HashSet::new();

        for category in self.categories.drain(..) {
            let key = category.key.trim().to_ascii_uppercase();
            if key.is_empty() {
                continue;
            }
            if !seen_keys.insert(key.clone()) {
                continue;
            }

            let mut aliases: Vec<String> = category
                .aliases
                .into_iter()
                .filter_map(|alias| sanitize_alias(alias, &key))
                .collect();

            let mut seen_aliases = HashSet::new();
            aliases.retain(|alias| seen_aliases.insert(alias.clone()));

            sanitized.push(CategoryRule { key, aliases });
        }

        if !sanitized.iter().any(|category| category.key == "DEFAULT") {
            sanitized.push(CategoryRule {
                key: "DEFAULT".to_string(),
                aliases: Vec::new(),
            });
        }

        self.categories = sanitized;
    }

    pub(crate) fn seconds_between_items_i64(&self) -> i64 {
        i64::from(self.seconds_between_items)
    }

    pub(crate) fn slots_per_category_i64(&self) -> i64 {
        i64::from(self.slots_per_category)
    }
}

impl Default for TimestampRules {
    fn default() -> Self {
        Self {
            seconds_between_items: Self::default_seconds_between_items(),
            slots_per_category: Self::default_slots_per_category(),
            categories: Self::default_categories(),
        }
    }
}

fn sanitize_alias(alias: String, key: &str) -> Option<String> {
    let mut value = alias.trim().to_ascii_uppercase();
    if value.is_empty() {
        return None;
    }

    if key != "APPS" && key != "DEFAULT" && value.starts_with(key) {
        value = value[key.len()..].to_string();
    }

    if value.is_empty() {
        return None;
    }

    Some(value)
}

pub(crate) fn planned_timestamp_for_folder(
    path: &Path,
    rules: &TimestampRules,
) -> Option<NaiveDateTime> {
    let name = path.file_name()?.to_str()?;
    planned_timestamp_for_name(name, rules)
}

pub(crate) fn planned_timestamp_for_name(
    name: &str,
    rules: &TimestampRules,
) -> Option<NaiveDateTime> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return None;
    }

    let offset_seconds = deterministic_offset_seconds(trimmed, rules)?;
    let base = base_datetime_local_to_utc()?;
    let planned_utc = base - Duration::seconds(offset_seconds);
    let snapped = snap_even_second(planned_utc);
    let local = snapped.with_timezone(&Local);
    Some(local.naive_local())
}

fn deterministic_offset_seconds(name: &str, rules: &TimestampRules) -> Option<i64> {
    let effective = normalize_name_for_rules(name, rules)?;
    let category_index = category_priority_index(&effective, rules)?;
    let slot = slot_index_within_category(&effective, rules);
    let category_block_seconds = rules.seconds_between_items_i64() * rules.slots_per_category_i64();
    let category_offset = category_index as i64 * category_block_seconds;
    let name_offset = slot * rules.seconds_between_items_i64();
    Some(category_offset + name_offset)
}

fn normalize_name_for_rules(name: &str, rules: &TimestampRules) -> Option<String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return None;
    }

    let upper = trimmed.to_ascii_uppercase();

    for category in &rules.categories {
        if category.aliases.iter().any(|alias| *alias == upper) {
            return Some(match category.key.as_str() {
                "APPS" => String::from("APPS"),
                "DEFAULT" => upper,
                key => format!("{key}{upper}"),
            });
        }
    }

    Some(upper)
}

fn category_priority_index(effective: &str, rules: &TimestampRules) -> Option<usize> {
    find_category(effective, rules).map(|(index, _)| index)
}

fn find_category<'a>(
    effective: &str,
    rules: &'a TimestampRules,
) -> Option<(usize, &'a CategoryRule)> {
    let mut fallback: Option<(usize, &'a CategoryRule)> = None;

    for (index, category) in rules.categories.iter().enumerate() {
        match category.key.as_str() {
            "DEFAULT" => fallback = Some((index, category)),
            "APPS" => {
                if effective == "APPS" {
                    return Some((index, category));
                }
            }
            key => {
                if effective.starts_with(key) {
                    return Some((index, category));
                }
            }
        }
    }

    fallback
}

fn slot_index_within_category(effective: &str, rules: &TimestampRules) -> i64 {
    let payload = payload_for_effective(effective, rules);

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

    let slots_per_category = rules.slots_per_category_i64();
    let mut slot = (total * slots_per_category as f64).floor() as i64;
    if slot >= slots_per_category {
        slot = slots_per_category - 1;
    }
    slot
}

fn payload_for_effective(effective: &str, rules: &TimestampRules) -> String {
    if let Some((_, category)) = find_category(effective, rules) {
        match category.key.as_str() {
            "APPS" => "APPS".to_string(),
            "DEFAULT" => effective.replace('-', ""),
            key => effective
                .strip_prefix(key)
                .unwrap_or(effective)
                .replace('-', ""),
        }
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
        let rules = TimestampRules::default();
        let timestamp = planned_timestamp_for_folder(&path, &rules).expect("timestamp");
        assert_eq!(timestamp.second() % 2, 0);
        assert_eq!(timestamp.nanosecond(), 0);
    }

    #[test]
    fn handles_aliases() {
        let mut rules = TimestampRules::default();
        rules.sanitize();
        let path = PathBuf::from("boot");
        let ts_boot = planned_timestamp_for_folder(&path, &rules).expect("timestamp");
        let sys_path = PathBuf::from("SYS_BOOT");
        let ts_sys = planned_timestamp_for_folder(&sys_path, &rules).expect("timestamp");
        assert_eq!(ts_boot, ts_sys);
    }

    #[test]
    fn custom_aliases_match_prefixed_names() {
        let mut rules = TimestampRules::default();
        if let Some(category) = rules
            .categories
            .iter_mut()
            .find(|category| category.key == "APP_")
        {
            category.aliases.push("CUSTOM".to_string());
        }
        rules.sanitize();

        let alias_path = PathBuf::from("custom");
        let prefixed_path = PathBuf::from("APP_CUSTOM");

        let alias_timestamp =
            planned_timestamp_for_folder(&alias_path, &rules).expect("alias timestamp");
        let prefixed_timestamp =
            planned_timestamp_for_folder(&prefixed_path, &rules).expect("prefixed timestamp");

        assert_eq!(alias_timestamp, prefixed_timestamp);
    }
}
