//! resctl — the Resonance command line.
//!
//! Commands:
//!   status                 current mode/scale, guard state
//!   doctor                 capability report for this machine
//!   modes                  driver mode list (above-native flagged)
//!   profiles               configured profiles
//!   apply <profile> [-y]   guarded switch with confirm countdown (-y skips)
//!   native                 back to panel-native + 100%
//!   revert                 restore the guarded previous state
//!   dpi [percent]          get or set display scale

use resonance_core::config::Config;
use std::io::Write as _;

fn main() {
    tuner::init();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = args.first().map(String::as_str).unwrap_or("help");

    // Crash recovery: a leftover guard means a previous process died
    // mid-switch — restore before doing anything else (except for the
    // commands that manage the guard themselves).
    if !matches!(cmd, "revert" | "apply") {
        match tuner::restore_pending() {
            Ok(Some(state)) => eprintln!("[guard] restored unconfirmed state -> {}", state.mode),
            Ok(None) => {}
            Err(e) => eprintln!("[guard] pending restore failed: {e}"),
        }
    }

    let result = match cmd {
        "status" => cmd_status(),
        "doctor" => tuner::doctor::run().map(|r| print!("{r}")),
        "modes" => cmd_modes(),
        "profiles" => cmd_profiles(),
        "apply" => cmd_apply(&args),
        "native" => cmd_native(),
        "revert" => tuner::revert().map(|s| println!("reverted -> {} @ {}%", s.mode, s.scale)),
        "dpi" => cmd_dpi(&args),
        _ => {
            eprintln!("resctl — Resonance CLI");
            eprintln!("usage: resctl status|doctor|modes|profiles|apply <profile> [-y]|native|revert|dpi [pct]");
            Ok(())
        }
    };

    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn load_config() -> Result<Config, String> {
    let (w, h) = tuner::display::native_resolution()?;
    Config::load_or_init(w, h)
}

fn cmd_status() -> Result<(), String> {
    let state = tuner::state()?;
    let native = tuner::display::native_resolution()?;
    let super_res = state.mode.width > native.0 || state.mode.height > native.1;
    println!(
        "mode  : {}{}",
        state.mode,
        if super_res {
            "  [super-resolution]"
        } else {
            ""
        }
    );
    println!("scale : {}%", state.scale);
    println!("native: {}x{}", native.0, native.1);
    match resonance_core::guard::pending()? {
        Some(p) => println!(
            "guard : pending revert -> {} @ {}%",
            p.saved.mode, p.saved.scale
        ),
        None => println!("guard : clear"),
    }
    Ok(())
}

fn cmd_modes() -> Result<(), String> {
    let native = tuner::display::native_resolution()?;
    let current = tuner::display::current_mode()?;
    for m in tuner::display::list_modes() {
        let mark = if m == current {
            "  <- current"
        } else if m.width > native.0 || m.height > native.1 {
            "  [super]"
        } else {
            ""
        };
        println!("{m}{mark}");
    }
    Ok(())
}

fn cmd_profiles() -> Result<(), String> {
    let config = load_config()?;
    for p in &config.profiles {
        let target = tuner::resolve(p);
        match target {
            Ok(t) => println!("{:<14} -> {} @ {}%", p.name, t.mode, t.scale),
            Err(e) => println!("{:<14} -> unavailable: {e}", p.name),
        }
    }
    Ok(())
}

fn cmd_apply(args: &[String]) -> Result<(), String> {
    let name = args.get(1).ok_or("usage: resctl apply <profile> [-y]")?;
    let skip_confirm = args.iter().any(|a| a == "-y" || a == "--yes");
    let config = load_config()?;
    let profile = config
        .profile(name)
        .ok_or_else(|| format!("no profile '{name}' (see: resctl profiles)"))?;
    let target = tuner::resolve(profile)?;

    let previous = tuner::apply_guarded(target)?;
    if previous == target {
        println!("already at {} @ {}%", target.mode, target.scale);
        return tuner::confirm();
    }
    let actual = tuner::state()?;
    println!(
        "applied {} @ {}% (was {} @ {}%)",
        actual.mode, actual.scale, previous.mode, previous.scale
    );

    if skip_confirm {
        tuner::confirm()?;
        return Ok(());
    }

    // Interactive countdown: Enter keeps the new state, timeout reverts.
    let timeout = config.confirm_timeout_s;
    let (tx, rx) = std::sync::mpsc::channel::<()>();
    std::thread::spawn(move || {
        let mut line = String::new();
        // Only a real keypress confirms; EOF (piped/closed stdin) must not,
        // otherwise non-interactive runs would silently skip the guard.
        if matches!(std::io::stdin().read_line(&mut line), Ok(n) if n > 0) {
            let _ = tx.send(());
        }
    });
    for remaining in (1..=timeout).rev() {
        print!("\rkeep this mode? press Enter to confirm — auto-revert in {remaining:>2}s ");
        std::io::stdout().flush().ok();
        if rx.recv_timeout(std::time::Duration::from_secs(1)).is_ok() {
            println!("\nconfirmed.");
            return tuner::confirm();
        }
    }
    println!("\nno confirmation — reverting.");
    let restored = tuner::revert()?;
    println!("reverted -> {} @ {}%", restored.mode, restored.scale);
    Ok(())
}

fn cmd_native() -> Result<(), String> {
    let target = tuner::native_state()?;
    tuner::apply(target)?;
    resonance_core::guard::clear()?;
    println!("native -> {} @ {}%", target.mode, target.scale);
    Ok(())
}

fn cmd_dpi(args: &[String]) -> Result<(), String> {
    match args.get(1) {
        Some(raw) => {
            let pct: u32 = raw
                .parse()
                .map_err(|e| format!("bad percent '{raw}': {e}"))?;
            tuner::dpi::set_scale(pct)?;
            println!("scale set to {pct}%");
            Ok(())
        }
        None => {
            let (min, max) = tuner::dpi::scale_range()?;
            println!(
                "scale: {}% (supported {min}%..{max}%)",
                tuner::display::current_scale()?
            );
            Ok(())
        }
    }
}
