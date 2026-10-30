//! Observe end-of-game; attribute set wins (engine FT1 owns results screen).
//! No score toasts — scoreboard is an in-match overlay.

use crate::logutil;
use crate::ruleset;
use crate::scoreboard;
use crate::set_state;
use crate::strike;
use crate::win;
use std::sync::atomic::{AtomicBool, Ordering};

static WIN_ATTRIBUTED: AtomicBool = AtomicBool::new(false);

#[skyline::hook(offset = 0x14d6590)]
unsafe fn game_end(game_state: u64) {
    logutil::write(&format!(
        "game_end: state={:#x} set_length={} score={} active={}",
        game_state,
        ruleset::set_length(),
        set_state::score_line(),
        strike::api_is_active()
    ));
    call_original!(game_state);

    if strike::api_is_active() && !WIN_ATTRIBUTED.swap(true, Ordering::SeqCst) {
        win::try_attribute_from_fighter_manager();
        // Intentionally no DialogOk / toast — crashes mid-flow and is the wrong UX.
        logutil::write(&format!(
            "scoreboard: set now {} (stay up through results)",
            set_state::score_line()
        ));
    }
    scoreboard::on_game_end();
}

// Not installed. Skyline's trampoline map aborts the process (MapProcessMemory
// 0xD401, then a store to 0x69) once smashline has filled the nearby code
// pool. game_exit only logged and called the original, so leaving it unhooked
// keeps the scoreboard up through results the same way.
#[allow(dead_code)]
#[skyline::hook(offset = 0x14d7ef0)]
unsafe fn game_exit(game_state: u64, arg: u64) {
    logutil::write(&format!(
        "game_exit: state={:#x} arg={:#x} set_length={}",
        game_state,
        arg,
        ruleset::set_length()
    ));
    call_original!(game_state, arg);
}

pub fn note_melee_start() {
    WIN_ATTRIBUTED.store(false, Ordering::SeqCst);
    scoreboard::on_melee_scene();
}

pub fn install() {
    skyline::install_hooks!(game_end);
    logutil::write("results: game_end observe + win attribute (no toast)");
}
