//! X11 focused-window class detection — pure-Rust port of `x11GetFocusWindowClass`
//! (`x11_introspector.c`) using x11rb instead of Xlib/cgo.

use x11rb::connection::Connection;
use x11rb::protocol::xproto::{AtomEnum, ConnectionExt, Window};

/// Parse the `WM_CLASS` property value (two NUL-separated strings: instance then class).
/// Returns the class (`res_class`) if present, else the instance (`res_name`).
pub fn parse_wm_class(value: &[u8]) -> Option<String> {
    let parts: Vec<&[u8]> = value
        .split(|&b| b == 0)
        .filter(|p| !p.is_empty())
        .collect();
    match parts.len() {
        0 => None,
        1 => Some(String::from_utf8_lossy(parts[0]).into_owned()),
        _ => Some(String::from_utf8_lossy(parts[1]).into_owned()),
    }
}

/// Return the WM_CLASS of the currently focused window, walking up the window tree until a window
/// carrying a WM_CLASS is found (mirrors the C introspector). Returns `None` if not on X11 or no
/// class is found.
pub fn get_focus_window_class() -> Option<String> {
    let (conn, _screen) = x11rb::connect(None).ok()?;
    let focus = conn.get_input_focus().ok()?.reply().ok()?.focus;
    let root = conn.setup().roots.first().map(|s| s.root).unwrap_or(0);

    let mut window: Window = focus;
    for _ in 0..32 {
        if window == 0 {
            break;
        }
        if let Some(class) = read_wm_class(&conn, window) {
            return Some(class);
        }
        if window == root {
            break;
        }
        // walk up to the parent
        match conn.query_tree(window).ok().and_then(|c| c.reply().ok()) {
            Some(tree) => {
                if tree.parent == window {
                    break;
                }
                window = tree.parent;
            }
            None => break,
        }
    }
    None
}

fn read_wm_class<C: Connection>(conn: &C, window: Window) -> Option<String> {
    let reply = conn
        .get_property(false, window, AtomEnum::WM_CLASS, AtomEnum::STRING, 0, 1024)
        .ok()?
        .reply()
        .ok()?;
    if reply.value.is_empty() {
        return None;
    }
    parse_wm_class(&reply.value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_instance_and_class() {
        assert_eq!(parse_wm_class(b"google-chrome\0Google-chrome\0").as_deref(), Some("Google-chrome"));
        assert_eq!(parse_wm_class(b"firefox\0Firefox\0").as_deref(), Some("Firefox"));
    }

    #[test]
    fn parses_single_component() {
        assert_eq!(parse_wm_class(b"xterm\0").as_deref(), Some("xterm"));
    }

    #[test]
    fn empty_is_none() {
        assert_eq!(parse_wm_class(b""), None);
        assert_eq!(parse_wm_class(b"\0\0"), None);
    }
}
