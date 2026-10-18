//! Настройки программы (settings.json рядом с данными).

use crate::paths;
use serde::{Deserialize, Serialize};
use windows::Win32::Globalization::GetUserDefaultUILanguage;

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Settings {
    /// "ru" | "en" | "" (= язык системы)
    pub lang: String,
    pub paused: bool,
    pub check_updates: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings { lang: String::new(), paused: false, check_updates: true }
    }
}

impl Settings {
    pub fn load() -> Settings {
        paths::read_text(&paths::file("settings.json")).and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
    }

    pub fn save(&self) {
        let _ = paths::write_text(&paths::file("settings.json"), &serde_json::to_string_pretty(self).unwrap_or_default());
    }
}

pub fn system_lang() -> &'static str {
    // LANG_RUSSIAN = 0x19 (младшие 10 бит — основной язык)
    if unsafe { GetUserDefaultUILanguage() } & 0x3FF == 0x19 {
        "ru"
    } else {
        "en"
    }
}

pub fn effective_lang(s: &Settings) -> String {
    match s.lang.as_str() {
        "ru" | "en" => s.lang.clone(),
        _ => system_lang().to_string(),
    }
}

/// Акцентный цвет Windows (#rrggbb) из DWM, если доступен.
pub fn accent_color() -> Option<String> {
    use windows::core::w;
    use windows::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};
    let mut v: u32 = 0;
    let mut size = 4u32;
    let r = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            w!("Software\\Microsoft\\Windows\\DWM"),
            w!("AccentColor"),
            RRF_RT_REG_DWORD,
            None,
            Some(&mut v as *mut u32 as _),
            Some(&mut size),
        )
    };
    if r.is_err() {
        return None;
    }
    // ABGR
    let (r, g, b) = (v & 0xFF, (v >> 8) & 0xFF, (v >> 16) & 0xFF);
    Some(format!("#{r:02x}{g:02x}{b:02x}"))
}
