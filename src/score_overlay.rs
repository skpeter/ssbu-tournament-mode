//! In-match set score via imgui-smash (same host plugin as ssbu-online-deluxe).
//!
//! `ig*` comes from `lib/libimgui_smash.a`. Draw/setup registration uses the
//! host NRO exports (LookupSymbol — those did resolve in 0.3.9).

use crate::board_visibility::ScoreMarks;
use crate::logutil;
use crate::scoreboard;
use skyline::nn::ro::LookupSymbol;
use std::os::raw::{c_char, c_void};
use std::sync::atomic::{AtomicBool, Ordering};

#[repr(C)]
#[derive(Clone, Copy)]
struct ImVec2 {
    x: f32,
    y: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct ImVec4 {
    x: f32,
    y: f32,
    z: f32,
    w: f32,
}

#[link(name = "imgui_smash")]
extern "C" {
    fn igSetCurrentContext(ctx: *mut c_void);
    fn igBegin(name: *const c_char, p_open: *mut bool, flags: i32) -> bool;
    fn igEnd();
    fn igTextUnformatted(text: *const c_char, text_end: *const c_char);
    fn igSameLine(offset_from_start_x: f32, spacing: f32);
    fn igPushStyleColor_Vec4(idx: i32, col: ImVec4);
    fn igPopStyleColor(count: i32);
    fn igSetNextWindowPos(pos: ImVec2, cond: i32, pivot: ImVec2);
    fn igSetNextWindowBgAlpha(alpha: f32);
    fn igGetIO() -> *mut ImGuiIO;
    fn igGetWindowDrawList() -> *mut c_void;
    fn igGetCursorScreenPos() -> ImVec2;
    fn igDummy(size: ImVec2);
    fn igBeginGroup();
    fn igEndGroup();
    fn ImDrawList_AddRect(
        draw_list: *mut c_void,
        p_min: ImVec2,
        p_max: ImVec2,
        col: u32,
        rounding: f32,
        flags: i32,
        thickness: f32,
    );
    fn ImDrawList_AddRectFilled(
        draw_list: *mut c_void,
        p_min: ImVec2,
        p_max: ImVec2,
        col: u32,
        rounding: f32,
        flags: i32,
    );
}

#[repr(C)]
struct ImGuiIO {
    config_flags: i32,
    backend_flags: i32,
    display_size: ImVec2,
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

const IMGUI_COL_TEXT: i32 = 0;
const SAME_LINE_SPACING: f32 = 16.0;
const BLOCK: f32 = 14.0;
const BLOCK_GAP: f32 = 4.0;
const GROUP_GAP: f32 = 22.0;
const BLOCK_ROUND: f32 = 2.0;
const BLOCK_THICK: f32 = 2.0;
const THUMB_W: f32 = 64.0;
const THUMB_H: f32 = 36.0;
const THUMB_GAP: f32 = 10.0;
static DRAW_LOGGED: AtomicBool = AtomicBool::new(false);

unsafe fn window_anchor() -> (f32, f32, f32, f32) {
    let io = igGetIO();
    if !io.is_null() {
        let size = (*io).display_size;
        if size.x >= 100.0 && size.y >= 100.0 {
            return (size.x * 0.5, size.y - 24.0, size.x, size.y);
        }
    }
    (960.0, 1080.0 - 24.0, 1920.0, 1080.0)
}

const COLOR_PORT_0: ImVec4 = ImVec4 {
    x: 0.93,
    y: 0.27,
    z: 0.24,
    w: 1.0,
};
const COLOR_PORT_1: ImVec4 = ImVec4 {
    x: 0.27,
    y: 0.52,
    z: 0.95,
    w: 1.0,
};

fn pack_color(color: ImVec4) -> u32 {
    let r = (color.x * 255.0) as u32;
    let g = (color.y * 255.0) as u32;
    let b = (color.z * 255.0) as u32;
    let a = (color.w * 255.0) as u32;
    r | (g << 8) | (b << 16) | (a << 24)
}

unsafe fn paint_marks(marks: ScoreMarks, color: ImVec4, mut same_line: bool, leading_gap: f32) -> bool {
    let list = igGetWindowDrawList();
    if list.is_null() {
        return same_line;
    }
    let total = marks.filled + marks.open;
    let filled_col = pack_color(color);
    for i in 0..total {
        if same_line {
            let gap = if i == 0 { leading_gap } else { BLOCK_GAP };
            igSameLine(0.0, gap);
        }
        same_line = true;
        let pos = igGetCursorScreenPos();
        let min = ImVec2 {
            x: pos.x,
            y: pos.y + 1.0,
        };
        let max = ImVec2 {
            x: pos.x + BLOCK,
            y: pos.y + 1.0 + BLOCK,
        };
        if i < marks.filled {
            ImDrawList_AddRectFilled(list, min, max, filled_col, BLOCK_ROUND, 0);
        } else {
            ImDrawList_AddRect(list, min, max, filled_col, BLOCK_ROUND, 0, BLOCK_THICK);
        }
        igDummy(ImVec2 {
            x: BLOCK,
            y: BLOCK + 1.0,
        });
    }
    same_line
}

unsafe fn paint_thumb(color: ImVec4) {
    let list = igGetWindowDrawList();
    let pos = igGetCursorScreenPos();
    if !list.is_null() {
        let max = ImVec2 {
            x: pos.x + THUMB_W,
            y: pos.y + THUMB_H,
        };
        ImDrawList_AddRect(list, pos, max, pack_color(color), BLOCK_ROUND, 0, BLOCK_THICK);
    }
    igDummy(ImVec2 {
        x: THUMB_W,
        y: THUMB_H,
    });
}

fn winner_color(winner: u8) -> ImVec4 {
    if winner == 0 {
        COLOR_PORT_0
    } else {
        COLOR_PORT_1
    }
}

unsafe fn paint_strip(parts: &scoreboard::DrawParts) {
    for (index, cell) in parts.strip.iter().enumerate() {
        if cell.stage_id == 0 {
            continue;
        }
        if index > 0 {
            igSameLine(0.0, THUMB_GAP);
        }
        let color = winner_color(cell.winner);
        igBeginGroup();
        if let Some(tag) = cell.tag.as_ref() {
            paint(tag, color, false);
        }
        paint_thumb(color);
        igEndGroup();
    }
}

unsafe fn paint(text: &std::ffi::CString, color: ImVec4, same_line: bool) {
    if same_line {
        igSameLine(0.0, SAME_LINE_SPACING);
    }
    igPushStyleColor_Vec4(IMGUI_COL_TEXT, color);
    igTextUnformatted(text.as_ptr(), std::ptr::null());
    igPopStyleColor(1);
}

unsafe extern "C" fn draw() {
    scoreboard::poll_missing_tags();
    let (center_x, bottom_y, width, height) = window_anchor();
    if !DRAW_LOGGED.swap(true, Ordering::Relaxed) {
        logutil::write(&format!(
            "score_overlay: draw {:.0}x{:.0} visible={}",
            width,
            height,
            scoreboard::overlay_visible()
        ));
    }
    if let Some(parts) = scoreboard::draw_parts() {
        igSetNextWindowPos(
            ImVec2 {
                x: center_x,
                y: bottom_y,
            },
            IMGUI_COND_ALWAYS,
            ImVec2 { x: 0.5, y: 1.0 },
        );
        igSetNextWindowBgAlpha(0.45);
        if igBegin(
            b"##tm_score\0".as_ptr() as _,
            std::ptr::null_mut(),
            SCORE_FLAGS,
        ) {
            if !parts.strip.is_empty() {
                paint_strip(&parts);
            }
            let mut follow = false;
            if let Some(tag) = parts.tag0.as_ref() {
                paint(tag, COLOR_PORT_0, false);
                follow = true;
            }
            follow = paint_marks(
                parts.left,
                COLOR_PORT_0,
                follow,
                if follow { SAME_LINE_SPACING } else { 0.0 },
            );
            follow = paint_marks(
                parts.right,
                COLOR_PORT_1,
                follow,
                if follow { GROUP_GAP } else { 0.0 },
            );
            if let Some(tag) = parts.tag1.as_ref() {
                paint(tag, COLOR_PORT_1, follow);
            }
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
    scoreboard::advance_frame();
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
