//! `cpu-balancer.exe --selftest <отчёт.txt>` — проверка из чек-листа релиза без окна:
//! сверка с powercfg, запись/«Исходные», бэкапы v1, эко-режимы на ping, наблюдатель.
//! Использует те же функции, что и кнопки интерфейса. Состояние схемы в конце возвращается как было.
//!
//! `--startlog <файл>` (обычный запуск) — записать время до первого кадра, мс.

use crate::editor::Editor;
use crate::{autostart, backup, eco, power};
use std::fmt::Write as _;
use std::os::windows::process::CommandExt;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

fn arg_after(args: &[String], flag: &str) -> Option<String> {
    args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1)).cloned()
}

pub fn startlog_path() -> Option<String> {
    arg_after(&std::env::args().collect::<Vec<_>>(), "--startlog")
}

/// Значения через powercfg /qh (только для сверки в тестах).
fn powercfg_values(scheme: &str) -> power::Values {
    let out = Command::new("powercfg.exe")
        .args(["/qh", scheme])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default();
    let mut vals = power::Values::new();
    let mut cur: Option<&'static str> = None;
    let mut hex: Vec<i64> = Vec::new();
    let flush = |cur: Option<&'static str>, hex: &Vec<i64>, vals: &mut power::Values| {
        if let Some(k) = cur {
            if hex.len() >= 2 {
                vals.insert(k.to_string(), [hex[hex.len() - 2], hex[hex.len() - 1]]);
            }
        }
    };
    for line in out.lines() {
        let t = line.trim_end();
        if let Some(p) = t.rfind("0x") {
            let h = &t[p + 2..];
            if h.len() == 8 && h.chars().all(|c| c.is_ascii_hexdigit()) {
                hex.push(i64::from_str_radix(h, 16).unwrap_or(0));
                continue;
            }
        }
        // строка заголовка настройки/подгруппы: ищем GUID, за которым идёт " ("
        let lower = t.to_ascii_lowercase();
        let found = lower.char_indices().find_map(|(i, _)| {
            let g = lower.get(i..i + 36)?;
            let ok = g.len() == 36
                && g.chars().enumerate().all(|(j, c)| if [8, 13, 18, 23].contains(&j) { c == '-' } else { c.is_ascii_hexdigit() });
            if ok && lower[i + 36..].trim_start().starts_with('(') {
                Some(g.to_string())
            } else {
                None
            }
        });
        if let Some(g) = found {
            flush(cur, &hex, &mut vals);
            cur = power::key_by_setting_guid(&g);
            hex.clear();
        }
    }
    flush(cur, &hex, &mut vals);
    vals
}

fn compare(r: &mut String, label: &str, api: &power::Values, pc: &power::Values) -> bool {
    let mut ok = true;
    let mut n = 0;
    for (k, _, _) in power::SETTINGS {
        match (api.get(*k), pc.get(*k)) {
            (Some(a), Some(p)) if a == p => n += 1,
            (None, None) => {}
            (a, p) => {
                ok = false;
                let _ = writeln!(r, "    РАСХОЖДЕНИЕ {k}: программа {a:?}, powercfg {p:?}");
            }
        }
    }
    let _ = writeln!(r, "  [{}] {label}: совпало {n} ключей", if ok { "OK" } else { "FAIL" });
    ok
}

fn spawn_ping() -> Option<Child> {
    Command::new("ping.exe")
        .args(["-n", "60", "127.0.0.1"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .ok()
}

fn prio_name(p: u32) -> String {
    match p {
        0x20 => "Normal".into(),
        0x4000 => "BelowNormal".into(),
        0x8000 => "AboveNormal".into(),
        0x40 => "Idle".into(),
        0x80 => "High".into(),
        x => format!("0x{x:X}"),
    }
}

pub fn run(args: &[String]) {
    let out_path = arg_after(args, "--selftest").unwrap_or_else(|| "selftest.txt".into());
    let mut r = String::new();
    let fails = std::cell::Cell::new(0u32);
    let check = |r: &mut String, ok: bool, text: String| {
        if !ok {
            fails.set(fails.get() + 1);
        }
        let _ = writeln!(r, "  [{}] {text}", if ok { "OK" } else { "FAIL" });
    };

    let _ = writeln!(r, "CPU Balancer {} — самопроверка", env!("CARGO_PKG_VERSION"));
    let _ = writeln!(r, "Папка данных: {}", crate::paths::data_dir().display());

    // --- скорость API
    let t = Instant::now();
    let schemes = power::list_schemes();
    let t_list = t.elapsed();
    let active = schemes.iter().find(|s| s.active).cloned();
    let Some(active) = active else {
        let _ = std::fs::write(&out_path, "нет активной схемы");
        return;
    };
    let t = Instant::now();
    let vals = power::read_scheme(&active.guid);
    let t_read = t.elapsed();
    let _ = writeln!(r, "\n1. Схемы и скорость");
    for s in &schemes {
        let _ = writeln!(r, "  {} {} {}", if s.active { "●" } else { " " }, s.guid, s.name);
    }
    let _ = writeln!(r, "  список схем: {:.2} мс, чтение {} настроек ×2: {:.2} мс", t_list.as_secs_f64() * 1e3, vals.len(), t_read.as_secs_f64() * 1e3);
    let missing: Vec<&str> = power::SETTINGS.iter().map(|s| s.0).filter(|k| !vals.contains_key(*k)).collect();
    if !missing.is_empty() {
        let _ = writeln!(r, "  нет в системе (н/д): {}", missing.join(", "));
    }
    let _ = writeln!(r, "  питание: {}", if power::on_ac() { "от сети (ACLineStatus)" } else { "от батареи" });

    // --- 2. сверка со всеми схемами
    let _ = writeln!(r, "\n2. Значения программы = powercfg /qh");
    for s in &schemes {
        let ok = compare(&mut r, &s.name, &power::read_scheme(&s.guid), &powercfg_values(&s.guid));
        if !ok {
            fails.set(fails.get() + 1);
        }
    }

    // --- 3. бэкапы v1
    let _ = writeln!(r, "\n3. Бэкапы v1");
    for s in &schemes {
        let cur = power::read_scheme(&s.guid);
        let bk = backup::get_backup(&s.guid, &s.name, &cur, false);
        let f = |k: &str| bk.get(k).map(|v| format!("{}/{}", v[0], v[1])).unwrap_or("—".into());
        let _ = writeln!(r, "  {}: исходные EPP P (EPP1) {}, EPP E {}, BOOST {}, FREQ1 {}", s.name, f("EPP1"), f("EPP"), f("BOOST"), f("FREQ1"));
        if s.guid == "381b4222-f694-41f0-9685-ff5bb260df2e" {
            let v = bk.get("EPP1").copied();
            check(&mut r, v == Some([33, 50]), format!("маркер исходного EPP P в Сбалансированной = 33/50 (есть {:?})", v));
        }
    }

    // --- 4. запись + «Исходные» на активной схеме, затем возврат как было
    let _ = writeln!(r, "\n4. Запись и «Исходные» (активная схема «{}»)", active.name);
    let snapshot = power::read_scheme(&active.guid);
    let mut ed = Editor::open(&active.guid, &active.name, true);
    let old = ed.get("EPP1", 0).unwrap_or(50);
    let new = if old <= 90 { old + 7 } else { old - 7 };
    ed.set_control("eppP", 0, new);
    let t = Instant::now();
    let res = ed.save();
    let t_save = t.elapsed();
    let pc = powercfg_values(&active.guid);
    check(
        &mut r,
        res.failed.is_empty() && pc.get("EPP1").map(|v| v[0]) == Some(new),
        format!("EPP P от сети {old} → {new}: сохранено {}, за {:.1} мс, powercfg показывает {:?}", res.saved, t_save.as_secs_f64() * 1e3, pc.get("EPP1")),
    );
    ed.restore_backup();
    let res = ed.save();
    let pc = powercfg_values(&active.guid);
    let mut same = true;
    for (k, v) in &ed.bk {
        if pc.get(k) != Some(v) {
            same = false;
            let _ = writeln!(r, "    {k}: бэкап {v:?}, powercfg {:?}", pc.get(k));
        }
    }
    check(&mut r, same && res.failed.is_empty(), format!("«Исходные» → все {} ключей = бэкапу (записано {})", ed.bk.len(), res.saved));
    // вернуть пользовательские значения как были до теста
    for (k, v) in &snapshot {
        for c in 0..2 {
            ed.set(k, c, v[c]);
        }
    }
    let res = ed.save();
    let back = powercfg_values(&active.guid);
    let restored = snapshot.iter().all(|(k, v)| back.get(k) == Some(v));
    check(&mut r, restored && res.failed.is_empty(), format!("значения до теста восстановлены (записано {})", res.saved));

    // --- 5. эко-режимы на ping
    let emask = eco::efficient_mask();
    let _ = writeln!(r, "\n5. Эко-режимы (маска E-ядер 0x{emask:X}, логических {})", eco::logical_cpus());
    if let Some(mut ping) = spawn_ping() {
        let pid = ping.id();
        std::thread::sleep(Duration::from_millis(200));
        let ok1 = eco::apply(pid, eco::MODE_ECO, emask);
        let s1 = eco::inspect(pid);
        check(
            &mut r,
            ok1 && matches!(s1, Some((0x4000, true, _))),
            format!("Эко → {:?}", s1.map(|(p, e, m)| format!("{} EcoQoS={} маска 0x{m:X}", prio_name(p), e))),
        );
        let ok2 = eco::apply(pid, eco::MODE_ECORE, emask);
        let s2 = eco::inspect(pid);
        check(
            &mut r,
            ok2 && s2.map(|s| s.1 && s.2 == emask).unwrap_or(false),
            format!("Только E → {:?}", s2.map(|(p, e, m)| format!("{} EcoQoS={} маска 0x{m:X}", prio_name(p), e))),
        );
        let ok0 = eco::apply(pid, eco::MODE_NORMAL, emask);
        let s0 = eco::inspect(pid);
        let full = (1u64 << eco::logical_cpus()) - 1;
        check(
            &mut r,
            ok0 && matches!(s0, Some((0x20, false, m)) if m == full),
            format!("Обычный → {:?}", s0.map(|(p, e, m)| format!("{} EcoQoS={} маска 0x{m:X}", prio_name(p), e))),
        );
        let _ = ping.kill();
    } else {
        check(&mut r, false, "не удалось запустить ping".into());
    }

    // --- 6. наблюдатель
    let _ = writeln!(r, "\n6. Наблюдатель: ping.exe в «Только E-ядра», новый ping");
    let t = Instant::now();
    let mut n = 0;
    for _ in 0..200 {
        n = eco::pids().len();
    }
    let snap = t.elapsed().as_secs_f64() * 1e3 / 200.0;
    let _ = writeln!(
        r,
        "  список {n} PID: {snap:.3} мс; опрос раз в {} мс ≈ {:.3}% одного ядра",
        eco::POLL.as_millis(),
        snap / eco::POLL.as_millis() as f64 * 100.0
    );
    let mut apps = eco::Apps::new();
    apps.insert("ping.exe".into(), eco::MODE_ECORE);
    let w = eco::Watcher::new(apps);
    w.start();
    let t = Instant::now();
    while *w.method.read().unwrap() == "—" && t.elapsed() < Duration::from_secs(5) {
        std::thread::sleep(Duration::from_millis(20));
    }
    std::thread::sleep(Duration::from_millis(300));
    let method = *w.method.read().unwrap();
    if let Some(mut ping) = spawn_ping() {
        let pid = ping.id();
        let t = Instant::now();
        let mut got = None;
        while t.elapsed() < Duration::from_secs(3) {
            if eco::state(pid) == 2 {
                got = Some(t.elapsed());
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let s = eco::inspect(pid);
        check(
            &mut r,
            got.map(|d| d <= Duration::from_secs(1)).unwrap_or(false),
            format!(
                "способ: {method}; режим включился через {} ({:?})",
                got.map(|d| format!("{} мс", d.as_millis())).unwrap_or("— (не включился за 3 с)".into()),
                s.map(|(p, e, m)| format!("{} EcoQoS={} маска 0x{m:X}", prio_name(p), e))
            ),
        );
        let _ = ping.kill();
    }

    // --- 7. Планировщик (только чтение статуса)
    let _ = writeln!(r, "\n7. Автозапуск");
    let t = Instant::now();
    let st = autostart::refresh();
    let _ = writeln!(
        r,
        "  задача «{}»: {}, задача v1: {} (запрос {:.0} мс, в UI — из кеша)",
        autostart::TASK_V2,
        if st.enabled { "есть" } else { "нет" },
        if st.v1_task { "есть" } else { "нет" },
        t.elapsed().as_secs_f64() * 1e3
    );

    let _ = writeln!(r, "\nИТОГ: {}", if fails.get() == 0 { "все проверки пройдены".to_string() } else { format!("провалено проверок: {}", fails.get()) });
    let _ = std::fs::write(&out_path, r);
}
