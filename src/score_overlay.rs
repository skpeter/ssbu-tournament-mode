//! In-match set score via imgui-smash (same host plugin as ssbu-online-deluxe).
//!
//! `ig*` comes from `lib/libimgui_smash.a`. Draw/setup registration uses the
//! host NRO exports (LookupSymbol — those did resolve in 0.3.9).

use crate::board_visibility::{
    cell_filled, hud_font_scale, side_column, stage_box_positions, ScoreMarks,
};
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
    fn igSetNextWindowSize(size: ImVec2, cond: i32);
    fn igSetNextWindowBgAlpha(alpha: f32);
    fn igGetIO() -> *mut ImGuiIO;
    fn igGetStyle() -> *mut c_void;
    fn igGetWindowDrawList() -> *mut c_void;
    fn igGetWindowPos() -> ImVec2;
    fn igGetCursorScreenPos() -> ImVec2;
    fn igGetCursorPos() -> ImVec2;
    fn igGetCursorPosX() -> f32;
    fn igSetCursorPos(pos: ImVec2);
    fn igGetContentRegionAvail() -> ImVec2;
    fn igSetCursorPosX(x: f32);
    fn igCalcTextSize(
        text: *const c_char,
        text_end: *const c_char,
        hide_text_after_double_hash: u8,
        wrap_width: f32,
    ) -> ImVec2;
    fn igPushStyleVar_Vec2(idx: i32, val: ImVec2);
    fn igPopStyleVar(count: i32);
    fn igDummy(size: ImVec2);
    #[link_name = "_ZN5ImGui18SetWindowFontScaleEf"]
    fn set_window_font_scale(scale: f32);
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
const BLOCK: f32 = 14.0;
const BLOCK_GAP: f32 = 4.0;
const GROUP_GAP: f32 = 22.0;
const BLOCK_ROUND: f32 = 2.0;
const BLOCK_THICK: f32 = 2.0;
const STRIP_GAP: f32 = 10.0;
const EDGE_MARGIN: f32 = 24.0;
const NAME_GRID_GAP: f32 = 10.0;
/// Window fill. Dark enough that the text stays readable over the stage.
const WINDOW_BG_ALPHA: f32 = 0.8;
/// imgui 1.92 `ImGuiStyleVar_WindowPadding` / `ImGuiStyleVar_ItemSpacing`.
const STYLE_WINDOW_PADDING: i32 = 2;
const STYLE_ITEM_SPACING: i32 = 14;
/// Byte offsets in imgui 1.92 `ImGuiStyle` (three font floats, then alpha, disabled alpha).
const STYLE_WINDOW_PADDING_OFFSET: usize = 20;
const STYLE_ITEM_SPACING_OFFSET: usize = 92;
const STAGE_FLAGS: i32 = SCORE_FLAGS & !WIN_ALWAYS_AUTO_RESIZE;
static DRAW_LOGGED: AtomicBool = AtomicBool::new(false);

struct Chrome {
    scale: f32,
    pad: ImVec2,
    spacing: ImVec2,
}

/// Center X, top Y, display width, display height. Pivot is top-center.
unsafe fn window_anchor(scale: f32) -> (f32, f32, f32, f32) {
    let margin = EDGE_MARGIN * scale;
    let io = igGetIO();
    if !io.is_null() {
        let size = (*io).display_size;
        if size.x >= 100.0 && size.y >= 100.0 {
            return (size.x * 0.5, margin, size.x, size.y);
        }
    }
    (960.0, margin, 1920.0, 1080.0)
}

unsafe fn display_width() -> f32 {
    let io = igGetIO();
    if !io.is_null() {
        let width = (*io).display_size.x;
        if width >= 100.0 {
            return width;
        }
    }
    1920.0
}

unsafe fn chrome() -> Chrome {
    let scale = hud_font_scale(skyline::nn::oe::GetOperationMode(), display_width());
    let style = igGetStyle();
    if style.is_null() {
        return Chrome {
            scale,
            pad: ImVec2 { x: 8.0, y: 8.0 },
            spacing: ImVec2 { x: 8.0, y: 4.0 },
        };
    }
    let base = style as *const u8;
    Chrome {
        scale,
        pad: sane_vec(read_vec(base, STYLE_WINDOW_PADDING_OFFSET), 8.0, 8.0),
        spacing: sane_vec(read_vec(base, STYLE_ITEM_SPACING_OFFSET), 8.0, 4.0),
    }
}

unsafe fn read_vec(base: *const u8, offset: usize) -> ImVec2 {
    *(base.add(offset) as *const ImVec2)
}

fn sane_vec(value: ImVec2, x: f32, y: f32) -> ImVec2 {
    ImVec2 {
        x: if (0.0..=64.0).contains(&value.x) {
            value.x
        } else {
            x
        },
        y: if (0.0..=64.0).contains(&value.y) {
            value.y
        } else {
            y
        },
    }
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

fn grid_width(marks: ScoreMarks, scale: f32) -> f32 {
    let total = marks.filled + marks.open;
    if total == 0 {
        return 0.0;
    }
    let block = BLOCK * scale;
    let gap = BLOCK_GAP * scale;
    total as f32 * block + (total - 1) as f32 * gap
}

unsafe fn paint_mark_row(
    marks: ScoreMarks,
    color: ImVec4,
    origin: ImVec2,
    scale: f32,
    from_right: bool,
) {
    let list = igGetWindowDrawList();
    if list.is_null() {
        return;
    }
    let block = BLOCK * scale;
    let gap = BLOCK_GAP * scale;
    let round = BLOCK_ROUND * scale;
    let thick = BLOCK_THICK * scale;
    let total = marks.filled + marks.open;
    let filled_col = pack_color(color);
    for i in 0..total {
        igSetCursorPos(ImVec2 {
            x: origin.x + i as f32 * (block + gap),
            y: origin.y,
        });
        let pos = igGetCursorScreenPos();
        let min = pos;
        let max = ImVec2 {
            x: pos.x + block,
            y: pos.y + block,
        };
        if cell_filled(i, marks.filled, total, from_right) {
            ImDrawList_AddRectFilled(list, min, max, filled_col, round, 0);
        } else {
            ImDrawList_AddRect(list, min, max, filled_col, round, 0, thick);
        }
        igDummy(ImVec2 { x: block, y: block });
    }
}

fn stack_size(tags: &[std::ffi::CString], line_gap: f32) -> (f32, f32) {
    let mut width = 0.0f32;
    let mut height = 0.0f32;
    for (i, tag) in tags.iter().enumerate() {
        let size = unsafe { line_px(tag) };
        width = width.max(size.x);
        height += size.y;
        if i + 1 < tags.len() {
            height += line_gap;
        }
    }
    (width, height)
}

unsafe fn paint_name_stack(
    tags: &[std::ffi::CString],
    color: ImVec4,
    origin: ImVec2,
    col_w: f32,
    line_gap: f32,
    align_end: bool,
) {
    let mut y = origin.y;
    for tag in tags {
        let size = line_px(tag);
        let x = if align_end {
            origin.x + (col_w - size.x).max(0.0)
        } else {
            origin.x
        };
        igSetCursorPos(ImVec2 { x, y });
        paint(tag, color, false, 0.0);
        y += size.y + line_gap;
    }
}

unsafe fn paint_scoreboard(parts: &scoreboard::DrawParts, chrome: &Chrome) {
    let scale = chrome.scale;
    let origin = igGetCursorPos();
    let line_gap = chrome.spacing.y * scale;
    let (left_w, left_h) = stack_size(&parts.left_tags, line_gap);
    let (right_w, right_h) = stack_size(&parts.right_tags, line_gap);
    let grid_h = BLOCK * scale;
    let left = side_column(
        left_w,
        left_h,
        grid_width(parts.left, scale),
        grid_h,
        NAME_GRID_GAP * scale,
        true,
    );
    let right = side_column(
        right_w,
        right_h,
        grid_width(parts.right, scale),
        grid_h,
        NAME_GRID_GAP * scale,
        false,
    );
    let right_x = origin.x + left.width + GROUP_GAP * scale;
    let row_h = left.height.max(right.height);
    let left_grid_y = origin.y + (row_h - grid_h) * 0.5;
    let right_grid_y = origin.y + (row_h - grid_h) * 0.5;
    let left_name_y = origin.y + (row_h - left_h) * 0.5;
    let right_name_y = origin.y + (row_h - right_h) * 0.5;

    if !parts.left_tags.is_empty() {
        paint_name_stack(
            &parts.left_tags,
            COLOR_PORT_0,
            ImVec2 {
                x: origin.x + left.name_x,
                y: left_name_y,
            },
            left_w,
            line_gap,
            true,
        );
    }
    if !parts.right_tags.is_empty() {
        paint_name_stack(
            &parts.right_tags,
            COLOR_PORT_1,
            ImVec2 {
                x: right_x + right.name_x,
                y: right_name_y,
            },
            right_w,
            line_gap,
            false,
        );
    }
    paint_mark_row(
        parts.left,
        COLOR_PORT_0,
        ImVec2 {
            x: origin.x + left.grid_x,
            y: left_grid_y,
        },
        scale,
        true,
    );
    paint_mark_row(
        parts.right,
        COLOR_PORT_1,
        ImVec2 {
            x: right_x + right.grid_x,
            y: right_grid_y,
        },
        scale,
        false,
    );
    // Reserve vertical space so auto-resize covers stacked names.
    igSetCursorPos(ImVec2 {
        x: origin.x,
        y: origin.y + row_h,
    });
    igDummy(ImVec2 { x: 1.0, y: 1.0 });
}

fn winner_color(winner: u8) -> ImVec4 {
    if winner == 0 {
        COLOR_PORT_0
    } else {
        COLOR_PORT_1
    }
}

unsafe fn paint(text: &std::ffi::CString, color: ImVec4, same_line: bool, spacing: f32) {
    if same_line {
        igSameLine(0.0, spacing);
    }
    igPushStyleColor_Vec4(IMGUI_COL_TEXT, color);
    igTextUnformatted(text.as_ptr(), std::ptr::null());
    igPopStyleColor(1);
}

unsafe fn paint_centered(text: &std::ffi::CString, color: ImVec4) {
    let measured = igCalcTextSize(text.as_ptr(), std::ptr::null(), 0, -1.0);
    let region = igGetContentRegionAvail();
    if region.x > measured.x {
        igSetCursorPosX(igGetCursorPosX() + (region.x - measured.x) * 0.5);
    }
    paint(text, color, false, 0.0);
}

unsafe fn line_px(text: &std::ffi::CString) -> ImVec2 {
    igCalcTextSize(text.as_ptr(), std::ptr::null(), 0, -1.0)
}

fn outer_size(name: ImVec2, tags: &[ImVec2], chrome: &Chrome) -> (f32, f32) {
    let mut text_w = name.x;
    let mut text_h = name.y;
    let mut gaps = 0usize;
    for tag in tags {
        text_w = text_w.max(tag.x);
        text_h += tag.y;
        gaps += 1;
    }
    let scale = chrome.scale;
    let gap = chrome.spacing.y * gaps as f32;
    (
        (text_w + chrome.pad.x * 2.0) * scale,
        (text_h + gap + chrome.pad.y * 2.0) * scale,
    )
}

fn stage_window_id(index: usize) -> [u8; 16] {
    let mut buf = [0u8; 16];
    let prefix = b"##tm_stage_";
    buf[..prefix.len()].copy_from_slice(prefix);
    let n = index.min(99);
    buf[prefix.len()] = b'0' + (n / 10) as u8;
    buf[prefix.len() + 1] = b'0' + (n % 10) as u8;
    buf
}

unsafe fn paint_stage_row(
    parts: &scoreboard::DrawParts,
    center_x: f32,
    score_bottom: f32,
    chrome: &Chrome,
) {
    let mut sizes = Vec::with_capacity(parts.strip.len());
    for cell in &parts.strip {
        let tag_px: Vec<ImVec2> = cell.tags.iter().map(|tag| line_px(tag)).collect();
        sizes.push(outer_size(line_px(&cell.name), &tag_px, chrome));
    }
    let gap = STRIP_GAP * chrome.scale;
    let positions = stage_box_positions(&sizes, score_bottom, center_x, gap, gap);
    for (index, cell) in parts.strip.iter().enumerate() {
        let Some(pos) = positions.get(index) else {
            break;
        };
        let (w, h) = sizes[index];
        igSetNextWindowPos(
            ImVec2 { x: pos.x, y: pos.y },
            IMGUI_COND_ALWAYS,
            ImVec2 { x: 0.0, y: 0.0 },
        );
        igSetNextWindowSize(ImVec2 { x: w, y: h }, IMGUI_COND_ALWAYS);
        igSetNextWindowBgAlpha(WINDOW_BG_ALPHA);
        let name = stage_window_id(index);
        if igBegin(name.as_ptr() as _, std::ptr::null_mut(), STAGE_FLAGS) {
            if chrome.scale > 1.0 {
                set_window_font_scale(chrome.scale);
            }
            let color = winner_color(cell.winner);
            for tag in &cell.tags {
                paint_centered(tag, color);
            }
            paint_centered(&cell.name, color);
        }
        igEnd();
    }
}

unsafe fn paint_y_debug(chrome: &Chrome) {
    let samples = scoreboard::height_debug();
    if samples.is_empty() {
        return;
    }
    let margin = EDGE_MARGIN * chrome.scale;
    igSetNextWindowPos(
        ImVec2 {
            x: margin,
            y: margin,
        },
        IMGUI_COND_ALWAYS,
        ImVec2 { x: 0.0, y: 0.0 },
    );
    igSetNextWindowBgAlpha(WINDOW_BG_ALPHA);
    if !igBegin(
        b"##tm_y_debug\0".as_ptr() as _,
        std::ptr::null_mut(),
        SCORE_FLAGS,
    ) {
        igEnd();
        return;
    }
    if chrome.scale > 1.0 {
        set_window_font_scale(chrome.scale);
    }
    let header = std::ffi::CString::new("Y debug (top hide off)").unwrap_or_default();
    paint(
        &header,
        ImVec4 {
            x: 1.0,
            y: 1.0,
            z: 1.0,
            w: 1.0,
        },
        false,
        0.0,
    );
    for sample in samples {
        let line = format!(
            "E{} y={:.1} team={}",
            sample.entry, sample.y, sample.team
        );
        if let Ok(text) = std::ffi::CString::new(line) {
            paint(
                &text,
                ImVec4 {
                    x: 0.85,
                    y: 0.9,
                    z: 0.7,
                    w: 1.0,
                },
                false,
                0.0,
            );
        }
    }
    igEnd();
}

unsafe extern "C" fn draw() {
    scoreboard::poll_missing_tags();
    let chrome = chrome();
    let (center_x, top_y, width, height) = window_anchor(chrome.scale);
    if !DRAW_LOGGED.swap(true, Ordering::Relaxed) {
        logutil::write(&format!(
            "score_overlay: draw {:.0}x{:.0} scale={:.0} visible={}",
            width,
            height,
            chrome.scale,
            scoreboard::overlay_visible()
        ));
    }
    paint_y_debug(&chrome);
    if let Some(parts) = scoreboard::draw_parts() {
        let scaled = chrome.scale > 1.0;
        if scaled {
            igPushStyleVar_Vec2(
                STYLE_WINDOW_PADDING,
                ImVec2 {
                    x: chrome.pad.x * chrome.scale,
                    y: chrome.pad.y * chrome.scale,
                },
            );
            igPushStyleVar_Vec2(
                STYLE_ITEM_SPACING,
                ImVec2 {
                    x: chrome.spacing.x * chrome.scale,
                    y: chrome.spacing.y * chrome.scale,
                },
            );
        }
        igSetNextWindowPos(
            ImVec2 {
                x: center_x,
                y: top_y,
            },
            IMGUI_COND_ALWAYS,
            ImVec2 { x: 0.5, y: 0.0 },
        );
        igSetNextWindowBgAlpha(WINDOW_BG_ALPHA);
        let mut score_bottom = None;
        if igBegin(
            b"##tm_score\0".as_ptr() as _,
            std::ptr::null_mut(),
            SCORE_FLAGS,
        ) {
            if scaled {
                set_window_font_scale(chrome.scale);
            }
            paint_scoreboard(&parts, &chrome);
            let pos = igGetWindowPos();
            // Auto-resize: approximate bottom from content cursor.
            let cursor = igGetCursorPos();
            score_bottom = Some(pos.y + cursor.y + chrome.pad.y * chrome.scale);
        }
        igEnd();
        if let Some(score_bottom) = score_bottom {
            if !parts.strip.is_empty() {
                paint_stage_row(&parts, center_x, score_bottom, &chrome);
            }
        }
        if scaled {
            igPopStyleVar(2);
        }
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
        logutil::write("score_overlay: missing libimgui_smash.nro — overlay host not loaded");
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
