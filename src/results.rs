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
            "scoreboard: set now {} (show in-match next game)",
            set_state::score_line()
        ));
    }
    scoreboard::on_melee_leave();
}

#[skyline::hook(offset = 0x14d7ef0)]
unsafe fn game_exit(game_state: u64, arg: u64) {
    logutil::write(&format!(
        "game_exit: state={:#x} arg={:#x} set_length={}",
        game_state,
        arg,
        ruleset::set_length()
    ));
    call_original!(game_state, arg);
    scoreboard::on_melee_leave();
}

pub fn note_melee_start() {
    WIN_ATTRIBUTED.store(false, Ordering::SeqCst);
    scoreboard::on_melee_enter();
}

pub fn install() {
    skyline::install_hooks!(game_end, game_exit);
    logutil::write("results: game_end observe + win attribute (no toast)");
}
