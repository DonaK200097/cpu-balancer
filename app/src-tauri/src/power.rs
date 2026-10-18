//! Схемы питания и их настройки — напрямую через powrprof (без powercfg.exe).

use serde::Serialize;
use std::collections::HashMap;
use windows::core::GUID;
use windows::Win32::Foundation::{HLOCAL, LocalFree, ERROR_SUCCESS};
use windows::Win32::System::Power::{
    GetSystemPowerStatus, PowerEnumerate, PowerGetActiveScheme, PowerReadACValueIndex, PowerReadDCValueIndex,
    PowerReadFriendlyName, PowerSetActiveScheme, PowerWriteACValueIndex, PowerWriteDCValueIndex, ACCESS_SCHEME,
    SYSTEM_POWER_STATUS,
};

pub const SUB_PROCESSOR: &str = "54533251-82be-4824-96c1-47b60b740d00";
pub const SUB_PCIE: &str = "501a4d13-42af-4429-9fd1-a8218c268e20";

/// Все настройки, с которыми работает программа: ключ → (подгруппа, настройка). Порядок = порядок в бэкапах v1.
pub const SETTINGS: &[(&str, &str, &str)] = &[
    ("AUTON", SUB_PROCESSOR, "8baa4a8a-14c6-4451-8e8b-14bdbd197537"),
    ("EPP", SUB_PROCESSOR, "36687f9e-e3a5-4dbf-b1dc-15eb381c6863"),
    ("EPP1", SUB_PROCESSOR, "36687f9e-e3a5-4dbf-b1dc-15eb381c6864"),
    ("BOOST", SUB_PROCESSOR, "be337238-0d82-4146-a960-4f3749d470c7"),
    ("MAXSTATE", SUB_PROCESSOR, "bc5038f7-23e0-4960-96da-33abaf5935ec"),
    ("MINSTATE", SUB_PROCESSOR, "893dee8e-2bef-41e0-89c6-b55d0929964c"),
    ("MINSTATE1", SUB_PROCESSOR, "893dee8e-2bef-41e0-89c6-b55d0929964d"),
    ("FREQ", SUB_PROCESSOR, "75b0ae3f-bce0-45a7-8c89-c9611c25e100"),
    ("FREQ1", SUB_PROCESSOR, "75b0ae3f-bce0-45a7-8c89-c9611c25e101"),
    ("CPMIN", SUB_PROCESSOR, "0cc5b647-c1df-4637-891a-dec35c318583"),
    ("CPMIN1", SUB_PROCESSOR, "0cc5b647-c1df-4637-891a-dec35c318584"),
    ("SCHED", SUB_PROCESSOR, "93b8b6dc-0698-4d1c-9ee4-0644e900c85d"),
    ("SHORT", SUB_PROCESSOR, "bae08b81-2d5e-4688-ad6a-13243356654b"),
    ("HETERO", SUB_PROCESSOR, "7f2f5cfa-f10c-4823-b5e1-e93ae85f46b5"),
    ("LATPERF", SUB_PROCESSOR, "619b7505-003b-4e82-b7a6-4dd29c300971"),
    ("LATUNPARK", SUB_PROCESSOR, "616cdaa5-695e-4545-97ad-97dc2d1bdd88"),
    ("FLOORC0", SUB_PROCESSOR, "fddc842b-8364-4edc-94cf-c17f60de1c80"),
    ("INITC1", SUB_PROCESSOR, "1facfc65-a930-4bc5-9f38-504ec097bbc0"),
    ("IDLEPROMOTE", SUB_PROCESSOR, "7b224883-b3cc-4d79-819f-8374152cbe7c"),
    ("IDLESCALE", SUB_PROCESSOR, "6c2993b0-8f48-481f-bcc6-00dd2742aa06"),
    ("CONTPOL", SUB_PROCESSOR, "60fbe21b-efd9-49f2-b066-8674d8e9f423"),
    ("HYBCU", SUB_PROCESSOR, "12fd031f-53d2-4bf4-ac6d-c699fc9538c7"),
    ("HYBCC", SUB_PROCESSOR, "6788488b-1b90-4d11-8fa7-973e470dff47"),
    ("EFFCU", SUB_PROCESSOR, "6ece9e1f-b6dd-42bf-b1b7-5a512b10c092"),
    ("EFFCC", SUB_PROCESSOR, "69439b22-221b-4830-bd34-f7bcece24583"),
    ("CONTINC", SUB_PROCESSOR, "64fcee6b-5b1f-45a4-a76a-19b2c36ee290"),
    ("CONTDEC", SUB_PROCESSOR, "6ff13aeb-7897-4356-9999-dd9930af065f"),
    ("COOLING", SUB_PROCESSOR, "94d3a615-a899-4ac5-ae2b-e4d8f634367f"),
    ("ASPM", SUB_PCIE, "ee12f906-d277-404b-b6da-e5fa1a576df5"),
];

/// Значения схемы: ключ → [от сети, от батареи]. Нет ключа — настройки нет в системе.
pub type Values = HashMap<String, [i64; 2]>;

pub fn setting(key: &str) -> Option<(&'static str, &'static str)> {
    SETTINGS.iter().find(|s| s.0 == key).map(|s| (s.1, s.2))
}

pub fn key_by_setting_guid(guid: &str) -> Option<&'static str> {
    let g = guid.to_ascii_lowercase();
    SETTINGS.iter().find(|s| s.2 == g).map(|s| s.0)
}

pub fn parse_guid(s: &str) -> GUID {
    GUID::try_from(s).unwrap_or_else(|_| GUID::zeroed())
}

pub fn guid_str(g: &GUID) -> String {
    format!("{:?}", g).to_ascii_lowercase()
}

#[derive(Serialize, Clone, Debug)]
pub struct Scheme {
    pub guid: String,
    pub name: String,
    pub active: bool,
}

fn friendly_name(scheme: &GUID) -> String {
    unsafe {
        let mut size: u32 = 0;
        let _ = PowerReadFriendlyName(None, Some(scheme), None, None, None, &mut size);
        if size == 0 {
            return guid_str(scheme);
        }
        let mut buf = vec![0u8; size as usize];
        if PowerReadFriendlyName(None, Some(scheme), None, None, Some(buf.as_mut_ptr()), &mut size) != ERROR_SUCCESS {
            return guid_str(scheme);
        }
        let wide: Vec<u16> = buf.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        let end = wide.iter().position(|&c| c == 0).unwrap_or(wide.len());
        String::from_utf16_lossy(&wide[..end])
    }
}

pub fn active_scheme() -> Option<GUID> {
    unsafe {
        let mut p: *mut GUID = std::ptr::null_mut();
        if PowerGetActiveScheme(None, &mut p) != ERROR_SUCCESS || p.is_null() {
            return None;
        }
        let g = *p;
        let _ = LocalFree(Some(HLOCAL(p as _)));
        Some(g)
    }
}

pub fn list_schemes() -> Vec<Scheme> {
    let active = active_scheme();
    let mut out = Vec::new();
    unsafe {
        let mut i = 0u32;
        loop {
            let mut g = GUID::zeroed();
            let mut size = std::mem::size_of::<GUID>() as u32;
            let r = PowerEnumerate(None, None, None, ACCESS_SCHEME, i, Some(&mut g as *mut GUID as *mut u8), &mut size);
            if r != ERROR_SUCCESS {
                break;
            }
            out.push(Scheme { guid: guid_str(&g), name: friendly_name(&g), active: Some(g) == active });
            i += 1;
        }
    }
    out
}

pub fn set_active(scheme: &str) -> bool {
    unsafe { PowerSetActiveScheme(None, Some(&parse_guid(scheme))) == ERROR_SUCCESS }
}

/// Читает все известные настройки схемы. Отсутствующие в системе не попадают в результат.
pub fn read_scheme(scheme: &str) -> Values {
    let sg = parse_guid(scheme);
    let mut vals = Values::new();
    for (key, sub, set) in SETTINGS {
        let (sub, set) = (parse_guid(sub), parse_guid(set));
        let mut ac = 0u32;
        let mut dc = 0u32;
        unsafe {
            if PowerReadACValueIndex(None, Some(&sg), Some(&sub), Some(&set), &mut ac) != ERROR_SUCCESS {
                continue;
            }
            if PowerReadDCValueIndex(None, Some(&sg), Some(&sub), Some(&set), &mut dc) != 0 {
                continue;
            }
        }
        vals.insert(key.to_string(), [ac as i64, dc as i64]);
    }
    vals
}

/// Пишет одно значение (col 0 = от сети, 1 = от батареи).
pub fn write_value(scheme: &str, key: &str, col: usize, value: i64) -> bool {
    let Some((sub, set)) = setting(key) else { return false };
    let (sg, sub, set) = (parse_guid(scheme), parse_guid(sub), parse_guid(set));
    let v = value.clamp(0, u32::MAX as i64) as u32;
    let r = unsafe {
        if col == 0 {
            PowerWriteACValueIndex(None, &sg, Some(&sub), Some(&set), v).0
        } else {
            PowerWriteDCValueIndex(None, &sg, Some(&sub), Some(&set), v)
        }
    };
    r == 0
}

/// Сейчас работает от сети? Именно ACLineStatus — по нему Windows выбирает AC/DC-значения
/// (батарея может разряжаться при подключённой зарядке USB-C PD, но значения при этом «от сети»).
pub fn on_ac() -> bool {
    let mut st = SYSTEM_POWER_STATUS::default();
    unsafe {
        if GetSystemPowerStatus(&mut st).is_err() {
            return true;
        }
    }
    st.ACLineStatus != 0
}
