//! Set length from **ruleset preset name** (preferred) or TOML fallback.
//!
//! Leave in-game First to = **1 Win**. Software set length comes from the
//! selected Versus preset name (`FT2` / `Competitive FT3` / …).

use crate::config;
use crate::logutil;
use crate::set_state;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

static SET_LENGTH: AtomicU32 = AtomicU32::new(0);
static SET_LENGTH_SOURCE_NAME: AtomicBool = AtomicBool::new(false);
static BOOT_DONE: AtomicBool = AtomicBool::new(false);

pub mod offsets {
    pub const GAME_END: usize = 0x14d6590;
    pub const GAME_EXIT: usize = 0x14d7ef0;
    pub const SCENE_TRANSITION: usize = 0x3726120;
    pub const SSS_TO_CSS: usize = 0x23357f8;
    pub const CSS_TO_SSS: usize = 0x2335184;
    /// HW: UTF-16LE ruleset title (e.g. `"Competitive FT3"` / `"FT2"`).
    pub const RULESET_NAME_PRIMARY: usize = 0x530456c;
    pub const RULESET_NAME_SCAN: usize = 0x5304000;
    pub const RULESET_NAME_SCAN_LEN: usize = 0x1000;
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

pub fn set_override(n: u32) {
    SET_LENGTH.store(n, Ordering::SeqCst);
    SET_LENGTH_SOURCE_NAME.store(false, Ordering::SeqCst);
    if n > 0 {
        set_state::set_first_to(n);
    }
}

fn apply_set_length(n: u32, source: &str, name: Option<&str>) {
    let prev = SET_LENGTH.load(Ordering::SeqCst);
    SET_LENGTH.store(n, Ordering::SeqCst);
    SET_LENGTH_SOURCE_NAME.store(source == "name", Ordering::SeqCst);
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

unsafe fn try_name_at(text: usize, rel: usize) -> Option<(String, u32, usize)> {
    let p = (text + rel) as *const u16;
    let s = read_utf16_cstr(p, 40)?;
    if !name_looks_complete(&s) {
        return None;
    }
    let n = parse_ft_from_name(&s)?;
    Some((s, n, rel))
}

unsafe fn scan_for_ruleset_ft_name() -> Option<(String, u32, usize)> {
    let text = text_base();

    if let Some(hit) = try_name_at(text, offsets::RULESET_NAME_PRIMARY) {
        return Some(hit);
    }

    let base = text + offsets::RULESET_NAME_SCAN;
    let end = base + offsets::RULESET_NAME_SCAN_LEN;
    let mut best: Option<(String, u32, usize)> = None;
    let mut addr = base;
    while addr + 8 < end {
        let p = addr as *const u16;
        if *p == b'F' as u16 && *p.add(1) == b'T' as u16 {
            let mut start = p;
            for _ in 0..32 {
                if (start as usize) <= base {
                    break;
                }
                let prev = start.sub(1);
                let u = *prev;
                if u < 0x20 || u >= 0x7f {
                    break;
                }
                start = prev;
            }
            let rel = (start as usize).saturating_sub(text);
            if let Some((s, n, rel)) = try_name_at(text, rel) {
                let better = match &best {
                    None => true,
                    Some((prev, _, _)) => s.len() > prev.len(),
                };
                if better {
                    best = Some((s, n, rel));
                }
            }
        }
        addr += 2;
    }
    best
}

/// Refresh set_length from ruleset name when entering MeleeRuleScene.
pub unsafe fn on_melee_rule_scene() {
    if !config::get().enabled {
        return;
    }

    if let Some((name, n, rel)) = scan_for_ruleset_ft_name() {
        logutil::write(&format!(
            "ruleset_name: \"{}\" (@{:#x}) → set_length={}",
            name, rel, n
        ));
        apply_set_length(n, "name", Some(&name));
    } else {
        logutil::write(&format!(
            "ruleset_name: miss — toml set_length={}",
            set_length()
        ));
    }

    if set_length() > 1 {
        set_state::set_first_to(set_length());
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
        offsets::RULESET_NAME_PRIMARY
    ));
    set_state::set_probe(Ok(()));
}
