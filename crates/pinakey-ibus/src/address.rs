//! Dò tìm địa chỉ D-Bus của IBus — chuyển thể từ `common.go` của goibus.

use std::env;
use std::fs;

pub fn ibus_address() -> std::io::Result<String> {
    if let Ok(addr) = env::var("IBUS_ADDRESS") {
        if !addr.is_empty() {
            return Ok(addr);
        }
    }
    let data = fs::read_to_string(socket_path()?)?;
    for line in data.lines() {
        if let Some(rest) = line.strip_prefix("IBUS_ADDRESS=") {
            return Ok(rest.to_string());
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "IBUS_ADDRESS not found in socket file",
    ))
}

fn socket_path() -> std::io::Result<String> {
    if let Ok(p) = env::var("IBUS_ADDRESS_FILE") {
        if !p.is_empty() {
            return Ok(p);
        }
    }
    let wayland = env::var("WAYLAND_DISPLAY").unwrap_or_default();
    let (is_wayland, display) = if !wayland.is_empty() {
        (true, wayland)
    } else {
        let d = env::var("DISPLAY").unwrap_or_default();
        let d = if d.is_empty() {
            eprintln!("DISPLAY is empty! Using default DISPLAY (:0.0)");
            ":0.0".to_string()
        } else {
            d
        };
        (false, d)
    };

    let mut hostname = "unix".to_string();
    let display_number;
    if is_wayland {
        display_number = display;
    } else {
        // Định dạng là {hostname}:{displaynumber}.{screennumber}
        let hds: Vec<&str> = display.splitn(2, ':').collect();
        let tail = hds.get(1).copied().unwrap_or("");
        let ds: Vec<&str> = tail.splitn(2, '.').collect();
        if !hds[0].is_empty() {
            hostname = hds[0].to_string();
        }
        display_number = ds.first().copied().unwrap_or("").to_string();
    }
    let p = format!("{}-{}-{}", local_machine_id()?, hostname, display_number);
    Ok(format!("{}/ibus/bus/{}", user_config_dir(), p))
}

fn local_machine_id() -> std::io::Result<String> {
    let id = fs::read_to_string("/var/lib/dbus/machine-id")
        .or_else(|_| fs::read_to_string("/etc/machine-id"))?;
    Ok(id.trim().to_string())
}

fn user_config_dir() -> String {
    match env::var("XDG_CONFIG_HOME") {
        Ok(d) if !d.is_empty() => d,
        _ => format!("{}/.config", env::var("HOME").unwrap_or_default()),
    }
}
