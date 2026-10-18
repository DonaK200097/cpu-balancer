//! Значок в трее: открыть, быстрое переключение схемы, пауза автоприменения, выход.

use crate::{power, show_main, AppState};
use std::sync::atomic::Ordering;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, Wry};

const TRAY_ID: &str = "main";

fn tr(lang: &str, key: &str) -> &'static str {
    let ru = lang == "ru";
    match key {
        "open" => if ru { "Открыть CPU Balancer" } else { "Open CPU Balancer" },
        "scheme" => if ru { "Схема питания" } else { "Power plan" },
        "pause" => if ru { "Пауза автоприменения" } else { "Pause auto-apply" },
        "exit" => if ru { "Выход" } else { "Exit" },
        _ => "",
    }
}

fn build_menu(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let state = app.state::<AppState>();
    let lang = state.lang();
    let schemes = power::list_schemes();
    let items: Vec<CheckMenuItem<Wry>> = schemes
        .iter()
        .map(|s| CheckMenuItem::with_id(app, format!("scheme:{}", s.guid), &s.name, true, s.active, None::<&str>))
        .collect::<Result<_, _>>()?;
    let refs: Vec<&dyn tauri::menu::IsMenuItem<Wry>> = items.iter().map(|i| i as &dyn tauri::menu::IsMenuItem<Wry>).collect();
    let sub = Submenu::with_items(app, tr(&lang, "scheme"), true, &refs)?;
    let paused = state.watcher.paused.load(Ordering::Relaxed);
    Menu::with_items(
        app,
        &[
            &MenuItem::with_id(app, "open", tr(&lang, "open"), true, None::<&str>)?,
            &PredefinedMenuItem::separator(app)?,
            &sub,
            &CheckMenuItem::with_id(app, "pause", tr(&lang, "pause"), true, paused, None::<&str>)?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, "exit", tr(&lang, "exit"), true, None::<&str>)?,
        ],
    )
}

fn tooltip() -> String {
    let active = power::active_scheme().map(|g| power::guid_str(&g));
    let name = power::list_schemes().into_iter().find(|s| Some(&s.guid) == active.as_ref()).map(|s| s.name);
    match name {
        Some(n) => format!("CPU Balancer — {n}"),
        None => "CPU Balancer".into(),
    }
}

/// Пересобрать меню и подсказку (после смены схемы/паузы/языка).
pub fn refresh(app: &AppHandle) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        if let Ok(menu) = build_menu(app) {
            let _ = tray.set_menu(Some(menu));
        }
        let _ = tray.set_tooltip(Some(tooltip()));
    }
}

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let menu = build_menu(app)?;
    let mut b = TrayIconBuilder::with_id(TRAY_ID).tooltip(tooltip()).menu(&menu).show_menu_on_left_click(false);
    if let Some(icon) = app.default_window_icon() {
        b = b.icon(icon.clone());
    }
    b.on_menu_event(|app, event| {
        let id = event.id().as_ref();
        match id {
            "open" => show_main(app),
            "exit" => app.exit(0),
            "pause" => {
                let st = app.state::<AppState>();
                let now = !st.watcher.paused.load(Ordering::Relaxed);
                st.set_paused(now);
                refresh(app);
                let _ = app.emit("state-changed", ());
            }
            _ => {
                if let Some(g) = id.strip_prefix("scheme:") {
                    power::set_active(g);
                    refresh(app);
                    let _ = app.emit("state-changed", ());
                }
            }
        }
    })
    .on_tray_icon_event(|tray, event| match event {
        TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } => show_main(tray.app_handle()),
        // схему могли переключить G-Helper/Armoury — обновляем меню при наведении (дёшево, без внешних процессов)
        TrayIconEvent::Enter { .. } => refresh(tray.app_handle()),
        _ => {}
    })
    .build(app)?;
    Ok(())
}
