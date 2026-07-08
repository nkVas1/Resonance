//! drs-probe — Resonance Phase 0 proof-of-concept.
//!
//! Commands:
//!   dump                       list every driver setting id+name, flag DSR-related, show values
//!   inspect <id-hex>           possible values + current value of one setting
//!   set <id-hex> <value-hex>   write DWORD setting into the base profile and save
//!   modes                      list display modes (marks those above native)
//!   switch <w> <h>             dynamic mode switch (non-persisted)
//!   dpi [percent]              get or set primary display scale
//!   cycle <w> <h> <dpi%> [hold_s]   full guarded demo: switch+dpi, hold, auto-revert

mod display;
mod nvapi;

use std::io::Write as _;

fn main() {
    display::enable_dpi_awareness();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = args.first().map(String::as_str).unwrap_or("help");
    let result = match cmd {
        "dump" => cmd_dump(),
        "inspect" => hex_arg(&args, 1).and_then(cmd_inspect),
        "set" => hex_arg(&args, 1).and_then(|id| hex_arg(&args, 2).and_then(|v| cmd_set(id, v))),
        "modes" => cmd_modes(),
        "switch" => num_arg(&args, 1).and_then(|w| num_arg(&args, 2).and_then(|h| cmd_switch(w, h))),
        "dpi-raw" => {
            let n = num_arg(&args, 1).unwrap_or(3) as usize;
            display::dpi_get_raw(n).map(|vals| {
                for (i, v) in vals.iter().enumerate() {
                    println!("payload[{i}] = {v} (0x{v:08X})");
                }
            })
        }
        "dpi" => match num_arg(&args, 1) {
            Ok(p) => display::dpi_set(p).map(|()| println!("scale set to {p}%")),
            Err(_) => display::dpi_get().map(|(min, cur, max)| println!("scale: {cur}% (min {min}%, max {max}%)")),
        },
        "cycle" => {
            let hold = num_arg(&args, 4).unwrap_or(8);
            num_arg(&args, 1).and_then(|w| {
                num_arg(&args, 2).and_then(|h| num_arg(&args, 3).and_then(|d| cmd_cycle(w, h, d, hold)))
            })
        }
        _ => {
            eprintln!("usage: drs-probe dump|inspect <id>|set <id> <val>|modes|switch <w> <h>|dpi [pct]|cycle <w> <h> <dpi> [hold_s]");
            Ok(())
        }
    };
    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn hex_arg(args: &[String], i: usize) -> Result<u32, String> {
    let raw = args.get(i).ok_or(format!("missing argument #{i}"))?;
    u32::from_str_radix(raw.trim_start_matches("0x"), 16).map_err(|e| format!("bad hex '{raw}': {e}"))
}

fn num_arg(args: &[String], i: usize) -> Result<u32, String> {
    let raw = args.get(i).ok_or(format!("missing argument #{i}"))?;
    raw.parse().map_err(|e| format!("bad number '{raw}': {e}"))
}

fn looks_dsr(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n.contains("dsr") || n.contains("super resolution") || n.contains("smooth scaling") || n.contains("hyperscaling")
}

fn cmd_dump() -> Result<(), String> {
    let api = nvapi::NvApi::load()?;
    let settings = api.available_settings()?;
    println!("driver exports {} setting ids\n", settings.len());

    api.with_base_profile(|api, session, profile| {
        let mut hits = Vec::new();
        for (id, name) in &settings {
            let marker = if looks_dsr(name) { " <<< DSR?" } else { "" };
            println!("0x{id:08X}  {name}{marker}");
            if looks_dsr(name) {
                hits.push((*id, name.clone()));
            }
        }
        println!("\n=== DSR candidates: {} ===", hits.len());
        for (id, name) in hits {
            println!("\n-- 0x{id:08X}  {name}");
            match api.setting_values(id) {
                Ok((stype, default, values)) => {
                    println!("   type={stype} default=0x{default:08X}");
                    for v in values {
                        println!("   possible: 0x{v:08X}");
                    }
                }
                Err(e) => println!("   values: {e}"),
            }
            match api.get_dword(session, profile, id)? {
                Some(v) => println!("   current(base profile): 0x{v:08X}"),
                None => println!("   current(base profile): <not set>"),
            }
        }
        Ok(())
    })
}

fn cmd_inspect(id: u32) -> Result<(), String> {
    let api = nvapi::NvApi::load()?;
    match api.setting_values(id) {
        Ok((stype, default, values)) => {
            println!("type={stype} default=0x{default:08X}");
            for v in values {
                println!("possible: 0x{v:08X}");
            }
        }
        Err(e) => println!("values: {e}"),
    }
    api.with_base_profile(|api, session, profile| {
        match api.get_dword(session, profile, id)? {
            Some(v) => println!("current: 0x{v:08X}"),
            None => println!("current: <not set>"),
        }
        Ok(())
    })
}

fn cmd_set(id: u32, value: u32) -> Result<(), String> {
    let api = nvapi::NvApi::load()?;
    api.with_base_profile(|api, session, profile| {
        let before = api.get_dword(session, profile, id)?;
        api.set_dword(session, profile, id, value)?;
        println!("0x{id:08X}: {:?} -> 0x{value:08X} (saved)", before.map(|v| format!("0x{v:08X}")));
        Ok(())
    })
}

fn cmd_modes() -> Result<(), String> {
    let native = display::current_mode()?;
    println!("current: {}x{} @{}Hz\n", native.width, native.height, native.hz);
    for m in display::list_modes() {
        if m.bpp != 32 {
            continue;
        }
        let above = if m.width > native.width || m.height > native.height { "  <-- ABOVE NATIVE" } else { "" };
        println!("{:>5}x{:<5} @{:>3}Hz{above}", m.width, m.height, m.hz);
    }
    Ok(())
}

fn cmd_switch(w: u32, h: u32) -> Result<(), String> {
    let picked = display::switch_mode(w, h)?;
    println!("switched to {}x{} @{}Hz (dynamic, not persisted)", picked.width, picked.height, picked.hz);
    Ok(())
}

fn cmd_cycle(w: u32, h: u32, dpi: u32, hold_s: u32) -> Result<(), String> {
    let original = display::current_mode()?;
    let (_, original_dpi, _) = display::dpi_get()?;
    println!("original: {}x{} @{}Hz, scale {original_dpi}%", original.width, original.height, original.hz);

    let picked = display::switch_mode(w, h)?;
    println!("switched: {}x{} @{}Hz", picked.width, picked.height, picked.hz);
    if let Err(e) = display::dpi_set(dpi) {
        eprintln!("dpi set failed ({e}) — continuing, will still revert");
    } else {
        println!("scale: {dpi}%");
    }

    for remaining in (1..=hold_s).rev() {
        print!("\rreverting in {remaining:>2}s ");
        std::io::stdout().flush().ok();
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    println!();

    display::switch_mode(original.width, original.height)?;
    display::dpi_set(original_dpi)?;
    println!("reverted to {}x{} @{}Hz, scale {original_dpi}%", original.width, original.height, original.hz);
    Ok(())
}
