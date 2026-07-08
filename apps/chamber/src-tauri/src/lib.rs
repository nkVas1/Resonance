//! Chamber backend: typed commands over the tuner plane, the conductor
//! automation loop, tray remote control, global hotkeys, and the revert-guard
//! timer. The UI is a pure renderer of `Snapshot`.

use conductor::{Action, Cause, Engine, Rule, Trigger};
use resonance_core::config::Config;
use resonance_core::rules::PowerSource;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, State};

// ---------- view models (serialized to the UI) ----------

#[derive(Serialize, Clone, Copy)]
struct ModeView {
    width: u32,
    height: u32,
    hz: u32,
}

impl From<resonance_core::Mode> for ModeView {
    fn from(m: resonance_core::Mode) -> Self {
        ModeView {
            width: m.width,
            height: m.height,
            hz: m.hz,
        }
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
struct RuleView {
    name: String,
    trigger: String,
    profile: String,
    priority: i32,
    /// True if this rule is the one currently driving the display.
    active: bool,
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
    automation_enabled: bool,
    /// Human-readable reason the current profile is active, e.g.
    /// "pinned" or "rule: Photoshop gets DLDSR". None when at native/idle.
    active_cause: Option<String>,
    pinned: Option<String>,
    rules: Vec<RuleView>,
}

#[derive(Serialize, Clone, Copy)]
struct RevertTick {
    remaining: u32,
}

// ---------- shared state ----------

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

/// Automation coordinator shared between the poll loop and IPC commands.
#[derive(Default)]
struct Coordinator {
    engine: Mutex<Engine>,
    /// Profile the user pinned manually (session-only). Beats rule-based logic.
    pin: Mutex<Option<String>>,
    /// Reason the current non-native state is active, for the "why?" readout.
    cause: Mutex<Option<String>>,
    /// Serializes display mutations between the loop and manual commands.
    apply_lock: Mutex<()>,
}

fn cause_text(cause: &Cause) -> String {
    match cause {
        Cause::Pin => "pinned".to_string(),
        Cause::Rule(name) => format!("rule: {name}"),
    }
}

fn trigger_text(t: &Trigger) -> String {
    match t {
        Trigger::Foreground(app) => format!("when {app} is focused"),
        Trigger::Running(app) => format!("while {app} runs"),
        Trigger::Power(PowerSource::Battery) => "on battery".to_string(),
        Trigger::Power(PowerSource::Ac) => "on AC power".to_string(),
    }
}

// ---------- snapshot assembly ----------

fn build_snapshot(app: &AppHandle) -> Result<Snapshot, String> {
    let coord: State<Coordinator> = app.state();
    let state = tuner::state()?;
    let native = tuner::display::native_resolution()?;
    let (adapter, monitor) = tuner::display::device_names()?;
    let config = Config::load_or_init(native.0, native.1)?;
    let pin = coord.pin.lock().expect("pin lock").clone();
    let cause = coord.cause.lock().expect("cause lock").clone();
    let active_profile = coord
        .engine
        .lock()
        .expect("engine lock")
        .active()
        .map(str::to_string);

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
            ProfileView {
                name: p.name.clone(),
                available,
                active,
                mode,
                scale,
                ratio,
            }
        })
        .collect();

    let rules = config
        .rules
        .iter()
        .map(|r| RuleView {
            name: r.name.clone(),
            trigger: trigger_text(&r.trigger),
            profile: r.profile.clone(),
            priority: r.priority,
            active: active_profile.as_deref() == Some(r.profile.as_str())
                && matches!(cause.as_deref(), Some(c) if c.starts_with("rule:")),
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
        automation_enabled: config.automation_enabled,
        active_cause: cause,
        pinned: pin,
        rules,
    })
}

fn broadcast(app: &AppHandle) {
    if let Ok(snap) = build_snapshot(app) {
        let _ = app.emit("snapshot", snap);
    }
}

// ---------- apply helpers ----------

/// Apply a profile without the confirm countdown (used by automation & hotkeys).
/// Guarded so a crash mid-switch still restores, then immediately confirmed.
fn apply_named_silent(app: &AppHandle, name: &str, cause: Cause) -> Result<(), String> {
    let coord: State<Coordinator> = app.state();
    let _lock = coord.apply_lock.lock().expect("apply lock");
    let native = tuner::display::native_resolution()?;
    let config = Config::load_or_init(native.0, native.1)?;
    let profile = config
        .profile(name)
        .ok_or_else(|| format!("unknown profile '{name}'"))?;
    let target = tuner::resolve(profile)?;
    tuner::apply_guarded(target)?;
    tuner::confirm()?;
    *coord.cause.lock().expect("cause lock") = if profile.is_native() {
        None
    } else {
        Some(cause_text(&cause))
    };
    Ok(())
}

fn go_native(app: &AppHandle) -> Result<(), String> {
    let coord: State<Coordinator> = app.state();
    let _lock = coord.apply_lock.lock().expect("apply lock");
    tuner::revert()?;
    let target = tuner::native_state()?;
    tuner::apply(target)?;
    resonance_core::guard::clear()?;
    *coord.cause.lock().expect("cause lock") = None;
    Ok(())
}

// ---------- automation loop ----------

/// Poll system state once a second, evaluate rules, and drive the display.
fn spawn_automation_loop(app: AppHandle) {
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_secs(1));

        let native = match tuner::display::native_resolution() {
            Ok(n) => n,
            Err(_) => continue,
        };
        let config = match Config::load_or_init(native.0, native.1) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let coord: State<Coordinator> = app.state();
        let pin = coord.pin.lock().expect("pin lock").clone();
        let needs_running = conductor::needs_running(&config.rules);
        let sys = conductor::watch::sample(needs_running, pin);

        let action = {
            let mut engine = coord.engine.lock().expect("engine lock");
            engine.set_enabled(config.automation_enabled);
            engine.evaluate(&config.rules, &sys)
        };

        match action {
            Action::Apply { profile, cause } => {
                if let Err(e) = apply_named_silent(&app, &profile, cause) {
                    eprintln!("[automation] apply {profile} failed: {e}");
                }
                broadcast(&app);
            }
            Action::RestoreBaseline => {
                if let Err(e) = go_native(&app) {
                    eprintln!("[automation] restore failed: {e}");
                }
                broadcast(&app);
            }
            Action::Idle => {}
        }
    });
}

// ---------- commands ----------

#[tauri::command]
fn snapshot(app: AppHandle) -> Result<Snapshot, String> {
    build_snapshot(&app)
}

/// Manual switch from the UI: pins the profile (so automation respects the
/// user's explicit choice) and applies it with the safety countdown.
#[tauri::command]
fn apply_profile(app: AppHandle, name: String) -> Result<Snapshot, String> {
    let coord: State<Coordinator> = app.state();
    let native = tuner::display::native_resolution()?;
    let config = Config::load_or_init(native.0, native.1)?;
    let profile = config
        .profile(&name)
        .ok_or_else(|| format!("unknown profile '{name}'"))?;
    let is_native = profile.is_native();
    let target = tuner::resolve(profile)?;

    {
        let _lock = coord.apply_lock.lock().expect("apply lock");
        tuner::apply_guarded(target)?;
        // Mark the engine so the poll loop treats this as already-applied.
        coord
            .engine
            .lock()
            .expect("engine lock")
            .force_active(if is_native { None } else { Some(name.clone()) });
        if is_native {
            *coord.pin.lock().expect("pin lock") = None;
            *coord.cause.lock().expect("cause lock") = None;
        } else {
            *coord.pin.lock().expect("pin lock") = Some(name.clone());
            *coord.cause.lock().expect("cause lock") = Some("pinned".into());
        }
    }

    // A guard is pending only when the mode actually changed to a non-native
    // target; that's exactly when the confirm countdown should run.
    if is_native {
        tuner::confirm()?;
    } else if resonance_core::guard::pending()?.is_some() {
        start_guard_countdown(app.clone(), config.confirm_timeout_s);
    }
    broadcast(&app);
    build_snapshot(&app)
}

/// Clear a manual pin and let automation resume (or fall back to native).
#[tauri::command]
fn resume_automation(app: AppHandle) -> Result<Snapshot, String> {
    let coord: State<Coordinator> = app.state();
    coord.cancel_pin();
    go_native(&app)?;
    broadcast(&app);
    build_snapshot(&app)
}

#[tauri::command]
fn confirm_state(app: AppHandle) -> Result<Snapshot, String> {
    let timer: State<GuardTimer> = app.state();
    timer.cancel_current();
    tuner::confirm()?;
    broadcast(&app);
    build_snapshot(&app)
}

#[tauri::command]
fn revert_now(app: AppHandle) -> Result<Snapshot, String> {
    let timer: State<GuardTimer> = app.state();
    timer.cancel_current();
    let coord: State<Coordinator> = app.state();
    coord.cancel_pin();
    tuner::revert()?;
    *coord.cause.lock().expect("cause lock") = None;
    broadcast(&app);
    build_snapshot(&app)
}

#[tauri::command]
fn set_automation(app: AppHandle, enabled: bool) -> Result<Snapshot, String> {
    let native = tuner::display::native_resolution()?;
    let mut config = Config::load_or_init(native.0, native.1)?;
    config.set_automation(enabled)?;
    broadcast(&app);
    build_snapshot(&app)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct NewRule {
    name: String,
    /// "foreground" | "running" | "power"
    kind: String,
    /// app image name, or "ac" | "battery" for power triggers
    value: String,
    profile: String,
    #[serde(default)]
    priority: i32,
}

#[tauri::command]
fn add_rule(app: AppHandle, rule: NewRule) -> Result<Snapshot, String> {
    let trigger = match rule.kind.as_str() {
        "foreground" => Trigger::Foreground(rule.value.trim().to_ascii_lowercase()),
        "running" => Trigger::Running(rule.value.trim().to_ascii_lowercase()),
        "power" => match rule.value.as_str() {
            "battery" => Trigger::Power(PowerSource::Battery),
            "ac" => Trigger::Power(PowerSource::Ac),
            other => {
                return Err(format!(
                    "power trigger must be 'ac' or 'battery', got '{other}'"
                ))
            }
        },
        other => return Err(format!("unknown trigger kind '{other}'")),
    };
    if rule.name.trim().is_empty() {
        return Err("rule name cannot be empty".into());
    }
    let native = tuner::display::native_resolution()?;
    let mut config = Config::load_or_init(native.0, native.1)?;
    config.upsert_rule(Rule {
        name: rule.name.trim().to_string(),
        trigger,
        profile: rule.profile,
        priority: rule.priority,
        restore: Default::default(),
    })?;
    broadcast(&app);
    build_snapshot(&app)
}

#[tauri::command]
fn remove_rule(app: AppHandle, name: String) -> Result<Snapshot, String> {
    let native = tuner::display::native_resolution()?;
    let mut config = Config::load_or_init(native.0, native.1)?;
    config.remove_rule(&name)?;
    broadcast(&app);
    build_snapshot(&app)
}

impl Coordinator {
    fn cancel_pin(&self) {
        *self.pin.lock().expect("pin lock") = None;
        self.engine.lock().expect("engine lock").force_active(None);
    }
}

// ---------- revert countdown ----------

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
        let coord: State<Coordinator> = app.state();
        coord.cancel_pin();
        let _ = tuner::revert();
        *coord.cause.lock().expect("cause lock") = None;
        broadcast(&app);
    });
}

// ---------- window & tray ----------

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
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    let mut items: Vec<Box<dyn tauri::menu::IsMenuItem<tauri::Wry>>> = vec![
        Box::new(open),
        Box::new(PredefinedMenuItem::separator(app)?),
    ];
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
                    let app = app.clone();
                    let _ = resume_automation(app);
                }
                other => {
                    if let Some(name) = other.strip_prefix("profile:") {
                        let _ = apply_profile(app.clone(), name.to_string());
                        show_main_window(app);
                    }
                }
            }
        })
        .build(app)?;
    Ok(())
}

// ---------- hotkeys ----------

fn register_hotkeys(app: &AppHandle) {
    use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut};

    // Ctrl+Alt+Shift+R — panic to native (always safe).
    let panic = Shortcut::new(
        Some(Modifiers::CONTROL | Modifiers::ALT | Modifiers::SHIFT),
        Code::KeyR,
    );
    // Ctrl+Alt+R — cycle to the next profile.
    let cycle = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyR);

    let handle = app.clone();
    let _ = app
        .global_shortcut()
        .on_shortcuts([panic, cycle], move |_app, sc, event| {
            use tauri_plugin_global_shortcut::ShortcutState;
            if event.state() != ShortcutState::Pressed {
                return;
            }
            if *sc == panic {
                let _ = resume_automation(handle.clone());
            } else if *sc == cycle {
                cycle_profile(&handle);
            }
        });
}

/// Advance to the next configured profile (wraps around).
fn cycle_profile(app: &AppHandle) {
    let Ok(native) = tuner::display::native_resolution() else {
        return;
    };
    let Ok(config) = Config::load_or_init(native.0, native.1) else {
        return;
    };
    if config.profiles.is_empty() {
        return;
    }
    let current = tuner::state().ok();
    // Find the profile whose resolved mode matches the current mode, then step.
    let idx = config
        .profiles
        .iter()
        .position(|p| {
            tuner::resolve(p)
                .ok()
                .zip(current)
                .map(|(t, c)| t.mode == c.mode)
                .unwrap_or(false)
        })
        .unwrap_or(0);
    let next = &config.profiles[(idx + 1) % config.profiles.len()];
    let _ = apply_profile(app.clone(), next.name.clone());
}

use tauri_plugin_global_shortcut::GlobalShortcutExt;

// ---------- entry point ----------

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tuner::init();
    let _ = tuner::restore_pending();

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main_window(app);
        }))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(GuardTimer::default())
        .manage(Coordinator::default())
        .setup(|app| {
            let handle = app.handle().clone();
            let native = tuner::display::native_resolution().unwrap_or((1920, 1080));
            if let Ok(config) = Config::load_or_init(native.0, native.1) {
                app.state::<Coordinator>()
                    .engine
                    .lock()
                    .expect("engine lock")
                    .set_enabled(config.automation_enabled);
            }
            build_tray(app)?;
            register_hotkeys(&handle);
            spawn_automation_loop(handle);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            snapshot,
            apply_profile,
            resume_automation,
            confirm_state,
            revert_now,
            set_automation,
            add_rule,
            remove_rule
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running Resonance");
}
