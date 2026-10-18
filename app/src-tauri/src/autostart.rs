//! Автозапуск при входе в систему с правами администратора — задача Планировщика
//! (ключ Run не умеет повышать права без UAC). Статус кешируется: schtasks медленный.

use crate::paths;
use serde::Serialize;
use std::os::windows::process::CommandExt;
use std::process::{Command, Stdio};
use std::sync::Mutex;

pub const TASK_V2: &str = "CPU Balancer";
pub const TASK_V1: &str = "CPU Balancer - фон на E-ядрах";
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Serialize, Clone, Copy, Default)]
pub struct Status {
    pub known: bool,
    pub enabled: bool,
    pub v1_task: bool,
}

static CACHE: Mutex<Status> = Mutex::new(Status { known: false, enabled: false, v1_task: false });

fn schtasks(args: &[&str]) -> bool {
    Command::new("schtasks.exe")
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn task_exists(name: &str) -> bool {
    schtasks(&["/Query", "/TN", name])
}

/// Медленно (~0,1–0,3 с): вызывать из фонового потока, результат кешируется.
pub fn refresh() -> Status {
    let st = Status { known: true, enabled: task_exists(TASK_V2), v1_task: task_exists(TASK_V1) };
    if let Ok(mut c) = CACHE.lock() {
        *c = st;
    }
    st
}

pub fn cached() -> Status {
    CACHE.lock().map(|c| *c).unwrap_or_default()
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn task_xml() -> String {
    let user = format!(
        "{}\\{}",
        std::env::var("USERDOMAIN").unwrap_or_default(),
        std::env::var("USERNAME").unwrap_or_default()
    );
    let exe = paths::exe_path().display().to_string();
    format!(
        r#"<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <RegistrationInfo><Description>CPU Balancer: tray + background apps on E-cores</Description></RegistrationInfo>
  <Triggers><LogonTrigger><Enabled>true</Enabled><UserId>{user}</UserId></LogonTrigger></Triggers>
  <Principals><Principal id="Author"><UserId>{user}</UserId><LogonType>InteractiveToken</LogonType><RunLevel>HighestAvailable</RunLevel></Principal></Principals>
  <Settings>
    <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>
    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>
    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>
    <ExecutionTimeLimit>PT0S</ExecutionTimeLimit>
    <Priority>5</Priority>
  </Settings>
  <Actions Context="Author"><Exec><Command>"{exe}"</Command><Arguments>--tray</Arguments></Exec></Actions>
</Task>
"#,
        user = xml_escape(&user),
        exe = xml_escape(&exe)
    )
}

pub fn enable() -> Result<Status, String> {
    let path = std::env::temp_dir().join("cpu-balancer-task.xml");
    let mut bytes = vec![0xFF, 0xFE];
    for u in task_xml().encode_utf16() {
        bytes.extend_from_slice(&u.to_le_bytes());
    }
    std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
    let ok = schtasks(&["/Create", "/TN", TASK_V2, "/XML", &path.display().to_string(), "/F"]);
    let _ = std::fs::remove_file(&path);
    let st = refresh();
    if ok && st.enabled {
        Ok(st)
    } else {
        Err("schtasks /Create".into())
    }
}

pub fn disable() -> Status {
    schtasks(&["/Delete", "/TN", TASK_V2, "/F"]);
    refresh()
}

/// Удалить задачу v1 (наблюдатель на PowerShell) — v2 делает то же изнутри программы.
pub fn remove_v1() -> Status {
    schtasks(&["/End", "/TN", TASK_V1]);
    schtasks(&["/Delete", "/TN", TASK_V1, "/F"]);
    refresh()
}
