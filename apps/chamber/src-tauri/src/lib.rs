//! Chamber backend: typed commands over the tuner plane, tray remote control,
//! and the revert-guard timer. The UI is a pure renderer of `Snapshot`.

use resonance_core::config::Config;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, State};

#[derive(Serialize, Clone, Copy)]
struct ModeView {
    width: u32,
    height: u32,
    hz: u32,
}

impl From<resonance_core::Mode> for ModeView {
    fn from(m: resonance_core::Mode) -> Self {
        ModeView { width: m.width, height: m.height, hz: m.hz }
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ProfileView {
    name: String,
    available: bool,
    active: bool,
    mode: Option<ModeView>,
    scale: Option<u32>,
    ratio: f64,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Snapshot {
    mode: ModeView,
    scale: u32,
    native: (u32, u32),
    super_res: bool,
    guard_pending: bool,
    adapter: String,
    monitor: String,
    confirm_timeout: u32,
    profiles: Vec<ProfileView>,
}

#[derive(Serialize, Clone, Copy)]
struct RevertTick {
    remaining: u32,
}

/// Cancellation handle for the currently running revert countdown, if any.
#[derive(Default)]
struct GuardTimer(Mutex<Option<Arc<AtomicBool>>>);

impl GuardTimer {
    fn cancel_current(&self) {
        if let Some(flag) = self.0.lock().expect("guard timer lock").take() {
            flag.store(true, Ordering::SeqCst);
        }
    }

    fn arm(&self) -> Arc<AtomicBool> {
        let flag = Arc::new(AtomicBool::new(false));
        let mut slot = self.0.lock().expect("guard timer lock");
        if let Some(old) = slot.take() {
            old.store(true, Ordering::SeqCst);
        }
        *slot = Some(flag.clone());
        flag
    }
}

fn build_snapshot() -> Result<Snapshot, String> {
    let state = tuner::state()?;
    let native = tuner::display::native_resolution()?;
    let (adapter, monitor) = tuner::display::device_names()?;
    let config = Config::load_or_init(native.0, native.1)?;

    let profiles = config
        .profiles
        .iter()
        .map(|p| {
            let resolved = tuner::resolve(p);
            let (available, mode, scale) = match &resolved {
                Ok(t) => (true, Some(ModeView::from(t.mode)), Some(t.scale)),
                Err(_) => (false, None, None),
            };
            let active = resolved
                .as_ref()
                .map(|t| t.mode == state.mode)
                .unwrap_or(false);
            let ratio = p
                .resolution
                .map(|(w, _)| w as f64 / native.0 as f64)
                .unwrap_or(1.0);
            ProfileView { name: p.name.clone(), available, active, mode, scale, ratio }
        })
        .collect();

    Ok(Snapshot {
        mode: state.mode.into(),
        scale: state.scale,
        native,
        super_res: state.mode.width > native.0 || state.mode.height > native.1,
        guard_pending: resonance_core::guard::pending()?.is_some(),
        adapter,
        monitor,
        confirm_timeout: config.confirm_timeout_s,
        profiles,
    })
}

fn broadcast(app: &AppHandle) {
    if let Ok(snap) = build_snapshot() {
        let _ = app.emit("snapshot", snap);
    }
}

/// Start the confirm-or-revert countdown after a guarded switch.
fn start_guard_countdown(app: AppHandle, secs: u32) {
    let timer: State<GuardTimer> = app.state();
    let cancel = timer.arm();
    std::thread::spawn(move || {
        for remaining in (1..=secs).rev() {
            if cancel.load(Ordering::SeqCst) {
                return;
            }
            let _ = app.emit("revert-tick", RevertTick { remaining });
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
        if cancel.load(Ordering::SeqCst) {
            return;
        }
        let _ = tuner::revert();
        broadcast(&app);
    });
}

#[tauri::command]
fn snapshot() -> Result<Snapshot, String> {
    build_snapshot()
}

#[tauri::command]
fn apply_profile(app: AppHandle, name: String) -> Result<Snapshot, String> {
    let native = tuner::display::native_resolution()?;
    let config = Config::load_or_init(native.0, native.1)?;
    let profile = config
        .profile(&name)
        .ok_or_else(|| format!("unknown profile '{name}'"))?;
    let target = tuner::resolve(profile)?;
    let previous = tuner::apply_guarded(target)?;

    if previous == target {
        tuner::confirm()?;
    } else if profile.is_native() {
        // Returning to native never needs confirmation — it is the safe state.
        tuner::confirm()?;
    } else {
        start_guard_countdown(app.clone(), config.confirm_timeout_s);
    }
    broadcast(&app);
    build_snapshot()
}

#[tauri::command]
fn confirm_state(app: AppHandle) -> Result<Snapshot, String> {
    let timer: State<GuardTimer> = app.state();
    timer.cancel_current();
    tuner::confirm()?;
    broadcast(&app);
    build_snapshot()
}

#[tauri::command]
fn revert_now(app: AppHandle) -> Result<Snapshot, String> {
    let timer: State<GuardTimer> = app.state();
    timer.cancel_current();
    tuner::revert()?;
    broadcast(&app);
    build_snapshot()
}

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn build_tray(app: &tauri::App) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open Resonance", true, None::<&str>)?;
    let native = MenuItem::with_id(app, "native", "Back to native", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    // Profile entries come from config; ids are namespaced "profile:<name>".
    let mut items: Vec<Box<dyn tauri::menu::IsMenuItem<tauri::Wry>>> =
        vec![Box::new(open), Box::new(sep)];
    if let Ok((w, h)) = tuner::display::native_resolution() {
        if let Ok(config) = Config::load_or_init(w, h) {
            for p in &config.profiles {
                let item = MenuItem::with_id(
                    app,
                    format!("profile:{}", p.name),
                    &p.name,
                    true,
                    None::<&str>,
                )?;
                items.push(Box::new(item));
            }
        }
    }
    items.push(Box::new(PredefinedMenuItem::separator(app)?));
    items.push(Box::new(native));
    items.push(Box::new(quit));

    let refs: Vec<&dyn tauri::menu::IsMenuItem<tauri::Wry>> =
        items.iter().map(|i| i.as_ref()).collect();
    let menu = Menu::with_items(app, &refs)?;

    TrayIconBuilder::with_id("resonance-tray")
        .icon(app.default_window_icon().expect("window icon").clone())
        .tooltip("Resonance")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| {
            let id = event.id.as_ref();
            match id {
                "open" => show_main_window(app),
                "quit" => app.exit(0),
                "native" => {
                    let _ = tuner::revert();
                    broadcast(app);
                }
                other => {
                    if let Some(name) = other.strip_prefix("profile:") {
                        let _ = apply_profile(app.clone(), name.to_string());
                        // Switching from the tray shows the window so the
                        // countdown (and the Keep button) is visible.
                        show_main_window(app);
                    }
                }
            }
        })
        .build(app)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tuner::init();
    // A previous instance may have died mid-switch — restore before UI shows.
    let _ = tuner::restore_pending();

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main_window(app);
        }))
        .manage(GuardTimer::default())
        .setup(|app| {
            build_tray(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            snapshot,
            apply_profile,
            confirm_state,
            revert_now
        ])
        .on_window_event(|window, event| {
            // Close-to-tray: Chamber stays resident.
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running Resonance");
}
