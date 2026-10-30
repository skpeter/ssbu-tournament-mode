//! Character Select before Stage Select.
//!
//! Ported from standalone `css_first` (AGPL-3.0), itself adapted from HDR
//! scene-transition hooks. 13.0.4 uses the standalone offsets. 13.0.5 uses
//! the HDR offsets (stage hooks match smush_info's 13.0.5 set).
//!
//! **Do not also load `libcss_first.nro` or full HDR** — same swap, double-apply
//! restores vanilla order or crashes.

use crate::config;
use crate::hashed_string::HashedString;
use crate::logutil;
use crate::ruleset;
use crate::set_state;
use skyline::hooks::InlineCtx;
use skyline::libc::{c_char, c_void};

static mut CSS_FIRST: bool = false;
static mut SSS_CANCEL_TO_CSS: bool = false;
static mut CSS_CANCEL_TO_LOCAL: bool = false;
static mut IN_LOCAL_WIRELESS: bool = false;

pub fn is_css_first_active() -> bool {
    unsafe { CSS_FIRST }
}

fn display_is_13_0_5() -> bool {
    crate::scoreboard::version_is_13_0_5_or_newer(&skyline::nn::oe::get_display_version())
}

/// Stage-mode byte. Data slides +0x1000 on 13.0.5.
fn stage_mode_flag_offset() -> u64 {
    if display_is_13_0_5() {
        0x530a96c
    } else {
        0x530996c
    }
}

/// Vanilla wants Stage Select first — rewrite destination to Character Select.
unsafe fn sss_to_css(ctx: &InlineCtx) {
    if !config::get().css_first {
        return;
    }
    let hashed_string = ctx.registers[1].x() as *mut HashedString;
    if hashed_string.is_null() {
        return;
    }
    let current_scene = (*hashed_string).as_str();

    if current_scene == "StageSelectScene" {
        CSS_FIRST = true;
        (*hashed_string).set("CharaSelectScene");
        CSS_CANCEL_TO_LOCAL = IN_LOCAL_WIRELESS;
        logutil::write("css_first: SSS→CSS rewrite");
    }
}

/// After Character Select, send players to Stage Select (when SSS applies).
unsafe fn css_to_sss(ctx: &InlineCtx) {
    if !config::get().css_first {
        return;
    }
    let hashed_string = ctx.registers[1].x() as *mut HashedString;
    if hashed_string.is_null() {
        return;
    }
    let current_scene = (*hashed_string).as_str();

    if current_scene == "CharaSelectScene" {
        let text = skyline::hooks::getRegionAddress(skyline::hooks::Region::Text) as u64;
        let flag_ptr = (text + stage_mode_flag_offset()) as *const u8;
        let flag = *flag_ptr.add(3);

        // Stage select modes 0/1/2 show SSS; 3/4 skip it.
        if flag <= 2 {
            (*hashed_string).set("StageSelectScene");
            SSS_CANCEL_TO_CSS = IN_LOCAL_WIRELESS;
            logutil::write("css_first: CSS→SSS rewrite");
        } else {
            CSS_FIRST = false;
        }
    }
}

/// Bookkeeping before the original scene change. Returns the destination name.
unsafe fn prepare_scene(key_struct: *const HashedString) -> String {
    if key_struct.is_null() {
        return String::new();
    }
    let initial_key = (*key_struct).as_str().to_string();
    logutil::write(&format!("scene: {}", initial_key));

    if config::get().css_first {
        if SSS_CANCEL_TO_CSS && initial_key == "MeleeRuleScene" {
            (*(key_struct as *mut HashedString)).set("CharaSelectScene");
            SSS_CANCEL_TO_CSS = false;
            logutil::write("css_first: cancel SSS→CSS (MeleeRule→CharaSelect)");
        } else if CSS_CANCEL_TO_LOCAL && initial_key == "MeleeRuleScene" {
            (*(key_struct as *mut HashedString)).set("LocalTopScene");
            CSS_CANCEL_TO_LOCAL = false;
            logutil::write("css_first: cancel CSS→LocalTop");
        }
    }

    let key_str = (*key_struct).as_str();
    set_state::note_scene(key_str);
    crate::scoreboard::on_scene(key_str);

    if key_str == "MeleeRuleScene"
        || key_str == "CharaSelectScene"
        || key_str == "StageSelectScene"
    {
        ruleset::on_melee_rule_scene();
    }

    if key_str == "MeleeRuleScene" {
        crate::ui_prompt::on_leave_versus();
    } else if key_str == "CharaSelectScene" && ruleset::set_length() > 1 {
        set_state::set_first_to(ruleset::set_length());
    } else if key_str == "MainMenuScene" {
        crate::ui_prompt::on_leave_versus();
    }

    if key_str == "MeleeRuleScene" || key_str == "MainMenuScene" {
        CSS_FIRST = false;
    }

    if key_str.starts_with("Local") {
        IN_LOCAL_WIRELESS = true;
    } else if key_str == "MainMenuScene" || key_str == "MenuSequenceScene" {
        IN_LOCAL_WIRELESS = false;
    }

    if key_str != "StageSelectScene"
        && key_str != "CharaSelectScene"
        && key_str != "MeleeRuleScene"
    {
        SSS_CANCEL_TO_CSS = false;
        CSS_CANCEL_TO_LOCAL = false;
    }

    (*key_struct).as_str().to_string()
}

unsafe fn finish_scene(entered: &str) {
    if entered == "StageSelectScene" {
        crate::ui_prompt::on_stage_select();
    } else if entered == "MeleeScene" {
        crate::results::note_melee_start();
    }
}

#[skyline::hook(offset = 0x23357f8, inline)]
unsafe fn sss_to_css_1304(ctx: &InlineCtx) {
    sss_to_css(ctx);
}

#[skyline::hook(offset = 0x2335c48, inline)]
unsafe fn sss_to_css_1305(ctx: &InlineCtx) {
    sss_to_css(ctx);
}

#[skyline::hook(offset = 0x2335184, inline)]
unsafe fn css_to_sss_1304(ctx: &InlineCtx) {
    css_to_sss(ctx);
}

#[skyline::hook(offset = 0x23355d4, inline)]
unsafe fn css_to_sss_1305(ctx: &InlineCtx) {
    css_to_sss(ctx);
}

/// 13.0.4 scene change. 13.0.5 uses `scene_transition_1305`.
#[skyline::hook(offset = 0x3726120)]
unsafe fn scene_transition_1304(
    list_ptr: *mut c_void,
    key_struct: *const HashedString,
    context_struct: *const HashedString,
    factory: *mut c_void,
) {
    let entered = prepare_scene(key_struct);
    call_original!(list_ptr, key_struct, context_struct, factory);
    finish_scene(&entered);
}

/// 13.0.5 scene change (HDR offsets; stage hooks in this file match that build).
#[skyline::hook(offset = 0x37266d0)]
unsafe fn scene_transition_1305(
    list_ptr: *mut c_void,
    key_struct: *const HashedString,
    context_struct: *const HashedString,
    factory: *mut c_void,
) {
    let entered = prepare_scene(key_struct);
    call_original!(list_ptr, key_struct, context_struct, factory);
    finish_scene(&entered);
}

/// Hide Rules on CSS when CSS is first — button otherwise dumps to main menu.
#[skyline::hook(offset = 0x3771220)]
unsafe fn register_panel_button(
    panel: *mut u64,
    event_code: i32,
    name: *const c_char,
    arg4: u64,
    arg5: u64,
    arg6: u64,
    arg7: u64,
    arg8: u64,
) {
    if config::get().css_first
        && CSS_FIRST
        && !name.is_null()
        && skyline::from_c_str(name as *const u8) == "set_btn_03_rule"
    {
        return;
    }
    call_original!(panel, event_code, name, arg4, arg5, arg6, arg7, arg8);
}

#[skyline::hook(offset = 0x1a2d440, inline)]
unsafe fn css_advance_sfx_hook(_ctx: &mut InlineCtx) {
    // The old body called play_se (text+0x2407280) with X0 from this hook.
    // On Start that pointer is not a live sound engine: play_se data-aborts
    // at address 0. Leave the vanilla call in place.
    crate::scoreboard::on_css_advance();
    if !config::get().css_first {
        return;
    }
    logutil::write(&format!(
        "css_advance: start pressed css_first_active={}",
        CSS_FIRST
    ));
}

#[skyline::hook(offset = 0x1a2d594, inline)]
unsafe fn css_advance_sfx2_hook(_ctx: &mut InlineCtx) {
    if !config::get().css_first {
        return;
    }
}

pub fn install() {
    // Scene and CSS↔SSS addresses differ on 13.0.5. game_end and the CSS
    // start hook are the same on both, and already fired on this console.
    let version = skyline::nn::oe::get_display_version();
    if display_is_13_0_5() {
        skyline::install_hooks!(
            sss_to_css_1305,
            css_to_sss_1305,
            scene_transition_1305,
            css_advance_sfx_hook,
            css_advance_sfx2_hook,
        );
        logutil::write(&format!(
            "css_first: 13.0.5 scene offsets (display {version})"
        ));
    } else {
        skyline::install_hooks!(
            sss_to_css_1304,
            css_to_sss_1304,
            scene_transition_1304,
            register_panel_button,
            css_advance_sfx_hook,
            css_advance_sfx2_hook,
        );
        logutil::write(&format!(
            "css_first: 13.0.4 scene offsets (display {version})"
        ));
    }
    if config::get().css_first {
        logutil::write("css_first: ON — remove libcss_first.nro (same hooks; dual load crashes)");
    } else {
        logutil::write("css_first: OFF — scene logging only");
    }
}
