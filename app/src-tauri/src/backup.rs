//! Исходные значения схем (совместимо с v1).
//!
//! Приоритет источников: существующий `backups\<guid>.json` → `g16-power-backup.json`
//! (самый ранний) → `Откат_<имя схемы>.cmd` → текущие значения.
//! Существующий файл не перезаписывается, недостающие ключи дописываются.

use crate::paths;
use crate::power::{self, Values, SETTINGS};
use serde_json::{json, Map, Value};
use std::collections::HashMap;

fn backup_file(scheme: &str) -> std::path::PathBuf {
    paths::data_dir().join("backups").join(format!("{scheme}.json"))
}

fn as_i64(v: &Value) -> Option<i64> {
    v.as_i64().or_else(|| v.as_f64().map(|f| f as i64)).or_else(|| v.as_str().and_then(|s| s.trim().parse().ok()))
}

/// Имя схемы → безопасное имя файла (как в v1).
fn safe_name(name: &str) -> String {
    name.chars().map(|c| if "\\/:*?\"<>|".contains(c) { '_' } else { c }).collect::<String>().trim().to_string()
}

/// Строки `powercfg /set(ac|dc)valueindex <scheme> <sub> <set> <value>` из отката CMD-профилей.
fn from_rollback(scheme: &str, name: &str) -> HashMap<String, [Option<i64>; 2]> {
    let mut out: HashMap<String, [Option<i64>; 2]> = HashMap::new();
    let path = paths::file(&format!("Откат_{}.cmd", safe_name(name)));
    let Some(text) = paths::read_text(&path) else { return out };
    for line in text.lines() {
        let lower = line.to_ascii_lowercase();
        let Some(pos) = lower.find("powercfg /set") else { continue };
        let rest = &lower[pos + "powercfg /set".len()..];
        let col = if rest.starts_with("acvalueindex") {
            0
        } else if rest.starts_with("dcvalueindex") {
            1
        } else {
            continue;
        };
        let parts: Vec<&str> = rest.split_whitespace().collect();
        // [acvalueindex, scheme, sub, set, value]
        if parts.len() < 5 || parts[1] != scheme {
            continue;
        }
        let Some(key) = power::key_by_setting_guid(parts[3]) else { continue };
        let Ok(v) = parts[4].parse::<i64>() else { continue };
        out.entry(key.to_string()).or_insert([None, None])[col] = Some(v);
    }
    out
}

/// Самый ранний бэкап v0: массив `{Scheme, Name, Sub, Set, AC, DC}`.
fn from_g16(scheme: &str) -> HashMap<String, [Option<i64>; 2]> {
    let mut out = HashMap::new();
    let Some(text) = paths::read_text(&paths::file("g16-power-backup.json")) else { return out };
    let Ok(Value::Array(rows)) = serde_json::from_str::<Value>(&text) else { return out };
    for r in rows {
        if r.get("Scheme").and_then(Value::as_str).map(str::to_ascii_lowercase).as_deref() != Some(scheme) {
            continue;
        }
        let Some(key) = r.get("Set").and_then(Value::as_str).and_then(power::key_by_setting_guid) else { continue };
        let (Some(ac), Some(dc)) = (r.get("AC").and_then(as_i64), r.get("DC").and_then(as_i64)) else { continue };
        out.insert(key.to_string(), [Some(ac), Some(dc)]);
    }
    out
}

/// Загружает (и при необходимости создаёт/дополняет) бэкап схемы. `write = false` — только прочитать.
pub fn get_backup(scheme: &str, name: &str, cur: &Values, write: bool) -> Values {
    let file = backup_file(scheme);
    let mut bk = Values::new();
    let exists = file.exists();
    if let Some(text) = paths::read_text(&file) {
        if let Ok(j) = serde_json::from_str::<Value>(&text) {
            if let Some(vals) = j.get("Values").and_then(Value::as_object) {
                for (k, v) in vals {
                    if let Some(arr) = v.as_array() {
                        if let (Some(ac), Some(dc)) = (arr.first().and_then(as_i64), arr.get(1).and_then(as_i64)) {
                            bk.insert(k.clone(), [ac, dc]);
                        }
                    }
                }
            }
        }
    }
    let mut changed = !exists;
    // g16-power-backup.json приоритетнее отката CMD-профилей
    let mut older = from_rollback(scheme, name);
    older.extend(from_g16(scheme));
    for (k, v) in cur {
        if bk.contains_key(k) {
            continue;
        }
        let mut v = *v;
        if let Some(o) = older.get(k) {
            for c in 0..2 {
                if let Some(x) = o[c] {
                    v[c] = x;
                }
            }
        }
        bk.insert(k.clone(), v);
        changed = true;
    }
    if changed && write {
        let mut o = Map::new();
        for (k, _, _) in SETTINGS {
            if let Some(v) = bk.get(*k) {
                o.insert(k.to_string(), json!([v[0], v[1]]));
            }
        }
        // ключи, которых нет в списке v2 (если появятся в будущем), не теряем
        for (k, v) in &bk {
            if !o.contains_key(k) {
                o.insert(k.clone(), json!([v[0], v[1]]));
            }
        }
        let doc = json!({ "Scheme": scheme, "Name": name, "Values": Value::Object(o) });
        let _ = paths::write_text(&file, &serde_json::to_string_pretty(&doc).unwrap_or_default());
    }
    bk
}
