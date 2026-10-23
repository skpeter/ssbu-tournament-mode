//! Pane-bind HUD is **disabled**. Calling `pane_set_text_string` on `info_melee`
//! (`txt_gsp`) Data Aborted on Melee enter (Atmosphere 2168-0002).
//! Score overlay is imgui-smash — see `score_overlay.rs`.

use crate::logutil;

pub fn install() {
    logutil::write(
        "hud_layout: pane bind OFF (Data Abort on set_text); overlay via imgui-smash",
    );
}

pub fn on_melee_enter() {}

pub fn on_melee_leave() {}
