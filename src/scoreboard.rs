//! In-match set scoreboard (Phase D).
//!
//! Score lives in `set_state`. This module owns the HUD-facing string and
//! Melee enter/leave visibility. Drawing is a real layout / battle HUD path —
//! **not** an end-of-game toast and **not** skyline-web.

use crate::config;
use crate::logutil;
use crate::ruleset;
use crate::set_state;
use std::sync::atomic::{AtomicBool, Ordering};

static IN_MELEE: AtomicBool = AtomicBool::new(false);

pub fn install() {
    if config::get().scoreboard {
        logutil::write(
            "scoreboard: Phase D — imgui overlay (libimgui_smash.nro); no pane bind",
        );
    } else {
        logutil::write("scoreboard: disabled by config");
    }
}

pub fn on_melee_enter() {
    IN_MELEE.store(true, Ordering::SeqCst);
    crate::hud_layout::on_melee_enter();
    if !should_show() {
        return;
    }
    logutil::write(&format!(
        "scoreboard: show in-match HUD text='{}'",
        hud_text()
    ));
}

pub fn on_melee_leave() {
    crate::hud_layout::on_melee_leave();
    if IN_MELEE.swap(false, Ordering::SeqCst) && config::get().scoreboard {
        logutil::write("scoreboard: hide in-match HUD");
    }
}

pub fn should_show() -> bool {
    config::get().scoreboard
        && ruleset::set_length() > 1
        && set_state::snapshot().probe_ok
}

/// Compact W–W for HUD digits (e.g. `0 — 1`).
pub fn hud_text() -> String {
    let s = set_state::snapshot();
    format!("{} — {}", s.wins[0], s.wins[1])
}

/// Longer line for logs / SSS chrome.
pub fn display_line() -> String {
    format!("SET {} (FT{})", set_state::score_line(), ruleset::set_length())
}

pub fn current_text() -> String {
    hud_text()
}

pub fn is_in_melee() -> bool {
    IN_MELEE.load(Ordering::SeqCst)
}
