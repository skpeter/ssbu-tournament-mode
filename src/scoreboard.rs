//! In-match set scoreboard.
//!
//! Score lives in `set_state`. This module owns `Place`, the tag cache,
//! and the world-Y sample slots. Drawing is the imgui-smash overlay in
//! `score_overlay` — not a battle HUD pane and not a skyline-web toast.
//!
//! `game_end` sets `Place::Results` and leaves the line up. It does not hide.
//! Sample path: fighter Main writes one slot per entry with the current frame
//! stamp. Death, a failed sample, or leaving melee clears that slot. The draw
//! hides only when a slot stamped this frame is under `WORLD_Y_BAND`.

use crate::board_visibility::{
    band_this_frame, board_visible, line_parts, place_for_scene, score_marks, should_sample,
    stage_name, strip_cells, strip_visible, wins_after_melee_enter, BoardView, HeightSample, Place,
    ScoreMarks, StripCell, WORLD_Y_BAND,
};
use crate::config;
use crate::logutil;
use crate::ruleset;
use crate::set_state::{self, SetPhase};
use once_cell::sync::OnceCell;
use skyline::hooks::{getRegionAddress, Region};
use skyline_smash::app::lua_bind::{PostureModule, StatusModule, WorkModule};
use skyline_smash::lib::lua_const::{
    FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID, FIGHTER_STATUS_KIND_DEAD,
};
use smashline::{Agent, L2CFighterCommon, Main};
use std::ffi::CString;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Mutex;

const TAG_CHARS: usize = 10;
const PLUGINS_DIR: &str = "sd:/atmosphere/contents/01006A800016E000/romfs/skyline/plugins";

struct DrawCache {
    tag0: Option<String>,
    tag1: Option<String>,
    tag0_c: Option<CString>,
    tag1_c: Option<CString>,
}

struct BoardRuntime {
    place: Place,
    frame: u64,
    slots: [Option<HeightSample>; 2],
    cache: DrawCache,
}

pub struct DrawParts {
    pub tag0: Option<CString>,
    pub tag1: Option<CString>,
    pub left: ScoreMarks,
    pub right: ScoreMarks,
    pub strip: Vec<StripPart>,
}

pub struct StripPart {
    pub tag: Option<CString>,
    pub name: CString,
    pub winner: u8,
}

static BOARD: OnceCell<Mutex<BoardRuntime>> = OnceCell::new();
static TAG_OFFSET_LOGGED: AtomicBool = AtomicBool::new(false);
/// Draw-callback retries while a tag is still empty. Reset on each place change.
static TAG_POLLS: AtomicU32 = AtomicU32::new(0);
const TAG_POLL_LIMIT: u32 = 1200;
const TAG_POLL_STRIDE: u32 = 30;

fn lock() -> std::sync::MutexGuard<'static, BoardRuntime> {
    BOARD
        .get_or_init(|| Mutex::new(fresh_runtime()))
        .lock()
        .unwrap()
}

fn fresh_runtime() -> BoardRuntime {
    let cache = cache_from(None, None);
    BoardRuntime {
        place: Place::Menu,
        frame: 1,
        slots: [None, None],
        cache,
    }
}

fn cache_from(tag0: Option<String>, tag1: Option<String>) -> DrawCache {
    let parts = line_parts(tag0.as_deref(), tag1.as_deref(), 0, 0, 1);
    DrawCache {
        tag0: parts.tag0.clone(),
        tag1: parts.tag1.clone(),
        tag0_c: parts.tag0.and_then(|text| CString::new(text).ok()),
        tag1_c: parts.tag1.and_then(|text| CString::new(text).ok()),
    }
}

pub fn install() {
    if config::get().scoreboard {
        logutil::write(&format!(
            "scoreboard: imgui overlay (libimgui_smash.nro); place enum; no pane bind; display {}",
            skyline::nn::oe::get_display_version()
        ));
        install_height_sample();
    } else {
        logutil::write("scoreboard: disabled by config");
    }
}

/// Character-select Start ran. Scene transitions are not logging on this
/// game, so this is the signal that the set screens have begun.
pub fn on_css_advance() {
    {
        let mut board = lock();
        if board.place == Place::Menu {
            board.place = Place::Css;
        }
    }
    arm_tag_poll();
    reread_tags();
    if overlay_visible() {
        logutil::write(&format!("scoreboard: showing '{}'", hud_text()));
    }
}

pub fn warn_if_both_smashline_nros() {
    let plugin = format!("{PLUGINS_DIR}/libsmashline_plugin.nro");
    let hook = format!("{PLUGINS_DIR}/libsmashline_hook.nro");
    if std::path::Path::new(&plugin).exists() && std::path::Path::new(&hook).exists() {
        logutil::write(
            "smashline: both libsmashline_plugin.nro and libsmashline_hook.nro are on the card; that pair crashes after character select. Keep one.",
        );
    }
}

fn install_height_sample() {
    Agent::new("fighter")
        .on_line(Main, sample_fighter_main)
        .install();
    logutil::write("scoreboard: smashline fighter Main world-Y sample installed");
}

pub fn on_scene(scene: &str) {
    let Some(place) = place_for_scene(scene) else {
        return;
    };
    let left_melee = {
        let mut board = lock();
        let left_melee = board.place == Place::Melee && place != Place::Melee;
        board.place = place;
        if place != Place::Melee {
            board.slots = [None, None];
        }
        left_melee
    };
    if left_melee {
        crate::hud_layout::on_melee_leave();
    }
    arm_tag_poll();
    if place == Place::Css || place == Place::Sss {
        reread_tags();
    }
}

pub fn on_game_end() {
    let left_melee = {
        let mut board = lock();
        let left_melee = board.place == Place::Melee;
        board.place = Place::Results;
        board.slots = [None, None];
        left_melee
    };
    if left_melee {
        crate::hud_layout::on_melee_leave();
    }
    arm_tag_poll();
    reread_tags();
}

pub fn on_melee_scene() {
    let snap = set_state::snapshot();
    let wins = wins_after_melee_enter(snap.phase == SetPhase::Complete, snap.wins);
    if wins != snap.wins {
        set_state::begin_next_set();
    }
    {
        let mut board = lock();
        board.place = Place::Melee;
        board.slots = [None, None];
    }
    arm_tag_poll();
    reread_tags();
    crate::hud_layout::on_melee_enter();
    if config::get().scoreboard && overlay_visible() {
        logutil::write(&format!("scoreboard: melee place text='{}'", hud_text()));
    }
}

pub fn advance_frame() {
    let mut board = lock();
    board.frame = board.frame.wrapping_add(1);
}

pub fn overlay_visible() -> bool {
    let snap = set_state::snapshot();
    let board = lock();
    board_visible(BoardView {
        enabled: config::get().scoreboard,
        set_length: ruleset::set_length(),
        probe_ok: snap.probe_ok,
        from_ruleset_name: ruleset::set_length_from_name(),
        set_complete: snap.phase == SetPhase::Complete,
        place: board.place,
        in_band: band_this_frame(board.slots, board.frame, WORLD_Y_BAND),
    })
}

pub fn draw_parts() -> Option<DrawParts> {
    if !overlay_visible() {
        return None;
    }
    let snap = set_state::snapshot();
    let first_to = snap.first_to.max(1);
    let board = lock();
    Some(DrawParts {
        tag0: board.cache.tag0_c.clone(),
        tag1: board.cache.tag1_c.clone(),
        left: score_marks(snap.wins[0], first_to),
        right: score_marks(snap.wins[1], first_to),
        strip: strip_parts(&snap, &board),
    })
}

fn strip_parts(snap: &set_state::SetState, board: &BoardRuntime) -> Vec<StripPart> {
    if !strip_visible(true, board.place) {
        return Vec::new();
    }
    let games: Vec<(i32, u8)> = snap
        .played
        .iter()
        .map(|game| (game.stage_id, game.winner))
        .collect();
    strip_cells(
        &games,
        board.cache.tag0.as_deref(),
        board.cache.tag1.as_deref(),
    )
    .into_iter()
    .map(strip_part)
    .collect()
}

fn strip_part(cell: StripCell) -> StripPart {
    StripPart {
        tag: cell.tag.and_then(|text| CString::new(text).ok()),
        name: stage_label(cell.stage_id),
        winner: cell.winner,
    }
}

fn stage_label(stage_id: i32) -> CString {
    if let Some(name) = stage_name(stage_id) {
        return CString::new(name).unwrap_or_else(|_| CString::new("stage").unwrap());
    }
    unknown_stage_label(stage_id)
}

fn unknown_stage_label(stage_id: i32) -> CString {
    static SEEN: OnceCell<Mutex<Vec<(i32, CString)>>> = OnceCell::new();
    let mut seen = SEEN.get_or_init(|| Mutex::new(Vec::new())).lock().unwrap();
    if let Some((_, label)) = seen.iter().find(|(id, _)| *id == stage_id) {
        return label.clone();
    }
    logutil::write(&format!("stage name: id {stage_id} has no mapping"));
    let label =
        CString::new(stage_id.to_string()).unwrap_or_else(|_| CString::new("stage").unwrap());
    seen.push((stage_id, label.clone()));
    label
}

/// The match tag buffer is empty on the first character select and filled
/// once the game copies names in. Keep trying for a while on each screen.
pub fn poll_missing_tags() {
    let (missing, place) = {
        let board = lock();
        let missing = board.cache.tag0.is_none() || board.cache.tag1.is_none();
        (missing, board.place)
    };
    if !missing {
        return;
    }
    if !matches!(
        place,
        Place::Css | Place::Sss | Place::Melee | Place::Results
    ) {
        return;
    }
    let n = TAG_POLLS.fetch_add(1, Ordering::Relaxed);
    if n >= TAG_POLL_LIMIT {
        return;
    }
    if n + 1 == TAG_POLL_LIMIT {
        logutil::write(&format!("scoreboard: tag retry stopped in {place:?}"));
    }
    if n % TAG_POLL_STRIDE != 0 {
        return;
    }
    reread_tags();
}

pub fn hud_text() -> String {
    let s = set_state::snapshot();
    format!("{} — {}", s.wins[0], s.wins[1])
}

fn arm_tag_poll() {
    TAG_POLLS.store(0, Ordering::Relaxed);
}

fn reread_tags() {
    let next0 = read_tag(0);
    let next1 = read_tag(1);
    let mut board = lock();
    if board.cache.tag0 == next0 && board.cache.tag1 == next1 {
        return;
    }
    board.cache = cache_from(next0.clone(), next1.clone());
    logutil::write(&format!(
        "scoreboard: tags {} / {}",
        next0.as_deref().unwrap_or("-"),
        next1.as_deref().unwrap_or("-"),
    ));
}

fn tag_base() -> usize {
    static BASE: OnceCell<usize> = OnceCell::new();
    *BASE.get_or_init(|| {
        let version = skyline::nn::oe::get_display_version();
        let base = if version_is_13_0_5_or_newer(&version) {
            0x52C5758
        } else {
            0x52C4758
        };
        logutil::write(&format!(
            "scoreboard: tag base {:#x} for display version {}",
            base, version
        ));
        base
    })
}

pub(crate) fn version_is_13_0_5_or_newer(version: &str) -> bool {
    let mut parts = version.split('.');
    let major = parts.next().and_then(|p| p.parse::<u32>().ok());
    let minor = parts.next().and_then(|p| p.parse::<u32>().ok());
    let patch = parts.next().and_then(|p| p.parse::<u32>().ok());
    match (major, minor, patch) {
        (Some(major), Some(minor), Some(patch)) => (major, minor, patch) >= (13, 0, 5),
        _ => false,
    }
}

fn read_tag(player_index: usize) -> Option<String> {
    if player_index > 7 {
        return None;
    }
    let offset = tag_base() + player_index * 0x260;
    let addr = unsafe { (getRegionAddress(Region::Text) as *const u8).add(offset) as *const u16 };
    if addr.is_null() {
        log_bad_tag_once();
        return None;
    }
    let mut units = [0u16; TAG_CHARS];
    let mut len = 0;
    unsafe {
        for slot in &mut units {
            let unit = *addr.add(len);
            if unit == 0 {
                break;
            }
            if unit < 0x20 {
                log_bad_tag_once();
                return None;
            }
            *slot = unit;
            len += 1;
        }
        if len == TAG_CHARS && *addr.add(len) != 0 {
            log_bad_tag_once();
            return None;
        }
    }
    if len == 0 {
        return None;
    }
    let text = String::from_utf16_lossy(&units[..len]);
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

fn log_bad_tag_once() {
    if !TAG_OFFSET_LOGGED.swap(true, Ordering::SeqCst) {
        logutil::write("scoreboard: tag read failed; drawing the score without tags");
    }
}

fn clear_slot(entry: i32) {
    if entry != 0 && entry != 1 {
        return;
    }
    let mut board = lock();
    board.slots[entry as usize] = None;
}

fn store_slot(entry: usize, y: f32) {
    let mut board = lock();
    let frame = board.frame;
    board.slots[entry] = Some(HeightSample { y, frame });
}

unsafe extern "C" fn sample_fighter_main(fighter: &mut L2CFighterCommon) {
    let place = lock().place;
    if place != Place::Melee {
        return;
    }
    let module_accessor = fighter.module_accessor;
    if module_accessor.is_null() {
        return;
    }
    let entry = WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID);
    let dead = StatusModule::status_kind(module_accessor) == *FIGHTER_STATUS_KIND_DEAD;
    if !should_sample(place, dead, entry) {
        clear_slot(entry);
        return;
    }
    let y = PostureModule::pos_y(module_accessor);
    store_slot(entry as usize, y);
}
