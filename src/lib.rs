#![feature(proc_macro_hygiene)]

mod board_visibility;
mod config;
mod css_first;
mod hashed_string;
mod hud_layout;
mod logutil;
mod lua_api;
mod results;
mod ruleset;
mod score_overlay;
mod scoreboard;
mod set_state;
mod strike;
mod ui_prompt;
mod win;

use skyline::nn::ro::LookupSymbol;

pub static mut FIGHTER_MANAGER_ADDR: usize = 0;

unsafe fn try_lookup_fighter_manager() {
    LookupSymbol(
        &mut FIGHTER_MANAGER_ADDR,
        b"_ZN3lib9SingletonIN3app14FighterManagerEE9instance_E\0".as_ptr(),
    );
    if FIGHTER_MANAGER_ADDR == 0 {
        logutil::write("lookup: FighterManager FAILED");
    } else {
        logutil::write(&format!(
            "lookup: FighterManager {:#x}",
            FIGHTER_MANAGER_ADDR
        ));
    }
}

#[skyline::main(name = "ssbu_tournament_mode")]
pub fn main() {
    logutil::reset("ssbu_tournament_mode boot");
    logutil::write("version=0.3.10 target<=13.0.4");

    config::load();
    scoreboard::warn_if_both_smashline_nros();

    ruleset::boot_probe();
    unsafe {
        try_lookup_fighter_manager();
    }

    let snap = set_state::snapshot();
    if !snap.probe_ok {
        logutil::write(&format!(
            "inert: probe_fail reason={}",
            snap.inert_reason
        ));
        return;
    }

    css_first::install();
    results::install();
    lua_api::install();
    scoreboard::install();
    hud_layout::install();
    score_overlay::install();
    strike::reset_for_game(0);

    logutil::write(&format!(
        "ready: enabled={} css_first={} set_length={} strike={} scoreboard={}",
        config::get().enabled,
        config::get().css_first,
        ruleset::set_length(),
        config::get().strike_preset,
        config::get().scoreboard
    ));
    logutil::write(
        "phase_d: imgui-smash overlay for in-match score (pane bind disabled)",
    );
}
