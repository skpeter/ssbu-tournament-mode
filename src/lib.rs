#![feature(proc_macro_hygiene)]

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

fn parse_u32_key(text: &str, key: &str) -> Option<u32> {
    for raw in text.lines() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if let Some(rest) = line.strip_prefix(key) {
            let val = rest
                .trim()
                .trim_start_matches('=')
                .trim()
                .trim_matches('"');
            if let Ok(n) = val.parse::<u32>() {
                return Some(n);
            }
        }
    }
    None
}

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
    if let Ok(text) = std::fs::read_to_string("sd:/ultimate/tournament_mode.toml") {
        let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
        let n = parse_u32_key(text, "set_length")
            .or_else(|| parse_u32_key(text, "first_to_override"));
        if let Some(n) = n {
            ruleset::set_override(n);
            logutil::write(&format!("config: set_length={}", n));
        }
    }

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
