//! Фоновые программы на E-ядрах: EcoQoS («Режим эффективности»), привязка к экономным ядрам,
//! наблюдатель, который применяет режим к новым процессам сразу при запуске.
//!
//! `powercfg /powerthrottling` умеет только исключать программу из экономии, поэтому режим
//! ставится на живой процесс и держится, пока процесс жив.

use crate::paths;
use serde::Serialize;
use serde_json::{json, Map, Value};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, OnceLock, RwLock};
use std::time::{Duration, Instant};
use windows::Win32::Foundation::{CloseHandle, FILETIME, HANDLE};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::SystemInformation::GetSystemCpuSetInformation;
use windows::Win32::System::Threading::{
    GetCurrentProcessId, GetPriorityClass, GetProcessAffinityMask, GetProcessInformation, GetProcessTimes,
    OpenProcess, ProcessPowerThrottling, SetPriorityClass, SetProcessAffinityMask, SetProcessInformation,
    BELOW_NORMAL_PRIORITY_CLASS, NORMAL_PRIORITY_CLASS, PROCESS_POWER_THROTTLING_EXECUTION_SPEED,
    PROCESS_POWER_THROTTLING_IGNORE_TIMER_RESOLUTION, PROCESS_POWER_THROTTLING_STATE,
    PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SET_INFORMATION,
};

pub const MODE_NORMAL: u8 = 0;
pub const MODE_ECO: u8 = 1;
pub const MODE_ECORE: u8 = 2;

/// Системные процессы, которые не показываем (имя без .exe, в нижнем регистре).
const SKIP: &[&str] = &[
    "idle", "system", "registry", "smss", "csrss", "wininit", "services", "lsass", "lsaiso", "winlogon",
    "fontdrvhost", "dwm", "memory compression", "secure system", "audiodg", "conhost", "svchost", "taskmgr",
    "[system process]", "cpu-balancer",
];

const ECO_BITS: u32 = PROCESS_POWER_THROTTLING_EXECUTION_SPEED | PROCESS_POWER_THROTTLING_IGNORE_TIMER_RESOLUTION;

struct Handle(HANDLE);
impl Drop for Handle {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

fn open(pid: u32, set: bool) -> Option<Handle> {
    let access = if set { PROCESS_SET_INFORMATION | PROCESS_QUERY_LIMITED_INFORMATION } else { PROCESS_QUERY_LIMITED_INFORMATION };
    unsafe { OpenProcess(access, false, pid).ok().map(Handle) }
}

/// Логические процессоры самого экономного класса (E + LP-E), группа 0. 0 — если ядра одинаковые.
pub fn efficient_mask() -> u64 {
    static MASK: OnceLock<u64> = OnceLock::new();
    *MASK.get_or_init(|| unsafe {
        let mut len = 0u32;
        let _ = GetSystemCpuSetInformation(None, 0, &mut len, None, None);
        if len == 0 {
            return 0;
        }
        let mut buf = vec![0u8; len as usize];
        if !GetSystemCpuSetInformation(Some(buf.as_mut_ptr() as _), len, &mut len, None, None).as_bool() {
            return 0;
        }
        // SYSTEM_CPU_SET_INFORMATION: Size u32 @0, Group u16 @12, LogicalProcessorIndex u8 @14, EfficiencyClass u8 @18
        let mut lps = Vec::new();
        let (mut min, mut max) = (u8::MAX, 0u8);
        let mut off = 0usize;
        while off + 19 <= len as usize {
            let size = u32::from_le_bytes(buf[off..off + 4].try_into().unwrap()) as usize;
            if size == 0 {
                break;
            }
            let group = u16::from_le_bytes(buf[off + 12..off + 14].try_into().unwrap());
            let idx = buf[off + 14];
            let cls = buf[off + 18];
            if group == 0 && idx < 64 {
                lps.push((idx, cls));
                min = min.min(cls);
                max = max.max(cls);
            }
            off += size;
        }
        if lps.is_empty() || min == max {
            return 0;
        }
        lps.iter().filter(|(_, c)| *c == min).fold(0u64, |m, (i, _)| m | (1u64 << i))
    })
}

pub fn logical_cpus() -> usize {
    std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1)
}

/// Режим процессу: 0 — обычный, 1 — EcoQoS, 2 — EcoQoS + только E-ядра. false — отказ доступа.
pub fn apply(pid: u32, mode: u8, emask: u64) -> bool {
    let Some(h) = open(pid, true) else { return false };
    unsafe {
        let mut t = PROCESS_POWER_THROTTLING_STATE { Version: 1, ControlMask: 0, StateMask: 0 };
        if mode > 0 {
            t.ControlMask = ECO_BITS;
            t.StateMask = ECO_BITS;
        }
        let ok = SetProcessInformation(
            h.0,
            ProcessPowerThrottling,
            &t as *const _ as _,
            std::mem::size_of::<PROCESS_POWER_THROTTLING_STATE>() as u32,
        )
        .is_ok();
        let _ = SetPriorityClass(h.0, if mode > 0 { BELOW_NORMAL_PRIORITY_CLASS } else { NORMAL_PRIORITY_CLASS });
        let (mut pm, mut sm) = (0usize, 0usize);
        if GetProcessAffinityMask(h.0, &mut pm, &mut sm).is_ok() {
            let all = sm as u64;
            let want = if mode == MODE_ECORE && (emask & all) != 0 { emask & all } else { all };
            if pm as u64 != want {
                let _ = SetProcessAffinityMask(h.0, want as usize);
            }
        }
        ok
    }
}

/// Текущее состояние: -1 нет доступа, 0 обычный, 1 EcoQoS, 2 EcoQoS + урезанная привязка.
pub fn state(pid: u32) -> i32 {
    let Some(h) = open(pid, false) else { return -1 };
    unsafe {
        let mut t = PROCESS_POWER_THROTTLING_STATE { Version: 1, ControlMask: 0, StateMask: 0 };
        if GetProcessInformation(
            h.0,
            ProcessPowerThrottling,
            &mut t as *mut _ as _,
            std::mem::size_of::<PROCESS_POWER_THROTTLING_STATE>() as u32,
        )
        .is_err()
        {
            return -1;
        }
        if t.StateMask & PROCESS_POWER_THROTTLING_EXECUTION_SPEED == 0 {
            return 0;
        }
        let (mut pm, mut sm) = (0usize, 0usize);
        if GetProcessAffinityMask(h.0, &mut pm, &mut sm).is_ok() && pm != sm {
            return 2;
        }
        1
    }
}

/// Для проверки: (класс приоритета, EcoQoS включён, маска привязки).
pub fn inspect(pid: u32) -> Option<(u32, bool, u64)> {
    let h = open(pid, false)?;
    unsafe {
        let prio = GetPriorityClass(h.0);
        let mut t = PROCESS_POWER_THROTTLING_STATE { Version: 1, ControlMask: 0, StateMask: 0 };
        let _ = GetProcessInformation(
            h.0,
            ProcessPowerThrottling,
            &mut t as *mut _ as _,
            std::mem::size_of::<PROCESS_POWER_THROTTLING_STATE>() as u32,
        );
        let (mut pm, mut sm) = (0usize, 0usize);
        let _ = GetProcessAffinityMask(h.0, &mut pm, &mut sm);
        Some((prio, t.StateMask & PROCESS_POWER_THROTTLING_EXECUTION_SPEED != 0, pm as u64))
    }
}

/// Все процессы: (pid, имя exe в нижнем регистре).
pub fn processes() -> Vec<(u32, String)> {
    let mut out = Vec::with_capacity(400);
    unsafe {
        let Ok(snap) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else { return out };
        let snap = Handle(snap);
        let mut e = PROCESSENTRY32W { dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32, ..Default::default() };
        if Process32FirstW(snap.0, &mut e).is_ok() {
            loop {
                let end = e.szExeFile.iter().position(|&c| c == 0).unwrap_or(e.szExeFile.len());
                out.push((e.th32ProcessID, String::from_utf16_lossy(&e.szExeFile[..end]).to_lowercase()));
                if Process32NextW(snap.0, &mut e).is_err() {
                    break;
                }
            }
        }
    }
    out
}

fn cpu_time(pid: u32) -> Option<u64> {
    let h = open(pid, false)?;
    let (mut c, mut x, mut k, mut u) = (FILETIME::default(), FILETIME::default(), FILETIME::default(), FILETIME::default());
    unsafe { GetProcessTimes(h.0, &mut c, &mut x, &mut k, &mut u).ok()? };
    let ft = |f: FILETIME| ((f.dwHighDateTime as u64) << 32) | f.dwLowDateTime as u64;
    Some(ft(k) + ft(u))
}

fn stem(name: &str) -> &str {
    name.strip_suffix(".exe").unwrap_or(name)
}

fn skipped(pid: u32, name: &str) -> bool {
    pid == 0 || pid == 4 || pid == unsafe { GetCurrentProcessId() } || SKIP.contains(&stem(name))
}

// ---------------------------------------------------------------- список программ (eco-apps.json)

pub type Apps = HashMap<String, u8>;

pub fn load_apps() -> Apps {
    let mut apps = Apps::new();
    let Some(text) = paths::read_text(&paths::file("eco-apps.json")) else { return apps };
    let Ok(j) = serde_json::from_str::<Value>(&text) else { return apps };
    if let Some(o) = j.get("Apps").and_then(Value::as_object) {
        for (k, v) in o {
            if let Some(m) = v.as_u64() {
                if (1..=2).contains(&m) {
                    apps.insert(k.to_lowercase(), m as u8);
                }
            }
        }
    }
    apps
}

pub fn save_apps(apps: &Apps) -> std::io::Result<()> {
    let mut names: Vec<&String> = apps.iter().filter(|(_, m)| **m > 0).map(|(n, _)| n).collect();
    names.sort();
    let mut o = Map::new();
    for n in names {
        o.insert(n.clone(), json!(apps[n]));
    }
    let doc = json!({ "Apps": Value::Object(o) });
    paths::write_text(&paths::file("eco-apps.json"), &serde_json::to_string_pretty(&doc).unwrap_or_default())
}

#[derive(Serialize, Clone)]
pub struct EcoRow {
    pub name: String,
    pub mode: u8,
    pub procs: u32,
    pub cpu: f64,
    pub eco: u32,
}

/// Выбранные программы + запущенные, с загрузкой CPU за ~0,5 с.
pub fn rows(apps: &Apps) -> Vec<EcoRow> {
    let first: HashMap<u32, u64> = processes().into_iter().filter_map(|(pid, _)| cpu_time(pid).map(|t| (pid, t))).collect();
    let t0 = Instant::now();
    std::thread::sleep(Duration::from_millis(500));
    let mut by: HashMap<String, (u32, u64, u32)> = HashMap::new();
    for (pid, name) in processes() {
        if skipped(pid, &name) {
            continue;
        }
        let r = by.entry(name).or_default();
        r.0 += 1;
        if let (Some(a), Some(b)) = (first.get(&pid), cpu_time(pid)) {
            r.1 += b.saturating_sub(*a);
        }
        if state(pid) > 0 {
            r.2 += 1;
        }
    }
    let elapsed_100ns = t0.elapsed().as_nanos() as f64 / 100.0;
    let ncpu = logical_cpus() as f64;
    let mut out: Vec<EcoRow> = by
        .into_iter()
        .map(|(name, (n, t, e))| EcoRow {
            mode: apps.get(&name).copied().unwrap_or(0),
            cpu: ((t as f64 / elapsed_100ns / ncpu) * 1000.0).round() / 10.0,
            name,
            procs: n,
            eco: e,
        })
        .collect();
    for (n, m) in apps {
        if !out.iter().any(|r| &r.name == n) {
            out.push(EcoRow { name: n.clone(), mode: *m, procs: 0, cpu: 0.0, eco: 0 });
        }
    }
    out.sort_by(|a, b| {
        let (sa, sb) = (a.mode > 0, b.mode > 0);
        sb.cmp(&sa).then_with(|| {
            if sa {
                a.name.cmp(&b.name)
            } else {
                b.cpu.partial_cmp(&a.cpu).unwrap_or(std::cmp::Ordering::Equal).then_with(|| a.name.cmp(&b.name))
            }
        })
    });
    out
}

/// Применить режим ко всем запущенным процессам с этим exe. → (успешно, отказано)
pub fn apply_name(name: &str, mode: u8) -> (u32, u32) {
    let emask = efficient_mask();
    let (mut ok, mut bad) = (0, 0);
    for (pid, n) in processes() {
        if n != name || pid == unsafe { GetCurrentProcessId() } {
            continue;
        }
        if apply(pid, mode, emask) {
            ok += 1;
        } else {
            bad += 1;
        }
    }
    (ok, bad)
}

// ---------------------------------------------------------------- наблюдатель

pub struct Watcher {
    pub apps: RwLock<Apps>,
    pub paused: AtomicBool,
    /// "wmi" — события запуска, "poll" — запасной опрос
    pub method: RwLock<&'static str>,
    pub applied: AtomicU64,
}

impl Watcher {
    pub fn new(apps: Apps) -> Arc<Watcher> {
        Arc::new(Watcher { apps: RwLock::new(apps), paused: AtomicBool::new(false), method: RwLock::new("—"), applied: AtomicU64::new(0) })
    }

    fn mode_for(&self, name: &str) -> u8 {
        self.apps.read().map(|a| a.get(name).copied().unwrap_or(0)).unwrap_or(0)
    }

    fn on_start(&self, pid: u32, name: &str) {
        if self.paused.load(Ordering::Relaxed) {
            return;
        }
        let m = self.mode_for(name);
        // событие WMI и опрос могут увидеть один и тот же процесс
        if m > 0 && state(pid) < m as i32 && apply(pid, m, efficient_mask()) {
            self.applied.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Перепроверка всех процессов: некоторые программы сами сбрасывают приоритет/режим.
    pub fn sweep(&self) {
        if self.paused.load(Ordering::Relaxed) {
            return;
        }
        let apps = self.apps.read().map(|a| a.clone()).unwrap_or_default();
        if apps.is_empty() {
            return;
        }
        let emask = efficient_mask();
        for (pid, name) in processes() {
            if let Some(&m) = apps.get(&name) {
                if m > 0 && state(pid) < m as i32 && apply(pid, m, emask) {
                    self.applied.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
    }

    pub fn start(self: &Arc<Self>) {
        let w = self.clone();
        std::thread::Builder::new()
            .name("eco-sweep".into())
            .spawn(move || loop {
                w.sweep();
                std::thread::sleep(Duration::from_secs(30));
            })
            .ok();
        let w = self.clone();
        std::thread::Builder::new()
            .name("eco-events".into())
            .spawn(move || {
                // не мешаем первому кадру: COM/WMI поднимаются ~0,1–0,3 с
                std::thread::sleep(Duration::from_millis(1500));
                if w.run_wmi().is_err() {
                    if let Ok(mut m) = w.method.write() {
                        *m = "poll";
                    }
                }
            })
            .ok();
        let w = self.clone();
        std::thread::Builder::new().name("eco-poll".into()).spawn(move || w.run_poll()).ok();
    }

    /// События Win32_ProcessStartTrace (нужны права администратора). Блокирует поток.
    /// WMI доставляет их с задержкой ~1–2 с (буфер ETW), поэтому рядом работает быстрый опрос.
    fn run_wmi(&self) -> Result<(), ()> {
        use wmi::{Variant, WMIConnection};
        let con = WMIConnection::new().map_err(|_| ())?;
        let iter = con
            .raw_notification::<HashMap<String, Variant>>("SELECT ProcessID, ProcessName FROM Win32_ProcessStartTrace")
            .map_err(|_| ())?;
        if let Ok(mut m) = self.method.write() {
            *m = "wmi";
        }
        for ev in iter {
            let Ok(ev) = ev else { continue };
            let pid = match ev.get("ProcessID") {
                Some(Variant::UI4(v)) => *v,
                Some(Variant::I4(v)) => *v as u32,
                _ => continue,
            };
            let name = match ev.get("ProcessName") {
                Some(Variant::String(s)) => s.to_lowercase(),
                _ => continue,
            };
            self.on_start(pid, &name);
        }
        Err(())
    }

    /// Быстрый опрос: список PID через EnumProcesses (микросекунды), имя exe — только у новых.
    /// Работает, только когда список не пуст и автоприменение не на паузе.
    fn run_poll(&self) {
        let mut seen: HashSet<u32> = HashSet::new();
        let mut idle = true;
        loop {
            std::thread::sleep(POLL);
            let active = !self.paused.load(Ordering::Relaxed) && self.apps.read().map(|a| !a.is_empty()).unwrap_or(false);
            if !active {
                idle = true;
                continue;
            }
            let now = pids();
            if !idle {
                for pid in &now {
                    if !seen.contains(pid) {
                        if let Some(name) = exe_name(*pid) {
                            self.on_start(*pid, &name);
                        }
                    }
                }
            }
            idle = false;
            seen.clear();
            seen.extend(now);
        }
    }
}

pub const POLL: Duration = Duration::from_millis(250);

/// Все PID (без имён) — очень дёшево.
pub fn pids() -> Vec<u32> {
    use windows::Win32::System::ProcessStatus::EnumProcesses;
    let mut buf = vec![0u32; 2048];
    loop {
        let mut needed = 0u32;
        let bytes = (buf.len() * 4) as u32;
        if unsafe { EnumProcesses(buf.as_mut_ptr(), bytes, &mut needed) }.is_err() {
            return Vec::new();
        }
        if needed < bytes {
            buf.truncate(needed as usize / 4);
            return buf;
        }
        buf.resize(buf.len() * 2, 0);
    }
}

/// Имя exe процесса в нижнем регистре.
pub fn exe_name(pid: u32) -> Option<String> {
    use windows::core::PWSTR;
    use windows::Win32::System::Threading::{QueryFullProcessImageNameW, PROCESS_NAME_WIN32};
    let h = open(pid, false)?;
    let mut buf = [0u16; 1024];
    let mut len = buf.len() as u32;
    unsafe { QueryFullProcessImageNameW(h.0, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len).ok()? };
    let path = String::from_utf16_lossy(&buf[..len as usize]);
    Some(path.rsplit('\\').next().unwrap_or(&path).to_lowercase())
}
