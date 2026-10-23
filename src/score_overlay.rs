//! In-match set score via imgui-smash (same host plugin as ssbu-online-deluxe).
//!
//! `ig*` comes from `lib/libimgui_smash.a`. Draw/setup registration uses the
//! host NRO exports (LookupSymbol — those did resolve in 0.3.9).

use crate::logutil;
use crate::ruleset;
use crate::scoreboard;
use skyline::nn::ro::LookupSymbol;
use std::os::raw::{c_char, c_void};

#[repr(C)]
#[derive(Clone, Copy)]
struct ImVec2 {
    x: f32,
    y: f32,
}

#[link(name = "imgui_smash")]
extern "C" {
    fn igSetCurrentContext(ctx: *mut c_void);
    fn igBegin(name: *const c_char, p_open: *mut bool, flags: i32) -> bool;
    fn igEnd();
    fn igTextUnformatted(text: *const c_char, text_end: *const c_char);
    fn igSetNextWindowPos(pos: ImVec2, cond: i32, pivot: ImVec2);
    fn igSetNextWindowBgAlpha(alpha: f32);
}

const IMGUI_COND_ALWAYS: i32 = 1;
const WIN_NO_TITLEBAR: i32 = 1 << 0;
const WIN_NO_RESIZE: i32 = 1 << 1;
const WIN_NO_MOVE: i32 = 1 << 2;
const WIN_NO_SCROLLBAR: i32 = 1 << 3;
const WIN_NO_COLLAPSE: i32 = 1 << 5;
const WIN_ALWAYS_AUTO_RESIZE: i32 = 1 << 6;
const WIN_NO_SAVED: i32 = 1 << 8;
const WIN_NO_MOUSE: i32 = 1 << 9;
const WIN_NO_FOCUS_APPEAR: i32 = 1 << 12;
const WIN_NO_NAV_INPUTS: i32 = 1 << 16;
const WIN_NO_NAV_FOCUS: i32 = 1 << 17;

const SCORE_FLAGS: i32 = WIN_NO_TITLEBAR
    | WIN_NO_RESIZE
    | WIN_NO_MOVE
    | WIN_NO_SCROLLBAR
    | WIN_NO_COLLAPSE
    | WIN_ALWAYS_AUTO_RESIZE
    | WIN_NO_SAVED
    | WIN_NO_MOUSE
    | WIN_NO_FOCUS_APPEAR
    | WIN_NO_NAV_INPUTS
    | WIN_NO_NAV_FOCUS;

const HIDDEN_FLAGS: i32 = SCORE_FLAGS | (1 << 7);

const HOST_NRO: &str =
    "sd:/atmosphere/contents/01006A800016E000/romfs/skyline/plugins/libimgui_smash.nro";

type SetupCtxFn = unsafe extern "C" fn(*mut u64);
type AddDrawFn = unsafe extern "C" fn(*const c_void);
type AddSetupFn = unsafe extern "C" fn(SetupCtxFn);

unsafe fn lookup(name: &[u8]) -> usize {
    let mut addr = 0usize;
    LookupSymbol(&mut addr, name.as_ptr());
    addr
}

unsafe extern "C" fn setup_ctx(ctx: *mut u64) {
    igSetCurrentContext(ctx as *mut c_void);
}

unsafe extern "C" fn draw() {
    if scoreboard::should_show() && scoreboard::is_in_melee() {
        let line = format!(
            "SET  {}  FT{}\0",
            scoreboard::hud_text().replace(" — ", "-"),
            ruleset::set_length()
        );
        igSetNextWindowPos(
            ImVec2 { x: 960.0, y: 18.0 },
            IMGUI_COND_ALWAYS,
            ImVec2 { x: 0.5, y: 0.0 },
        );
        igSetNextWindowBgAlpha(0.45);
        if igBegin(
            b"##tm_score\0".as_ptr() as _,
            std::ptr::null_mut(),
            SCORE_FLAGS,
        ) {
            igTextUnformatted(line.as_ptr() as _, std::ptr::null());
        }
        igEnd();
    } else {
        igSetNextWindowPos(
            ImVec2 { x: 0.0, y: 0.0 },
            IMGUI_COND_ALWAYS,
            ImVec2 { x: 0.0, y: 0.0 },
        );
        igSetNextWindowBgAlpha(0.0);
        let _ = igBegin(
            b"##tm_hidden\0".as_ptr() as _,
            std::ptr::null_mut(),
            HIDDEN_FLAGS,
        );
        igEnd();
    }
}

pub fn install() {
    if !crate::config::get().scoreboard {
        logutil::write("score_overlay: skipped (scoreboard=false)");
        return;
    }
    if !std::path::Path::new(HOST_NRO).exists() {
        logutil::write(
            "score_overlay: missing libimgui_smash.nro — overlay host not loaded",
        );
        return;
    }

    unsafe {
        let add_draw = lookup(b"imgui_smash_add_on_draw_frame_wrapper\0");
        let add_setup = lookup(b"imgui_setup_context_export\0");
        if add_draw == 0 || add_setup == 0 {
            logutil::write(&format!(
                "score_overlay: host export fail draw={:#x} setup={:#x}",
                add_draw, add_setup
            ));
            return;
        }
        let add_setup: AddSetupFn = std::mem::transmute(add_setup);
        let add_draw: AddDrawFn = std::mem::transmute(add_draw);
        add_setup(setup_ctx);
        add_draw(draw as *const c_void);
    }
    logutil::write("score_overlay: imgui-smash draw registered (linked ig*, host callbacks)");
}
