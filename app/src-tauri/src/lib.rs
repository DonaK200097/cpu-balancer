//! CPU Balancer v2: вся логика в Rust, фронтенд только отображает и вызывает команды.

pub mod autostart;
pub mod backup;
pub mod controls;
pub mod eco;
pub mod editor;
pub mod paths;
pub mod power;
pub mod selftest;
pub mod settings;
pub mod tray;

use editor::{Editor, EditorView, SaveResult};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tauri::{AppHandle, Manager, State, WindowEvent};

pub struct AppState {
    pub editor: Mutex<Editor>,
    pub watcher: Arc<eco::Watcher>,
    pub settings: Mutex<settings::Settings>,
    pub started: Instant,
    pub start_hidden: bool,
    pub shown_once: AtomicBool,
    pub startup_ms: Mutex<Option<u128>>,
}

impl AppState {
    pub fn lang(&self) -> String {
        self.settings.lock().map(|s| settings::effective_lang(&s)).unwrap_or_else(|_| "en".into())
    }

    pub fn set_paused(&self, paused: bool) {
        self.watcher.paused.store(paused, Ordering::Relaxed);
        if let Ok(mut s) = self.settings.lock() {
            s.paused = paused;
            s.save();
        }
        if !paused {
            let w = self.watcher.clone();
            std::thread::spawn(move || w.sweep());
        }
    }
}

/// Пока окно скрыто в трее, WebView2 приостановлен: ни таймеров, ни отрисовки, ни нагрузки на GPU.
pub fn set_webview_active(app: &AppHandle, active: bool) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.with_webview(move |wv| unsafe {
            use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2_3;
            use windows::core::Interface;
            let c = wv.controller();
            let _ = c.SetIsVisible(active);
            if !active {
                if let Ok(c3) = c.CoreWebView2().and_then(|core| core.cast::<ICoreWebView2_3>()) {
                    let handler = webview2_com::TrySuspendCompletedHandler::create(Box::new(|_, _| Ok(())));
                    let _ = c3.TrySuspend(&handler);
                }
            }
        });
    }
}

pub fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        set_webview_active(app, true);
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

fn os_build() -> u32 {
    use windows::core::w;
    use windows::Win32::System::Registry::{RegGetValueW, HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ};
    let mut buf = [0u16; 32];
    let mut size = (buf.len() * 2) as u32;
    let r = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            w!("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion"),
            w!("CurrentBuildNumber"),
            RRF_RT_REG_SZ,
            None,
            Some(buf.as_mut_ptr() as _),
            Some(&mut size),
        )
    };
    if r.is_err() {
        return 0;
    }
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end]).parse().unwrap_or(0)
}

// ---------------------------------------------------------------- команды

#[derive(Serialize)]
struct Meta {
    version: &'static str,
    lang: String,
    lang_setting: String,
    accent: Option<String>,
    mica: bool,
    groups: Vec<controls::Group>,
    presets: Vec<controls::Preset>,
    emask: String,
    ecores: u32,
    ncpu: usize,
    data_dir: String,
    check_updates: bool,
}

#[tauri::command]
fn get_meta(state: State<AppState>) -> Meta {
    let s = state.settings.lock().unwrap();
    let emask = eco::efficient_mask();
    Meta {
        version: env!("CARGO_PKG_VERSION"),
        lang: settings::effective_lang(&s),
        lang_setting: s.lang.clone(),
        accent: settings::accent_color(),
        mica: os_build() >= 22000,
        groups: controls::groups(),
        presets: controls::presets(),
        emask: format!("0x{emask:X}"),
        ecores: emask.count_ones(),
        ncpu: eco::logical_cpus(),
        data_dir: paths::data_dir().display().to_string(),
        check_updates: s.check_updates,
    }
}

/// Фронтенд отрисовал первый кадр — показать окно (если не запуск в трей).
#[tauri::command]
fn ui_ready(app: AppHandle, state: State<AppState>) -> u128 {
    let ms = state.started.elapsed().as_millis();
    if let Ok(mut s) = state.startup_ms.lock() {
        if s.is_none() {
            *s = Some(ms);
            if let Some(path) = selftest::startlog_path() {
                let _ = std::fs::write(path, format!("{ms}"));
            }
        }
    }
    if !state.shown_once.swap(true, Ordering::Relaxed) {
        if state.start_hidden {
            set_webview_active(&app, false);
        } else {
            show_main(&app);
        }
    }
    ms
}

#[tauri::command]
fn list_schemes() -> Vec<power::Scheme> {
    power::list_schemes()
}

#[tauri::command]
fn set_active(app: AppHandle, guid: String) -> Vec<power::Scheme> {
    power::set_active(&guid);
    tray::refresh(&app);
    power::list_schemes()
}

#[tauri::command]
fn open_scheme(state: State<AppState>, guid: String) -> Result<EditorView, String> {
    let name = power::list_schemes().into_iter().find(|s| s.guid == guid).map(|s| s.name).ok_or("scheme not found")?;
    let mut ed = state.editor.lock().unwrap();
    *ed = Editor::open(&guid, &name, true);
    Ok(ed.view())
}

#[tauri::command]
fn editor_view(state: State<AppState>) -> EditorView {
    state.editor.lock().unwrap().view()
}

/// Перечитать значения схемы из системы, сохранив правки.
#[tauri::command]
fn reload_scheme(state: State<AppState>) -> EditorView {
    let mut ed = state.editor.lock().unwrap();
    if !ed.scheme.is_empty() {
        ed.cur = power::read_scheme(&ed.scheme);
        let cur = ed.cur.clone();
        ed.pend.retain(|k, v| cur.get(k) != Some(v));
    }
    ed.view()
}

#[tauri::command]
fn set_control(state: State<AppState>, id: String, col: usize, value: i64) -> EditorView {
    let mut ed = state.editor.lock().unwrap();
    ed.set_control(&id, col, value);
    ed.view()
}

#[tauri::command]
fn copy_control(state: State<AppState>, id: String, col: usize) -> EditorView {
    let mut ed = state.editor.lock().unwrap();
    ed.copy_to_other(&id, col);
    ed.view()
}

#[tauri::command]
fn apply_preset(state: State<AppState>, id: String) -> EditorView {
    let mut ed = state.editor.lock().unwrap();
    ed.apply_preset(&id);
    ed.view()
}

#[tauri::command]
fn discard(state: State<AppState>) -> EditorView {
    let mut ed = state.editor.lock().unwrap();
    ed.discard();
    ed.view()
}

#[tauri::command]
fn restore_backup(state: State<AppState>) -> EditorView {
    let mut ed = state.editor.lock().unwrap();
    ed.restore_backup();
    ed.view()
}

#[derive(Serialize)]
struct Saved {
    result: SaveResult,
    view: EditorView,
}

#[tauri::command]
fn save(app: AppHandle, state: State<AppState>) -> Saved {
    let mut ed = state.editor.lock().unwrap();
    let result = ed.save();
    tray::refresh(&app);
    Saved { result, view: ed.view() }
}

#[tauri::command]
fn power_status() -> bool {
    power::on_ac()
}

// ---- фоновые программы

#[tauri::command]
async fn eco_rows(state: State<'_, AppState>) -> Result<Vec<eco::EcoRow>, String> {
    let apps = state.watcher.apps.read().map(|a| a.clone()).unwrap_or_default();
    tauri::async_runtime::spawn_blocking(move || eco::rows(&apps)).await.map_err(|e| e.to_string())
}

#[derive(Serialize)]
struct EcoSetResult {
    ok: u32,
    bad: u32,
    saved: bool,
}

#[tauri::command]
fn eco_set(state: State<AppState>, name: String, mode: u8) -> EcoSetResult {
    let name = name.to_lowercase();
    let mode = if eco::efficient_mask() == 0 { mode.min(1) } else { mode.min(2) };
    let saved = {
        let mut apps = state.watcher.apps.write().unwrap();
        if mode > 0 {
            apps.insert(name.clone(), mode);
        } else {
            apps.remove(&name);
        }
        eco::save_apps(&apps).is_ok()
    };
    let (ok, bad) = eco::apply_name(&name, mode);
    EcoSetResult { ok, bad, saved }
}

#[derive(Serialize)]
struct EcoInfo {
    method: &'static str,
    paused: bool,
    applied: u64,
    autostart: autostart::Status,
}

#[tauri::command]
fn eco_info(state: State<AppState>) -> EcoInfo {
    EcoInfo {
        method: *state.watcher.method.read().unwrap(),
        paused: state.watcher.paused.load(Ordering::Relaxed),
        applied: state.watcher.applied.load(Ordering::Relaxed),
        autostart: autostart::cached(),
    }
}

#[tauri::command]
fn set_paused(app: AppHandle, state: State<AppState>, paused: bool) {
    state.set_paused(paused);
    tray::refresh(&app);
}

#[tauri::command]
async fn autostart_refresh() -> Result<autostart::Status, String> {
    tauri::async_runtime::spawn_blocking(autostart::refresh).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn autostart_set(enabled: bool) -> Result<autostart::Status, String> {
    tauri::async_runtime::spawn_blocking(move || if enabled { autostart::enable() } else { Ok(autostart::disable()) })
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn autostart_remove_v1() -> Result<autostart::Status, String> {
    tauri::async_runtime::spawn_blocking(autostart::remove_v1).await.map_err(|e| e.to_string())
}

// ---- прочее

#[tauri::command]
fn set_lang(app: AppHandle, state: State<AppState>, lang: String) -> String {
    let eff = {
        let mut s = state.settings.lock().unwrap();
        s.lang = if lang == "ru" || lang == "en" { lang } else { String::new() };
        s.save();
        settings::effective_lang(&s)
    };
    tray::refresh(&app);
    eff
}

#[tauri::command]
fn set_check_updates(state: State<AppState>, enabled: bool) {
    let mut s = state.settings.lock().unwrap();
    s.check_updates = enabled;
    s.save();
}

fn shell_open(target: &str) {
    use windows::core::{w, HSTRING};
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    unsafe {
        ShellExecuteW(None, w!("open"), &HSTRING::from(target), None, None, SW_SHOWNORMAL);
    }
}

#[tauri::command]
fn open_url(url: String) {
    if url.starts_with("https://github.com/DonaK200097/cpu-balancer") {
        shell_open(&url);
    }
}

#[tauri::command]
fn open_data_dir() {
    shell_open(&paths::data_dir().display().to_string());
}

#[tauri::command]
fn quit(app: AppHandle) {
    app.exit(0);
}

// ---------------------------------------------------------------- запуск

pub fn run() {
    let started = Instant::now();
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--selftest") {
        selftest::run(&args);
        return;
    }
    let start_hidden = args.iter().any(|a| a == "--tray");

    let settings = settings::Settings::load();
    let watcher = eco::Watcher::new(eco::load_apps());
    watcher.paused.store(settings.paused, Ordering::Relaxed);
    watcher.start();
    // статус задачи Планировщика — медленно, в фоне, после первого кадра и в кеш
    std::thread::spawn(|| {
        std::thread::sleep(std::time::Duration::from_millis(1500));
        autostart::refresh();
    });

    let state = AppState {
        editor: Mutex::new(Editor::default()),
        watcher,
        settings: Mutex::new(settings),
        started,
        start_hidden,
        shown_once: AtomicBool::new(false),
        startup_ms: Mutex::new(None),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| show_main(app)))
        .manage(state)
        .setup(|app| {
            tray::create(app.handle())?;
            Ok(())
        })
        .on_window_event(|window, event| {
            // закрытие окна сворачивает в трей
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
                set_webview_active(window.app_handle(), false);
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_meta,
            ui_ready,
            list_schemes,
            set_active,
            open_scheme,
            editor_view,
            reload_scheme,
            set_control,
            copy_control,
            apply_preset,
            discard,
            restore_backup,
            save,
            power_status,
            eco_rows,
            eco_set,
            eco_info,
            set_paused,
            autostart_refresh,
            autostart_set,
            autostart_remove_v1,
            set_lang,
            set_check_updates,
            open_url,
            open_data_dir,
            quit,
        ])
        .run(tauri::generate_context!())
        .expect("error while running CPU Balancer");
}
