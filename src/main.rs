#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio;
mod config;

use std::{
    collections::HashSet,
    sync::{
        atomic::{AtomicU8, Ordering},
        Mutex, OnceLock,
    },
    thread,
    time::Duration,
};

use serde::Serialize;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    window::{Color, Effect, EffectsBuilder},
    AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder, WindowEvent, Wry,
};
use windows::Win32::Globalization::GetUserDefaultUILanguage;

use audio::Flow;
use config::Config;

struct AppState {
    config: Mutex<Config>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DeviceView {
    id: String,
    name: String,
    is_default: bool,
    starred: bool,
    volume: Option<f32>,
    muted: Option<bool>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Snapshot {
    input: Vec<DeviceView>,
    output: Vec<DeviceView>,
    auto_switch: bool,
    autostart: bool,
    language: String,
}

static LANGUAGE: AtomicU8 = AtomicU8::new(0);

fn set_language_mode(language: &str) {
    let mode = match language {
        "ja" => 1,
        "en" => 2,
        _ => 0,
    };
    LANGUAGE.store(mode, Ordering::Relaxed);
}

fn tr(ja: &'static str, en: &'static str) -> &'static str {
    static OS_JAPANESE: OnceLock<bool> = OnceLock::new();
    const LANG_JAPANESE: u16 = 0x11;
    let japanese = match LANGUAGE.load(Ordering::Relaxed) {
        1 => true,
        2 => false,
        _ => *OS_JAPANESE.get_or_init(|| unsafe { GetUserDefaultUILanguage() & 0x3ff == LANG_JAPANESE }),
    };
    if japanese { ja } else { en }
}

fn err(e: windows::core::Error) -> String {
    e.message().to_string()
}

fn device_views(flow: Flow, config: &Config) -> Result<Vec<DeviceView>, String> {
    Ok(audio::list(flow)
        .map_err(err)?
        .into_iter()
        .map(|d| {
            let volume = audio::volume(&d.id).ok();
            DeviceView {
                starred: config.starred.contains(&d.name),
                volume: volume.map(|v| v.0),
                muted: volume.map(|v| v.1),
                is_default: d.is_default,
                id: d.id,
                name: d.name,
            }
        })
        .collect())
}

fn snapshot(state: &AppState) -> Result<Snapshot, String> {
    audio::com_init();
    let config = state.config.lock().unwrap().clone();
    Ok(Snapshot {
        input: device_views(Flow::Input, &config)?,
        output: device_views(Flow::Output, &config)?,
        auto_switch: config.auto_switch,
        autostart: config::autostart_enabled(),
        language: config.language,
    })
}

#[tauri::command]
async fn get_state(state: State<'_, AppState>) -> Result<Snapshot, String> {
    snapshot(&state)
}

#[tauri::command]
async fn set_default(id: String, state: State<'_, AppState>) -> Result<Snapshot, String> {
    audio::com_init();
    audio::set_default(&id).map_err(err)?;
    snapshot(&state)
}

#[tauri::command]
async fn toggle_star(name: String, state: State<'_, AppState>) -> Result<Snapshot, String> {
    {
        let mut config = state.config.lock().unwrap();
        match config.starred.iter().position(|n| *n == name) {
            Some(i) => {
                config.starred.remove(i);
            }
            None => config.starred.push(name),
        }
        config.save();
    }
    snapshot(&state)
}

#[tauri::command]
async fn set_auto_switch(enabled: bool, state: State<'_, AppState>) -> Result<Snapshot, String> {
    {
        let mut config = state.config.lock().unwrap();
        config.auto_switch = enabled;
        config.save();
    }
    snapshot(&state)
}

#[tauri::command]
async fn set_autostart(enabled: bool, state: State<'_, AppState>) -> Result<Snapshot, String> {
    if !config::set_autostart(enabled) {
        return Err(tr("スタートアップの登録に失敗しました", "Failed to update the startup setting").into());
    }
    snapshot(&state)
}

#[tauri::command]
async fn set_language(language: String, app: AppHandle, state: State<'_, AppState>) -> Result<Snapshot, String> {
    {
        let mut config = state.config.lock().unwrap();
        config.language = language.clone();
        config.save();
    }
    set_language_mode(&language);
    refresh_tray(&app);
    snapshot(&state)
}

#[tauri::command]
async fn set_volume(id: String, level: f32) -> Result<(), String> {
    audio::com_init();
    audio::set_volume(&id, level).map_err(err)
}

#[tauri::command]
async fn set_mute(id: String, muted: bool, state: State<'_, AppState>) -> Result<Snapshot, String> {
    audio::com_init();
    audio::set_mute(&id, muted).map_err(err)?;
    snapshot(&state)
}

fn show_main(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn default_name(devices: &[audio::Device]) -> &str {
    devices.iter().find(|d| d.is_default).map(|d| d.name.as_str()).unwrap_or(tr("なし", "None"))
}

fn update_tray(app: &AppHandle, inputs: &[audio::Device], outputs: &[audio::Device]) {
    if let Some(tray) = app.tray_by_id("main") {
        let tooltip = format!(
            "Mic Switch\n{}: {}\n{}: {}",
            tr("入力", "Input"),
            default_name(inputs),
            tr("出力", "Output"),
            default_name(outputs)
        );
        let _ = tray.set_tooltip(Some(tooltip));
    }
}

fn tray_menu(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let open = MenuItem::with_id(app, "open", tr("開く", "Open"), true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", tr("終了", "Quit"), true, None::<&str>)?;
    Menu::with_items(app, &[&open, &quit])
}

fn refresh_tray(app: &AppHandle) {
    audio::com_init();
    if let (Some(tray), Ok(menu)) = (app.tray_by_id("main"), tray_menu(app)) {
        let _ = tray.set_menu(Some(menu));
    }
    if let (Ok(inputs), Ok(outputs)) = (audio::list(Flow::Input), audio::list(Flow::Output)) {
        update_tray(app, &inputs, &outputs);
    }
}

fn spawn_watcher(app: AppHandle) {
    thread::spawn(move || {
        audio::com_init();
        let mut known: Option<HashSet<String>> = None;
        let mut last_defaults = Vec::new();
        loop {
            if let (Ok(inputs), Ok(outputs)) = (audio::list(Flow::Input), audio::list(Flow::Output)) {
                let ids: HashSet<String> = inputs.iter().chain(&outputs).map(|d| d.id.clone()).collect();
                if let Some(previous) = &known {
                    let config = app.state::<AppState>().config.lock().unwrap().clone();
                    if config.auto_switch {
                        for devices in [&inputs, &outputs] {
                            let arrived = devices
                                .iter()
                                .find(|d| !previous.contains(&d.id) && config.starred.contains(&d.name));
                            if let Some(device) = arrived {
                                let _ = audio::set_default(&device.id);
                            }
                        }
                    }
                }
                let defaults: Vec<Option<String>> = Flow::ALL.into_iter().map(audio::default_id).collect();
                if known.as_ref() != Some(&ids) || defaults != last_defaults {
                    let inputs = audio::list(Flow::Input).unwrap_or(inputs);
                    let outputs = audio::list(Flow::Output).unwrap_or(outputs);
                    update_tray(&app, &inputs, &outputs);
                    let _ = app.emit("devices-changed", ());
                }
                known = Some(ids);
                last_defaults = defaults;
            }
            thread::sleep(Duration::from_millis(1500));
        }
    });
}

fn main() {
    let start_hidden = std::env::args().any(|arg| arg == "--hidden");
    let config = Config::load();
    set_language_mode(&config.language);

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| show_main(app)))
        .manage(AppState { config: Mutex::new(config) })
        .invoke_handler(tauri::generate_handler![
            get_state,
            set_default,
            toggle_star,
            set_auto_switch,
            set_autostart,
            set_language,
            set_volume,
            set_mute
        ])
        .setup(move |app| {
            WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                .title("Mic Switch")
                .inner_size(400.0, 720.0)
                .resizable(false)
                .maximizable(false)
                .decorations(false)
                .transparent(true)
                .shadow(true)
                .effects(EffectsBuilder::new().effect(Effect::Acrylic).color(Color(20, 20, 24, 96)).build())
                .data_directory(config::data_dir().join("webview"))
                .center()
                .visible(!start_hidden)
                .build()?;

            let menu = tray_menu(app.handle())?;
            TrayIconBuilder::with_id("main")
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("Mic Switch")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "open" => show_main(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        show_main(tray.app_handle());
                    }
                })
                .build(app)?;

            spawn_watcher(app.handle().clone());
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .run(tauri::generate_context!())
        .expect("failed to start Mic Switch");
}
