//! Set length from the selected Versus **ruleset preset name**.
//!
//! Leave in-game First to = **1 Win**. Software set length comes from
//! `FT2` / `Competitive FT3` / … in that name. A name without `FT` leaves
//! the set at length 1.

use crate::config;
use crate::logutil;
use crate::set_state;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

static SET_LENGTH: AtomicU32 = AtomicU32::new(1);
static SET_LENGTH_SOURCE_NAME: AtomicBool = AtomicBool::new(false);
static BOOT_DONE: AtomicBool = AtomicBool::new(false);
static LAST_NAME_LOG: once_cell::sync::Lazy<std::sync::Mutex<String>> =
    once_cell::sync::Lazy::new(|| std::sync::Mutex::new(String::new()));

pub mod offsets {
    /// HW: UTF-16LE ruleset title on 13.0.4 (e.g. `"Competitive FT3"` / `"FT2"`).
    /// 13.0.5 is this plus `0x1000`.
    pub const RULESET_NAME_PRIMARY: usize = 0x530456c;
}

fn text_base() -> usize {
    unsafe { skyline::hooks::getRegionAddress(skyline::hooks::Region::Text) as usize }
}

/// Parse set length from a preset name. Matches `FT2`, `FT 3`, `Midwest FT3`, etc.
pub fn parse_ft_from_name(name: &str) -> Option<u32> {
    let s = name.to_ascii_uppercase();
    let b = s.as_bytes();
    let mut found = None;
    let mut i = 0;
    while i + 2 < b.len() {
        if b[i] == b'F' && b[i + 1] == b'T' {
            let mut j = i + 2;
            while j < b.len() && (b[j] == b' ' || b[j] == b'-' || b[j] == b'_') {
                j += 1;
            }
            if j < b.len() && b[j].is_ascii_digit() {
                let mut n = 0u32;
                while j < b.len() && b[j].is_ascii_digit() {
                    n = n
                        .saturating_mul(10)
                        .saturating_add((b[j] - b'0') as u32);
                    j += 1;
                }
                if (2..=10).contains(&n) {
                    found = Some(n);
                }
            }
        }
        i += 1;
    }
    found
}

fn apply_set_length(n: u32, source: &str, name: Option<&str>) {
    let named = source == "name";
    let prev = SET_LENGTH.load(Ordering::SeqCst);
    let prev_named = SET_LENGTH_SOURCE_NAME.load(Ordering::SeqCst);
    if prev == n && prev_named == named {
        return;
    }
    SET_LENGTH.store(n, Ordering::SeqCst);
    SET_LENGTH_SOURCE_NAME.store(named, Ordering::SeqCst);
    if n > 0 {
        set_state::set_first_to(n);
    }
    if prev != n {
        if let Some(nm) = name {
            logutil::write(&format!(
                "set_length: {}→{} from {} name=\"{}\"",
                prev, n, source, nm
            ));
        } else {
            logutil::write(&format!("set_length: {}→{} from {}", prev, n, source));
        }
    }
}

pub fn current_first_to() -> u32 {
    let n = SET_LENGTH.load(Ordering::SeqCst);
    if n > 0 {
        return n;
    }
    set_state::snapshot().first_to
}

pub fn set_length() -> u32 {
    current_first_to()
}

pub fn set_length_from_name() -> bool {
    SET_LENGTH_SOURCE_NAME.load(Ordering::SeqCst)
}

unsafe fn read_utf16_cstr(ptr: *const u16, max_chars: usize) -> Option<String> {
    if ptr.is_null() || (ptr as usize) < 0x10000 {
        return None;
    }
    let mut units = Vec::with_capacity(max_chars.min(64));
    for i in 0..max_chars {
        let u = *ptr.add(i);
        if u == 0 {
            break;
        }
        if u > 0x7f && u < 0xa0 {
            return None;
        }
        if u >= 0xd800 && u <= 0xdfff {
            return None;
        }
        units.push(u);
    }
    if units.len() < 2 {
        return None;
    }
    Some(String::from_utf16_lossy(&units))
}

fn name_looks_complete(s: &str) -> bool {
    let t = s.trim();
    if t.len() < 3 {
        return false;
    }
    let first = t.chars().next().unwrap();
    first.is_ascii_alphanumeric()
        && t.chars()
            .all(|c| c.is_ascii_alphanumeric() || c.is_ascii_whitespace() || "-_./".contains(c))
}

fn selected_name_rel() -> usize {
    let version = skyline::nn::oe::get_display_version();
    if crate::scoreboard::version_is_13_0_5_or_newer(&version) {
        // 13.0.4 primary plus the 13.0.5 data slide.
        0x530556c
    } else {
        offsets::RULESET_NAME_PRIMARY
    }
}

fn log_name_once(line: &str) {
    let mut last = LAST_NAME_LOG.lock().unwrap();
    if *last == line {
        return;
    }
    *last = line.to_string();
    logutil::write(line);
}

unsafe fn read_selected_name() -> Option<String> {
    let rel = selected_name_rel();
    let p = (text_base() + rel) as *const u16;
    let name = read_utf16_cstr(p, 40)?;
    if name_looks_complete(&name) {
        Some(name)
    } else {
        None
    }
}

/// Read the selected Versus preset name. `FT2` / `FT3` sets the length.
/// Any other name, or an unreadable slot, leaves the set at length 1.
pub unsafe fn on_melee_rule_scene() {
    if !config::get().enabled {
        return;
    }

    let rel = selected_name_rel();
    match read_selected_name() {
        Some(name) => match parse_ft_from_name(&name) {
            Some(n) => {
                log_name_once(&format!(
                    "ruleset_name: \"{}\" (@{:#x}) → set_length={}",
                    name, rel, n
                ));
                apply_set_length(n, "name", Some(&name));
            }
            None => {
                log_name_once(&format!(
                    "ruleset_name: \"{}\" has no FT — set_length=1",
                    name
                ));
                apply_set_length(1, "none", None);
            }
        },
        None => {
            log_name_once(&format!(
                "ruleset_name: unreadable @{:#x} — set_length=1",
                rel
            ));
            apply_set_length(1, "none", None);
        }
    }
}

pub fn boot_probe() {
    if BOOT_DONE.swap(true, Ordering::SeqCst) {
        return;
    }

    let text = text_base();
    if text == 0 {
        set_state::set_probe(Err("text_region_null"));
        return;
    }

    logutil::write(&format!(
        "probe: text={:#x} set_length_from_ruleset_name primary={:#x}",
        text,
        selected_name_rel()
    ));
    set_state::set_probe(Ok(()));
}
