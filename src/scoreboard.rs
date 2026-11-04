//! In-match set scoreboard.
//!
//! Score lives in `set_state`. This module owns `Place`, the tag cache,
//! team grouping, and the world-Y sample slots. Drawing is the imgui-smash
//! overlay in `score_overlay` — not a battle HUD pane and not a skyline-web
//! toast.
//!
//! `game_end` sets `Place::Results` and leaves the line up. It does not hide.
//! Sample path: fighter Main writes one slot per entry with the current frame
//! stamp, plus `TeamModule::team_no`. Top proximity hide stays off until
//! `WORLD_Y_TOP_BAND` is set from the temporary Y debug overlay.

use crate::board_visibility::{
    band_this_frame, board_visible, doubles_roster, ordered_team_pair, place_for_scene,
    remember_stage, score_marks, should_sample, skip_multi_roster, stage_name, strip_cells,
    strip_visible, wins_after_melee_enter, BoardView, HeightSample, Place, ScoreMarks, StripCell,
    MAX_ENTRIES, WORLD_Y_TOP_BAND,
};
use crate::config;
use crate::logutil;
use crate::ruleset;
use crate::set_state::{self, SetPhase};
use once_cell::sync::OnceCell;
use skyline::hooks::{getRegionAddress, Region};
use skyline_smash::app::lua_bind::{PostureModule, StatusModule, TeamModule, WorkModule};
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
    left_tags: Vec<String>,
    right_tags: Vec<String>,
    left_c: Vec<CString>,
    right_c: Vec<CString>,
}

#[derive(Clone, Copy, Debug)]
pub struct HeightDebug {
    pub entry: u8,
    pub y: f32,
    pub team: i32,
}

struct BoardRuntime {
    place: Place,
    frame: u64,
    slots: [Option<HeightSample>; MAX_ENTRIES],
    /// `TeamModule::team_no` per entry. `-1` means unknown / FFA.
    teams: [i32; MAX_ENTRIES],
    /// Entry has been sampled at least once this set (cleared on Menu).
    seen: [bool; MAX_ENTRIES],
    cache: DrawCache,
    /// Stage seen while the match was loaded. Random's selection id stays invalid.
    live_stage: Option<i32>,
}

pub struct DrawParts {
    pub left_tags: Vec<CString>,
    pub right_tags: Vec<CString>,
    pub left: ScoreMarks,
    pub right: ScoreMarks,
    pub strip: Vec<StripPart>,
}

pub struct StripPart {
    pub tags: Vec<CString>,
    pub name: CString,
    pub winner: u8,
}

/// Resolved scoreboard sides: entry indices on left (side 0) and right (side 1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SideRoster {
    pub left: Vec<usize>,
    pub right: Vec<usize>,
    pub doubles: bool,
}

static BOARD: OnceCell<Mutex<BoardRuntime>> = OnceCell::new();
static TAG_OFFSET_LOGGED: AtomicBool = AtomicBool::new(false);
static TEAM_LOGGED: AtomicBool = AtomicBool::new(false);
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
    BoardRuntime {
        place: Place::Menu,
        frame: 1,
        slots: [None; MAX_ENTRIES],
        teams: [-1; MAX_ENTRIES],
        seen: [false; MAX_ENTRIES],
        cache: cache_from(Vec::new(), Vec::new()),
        live_stage: None,
    }
}

fn cache_from(left_tags: Vec<String>, right_tags: Vec<String>) -> DrawCache {
    let left_c = left_tags
        .iter()
        .filter_map(|text| CString::new(text.as_str()).ok())
        .collect();
    let right_c = right_tags
        .iter()
        .filter_map(|text| CString::new(text.as_str()).ok())
        .collect();
    DrawCache {
        left_tags,
        right_tags,
        left_c,
        right_c,
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
    logutil::write("scoreboard: smashline fighter Main world-Y + team sample installed");
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
            board.slots = [None; MAX_ENTRIES];
        }
        if place == Place::Menu {
            board.teams = [-1; MAX_ENTRIES];
            board.seen = [false; MAX_ENTRIES];
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
        board.slots = [None; MAX_ENTRIES];
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
        board.slots = [None; MAX_ENTRIES];
        board.live_stage = None;
    }
    arm_tag_poll();
    reread_tags();
    crate::hud_layout::on_melee_enter();
    if config::get().scoreboard && overlay_visible() {
        logutil::write(&format!("scoreboard: melee place text='{}'", hud_text()));
    }
}

/// Stage id sampled during the match. `None` until a playable stage loads.
pub fn cached_stage() -> Option<i32> {
    lock().live_stage
}

fn observe_loaded_stage() {
    let loaded = unsafe { skyline_smash::app::stage::get_stage_id() };
    let selected = unsafe { skyline_smash::app::sv_information::stage_id() };
    let mut board = lock();
    if board.place != Place::Melee {
        return;
    }
    let next = remember_stage(remember_stage(board.live_stage, loaded), selected);
    if next != board.live_stage {
        board.live_stage = next;
        if let Some(id) = next {
            logutil::write(&format!(
                "scoreboard: loaded stage {id} (selected {selected})"
            ));
        }
    }
}

pub fn advance_frame() {
    let mut board = lock();
    board.frame = board.frame.wrapping_add(1);
}

pub fn overlay_visible() -> bool {
    let snap = set_state::snapshot();
    let board = lock();
    if roster_skips_board(&board.teams, &board.seen) {
        return false;
    }
    let in_band = match WORLD_Y_TOP_BAND {
        Some(constant) => band_this_frame(&board.slots, board.frame, constant),
        None => false,
    };
    board_visible(BoardView {
        enabled: config::get().scoreboard,
        set_length: ruleset::set_length(),
        probe_ok: snap.probe_ok,
        from_ruleset_name: ruleset::set_length_from_name(),
        set_complete: snap.phase == SetPhase::Complete,
        place: board.place,
        in_band,
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
        left_tags: board.cache.left_c.clone(),
        right_tags: board.cache.right_c.clone(),
        left: score_marks(snap.wins[0], first_to),
        right: score_marks(snap.wins[1], first_to),
        strip: strip_parts(&snap, &board),
    })
}

/// Live world-Y samples for the temporary top-hide tuning overlay.
pub fn height_debug() -> Vec<HeightDebug> {
    let board = lock();
    if board.place != Place::Melee {
        return Vec::new();
    }
    let mut out = Vec::new();
    for (entry, slot) in board.slots.iter().enumerate() {
        if let Some(sample) = slot {
            if sample.frame == board.frame {
                out.push(HeightDebug {
                    entry: entry as u8,
                    y: sample.y,
                    team: board.teams[entry],
                });
            }
        }
    }
    out
}

/// Entry indices grouped into scoreboard sides. Singles = `[0]` vs `[1]`.
pub fn side_roster() -> SideRoster {
    let board = lock();
    resolve_sides(&board.teams, &board.seen)
}

fn roster_skips_board(teams: &[i32; MAX_ENTRIES], seen: &[bool; MAX_ENTRIES]) -> bool {
    let mut seen_n = 0u32;
    let mut distinct = Vec::new();
    let mut any_ffa = false;
    for (entry, &is_seen) in seen.iter().enumerate() {
        if !is_seen {
            continue;
        }
        seen_n += 1;
        let team = teams[entry];
        if team < 0 {
            any_ffa = true;
            continue;
        }
        if !distinct.contains(&team) {
            distinct.push(team);
        }
    }
    skip_multi_roster(seen_n, distinct.len() as u32, any_ffa)
}

fn resolve_sides(teams: &[i32; MAX_ENTRIES], seen: &[bool; MAX_ENTRIES]) -> SideRoster {
    let mut by_team: Vec<(i32, Vec<usize>)> = Vec::new();
    for (entry, &team) in teams.iter().enumerate() {
        if !seen[entry] || team < 0 {
            continue;
        }
        if let Some((_, members)) = by_team.iter_mut().find(|(t, _)| *t == team) {
            members.push(entry);
        } else {
            by_team.push((team, vec![entry]));
        }
    }
    let entry_count = by_team.iter().map(|(_, m)| m.len() as u32).sum::<u32>();
    let distinct = by_team.len() as u32;
    if doubles_roster(entry_count, distinct) && by_team.len() == 2 {
        let (a, b) = ordered_team_pair(by_team[0].0, by_team[1].0);
        let left = by_team
            .iter()
            .find(|(t, _)| *t == a)
            .map(|(_, m)| m.clone())
            .unwrap_or_default();
        let right = by_team
            .iter()
            .find(|(t, _)| *t == b)
            .map(|(_, m)| m.clone())
            .unwrap_or_default();
        return SideRoster {
            left,
            right,
            doubles: true,
        };
    }
    SideRoster {
        left: vec![0],
        right: vec![1],
        doubles: false,
    }
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
    strip_cells(&games, &board.cache.left_tags, &board.cache.right_tags)
        .into_iter()
        .map(strip_part)
        .collect()
}

fn strip_part(cell: StripCell) -> StripPart {
    StripPart {
        tags: cell
            .tags
            .into_iter()
            .filter_map(|text| CString::new(text).ok())
            .collect(),
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
        let missing = board.cache.left_tags.is_empty() || board.cache.right_tags.is_empty();
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
    let roster = {
        let board = lock();
        resolve_sides(&board.teams, &board.seen)
    };
    let left = read_side_tags(&roster.left);
    let right = read_side_tags(&roster.right);
    let mut board = lock();
    if board.cache.left_tags == left && board.cache.right_tags == right {
        return;
    }
    board.cache = cache_from(left.clone(), right.clone());
    logutil::write(&format!(
        "scoreboard: tags [{}] / [{}] doubles={}",
        left.join("+"),
        right.join("+"),
        roster.doubles
    ));
}

fn read_side_tags(entries: &[usize]) -> Vec<String> {
    entries.iter().filter_map(|&entry| read_tag(entry)).collect()
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
    if entry < 0 || (entry as usize) >= MAX_ENTRIES {
        return;
    }
    let mut board = lock();
    board.slots[entry as usize] = None;
}

fn store_slot(entry: usize, y: f32, team: i32) {
    let mut board = lock();
    let frame = board.frame;
    board.slots[entry] = Some(HeightSample { y, frame });
    board.seen[entry] = true;
    let prev = board.teams[entry];
    board.teams[entry] = team;
    if prev != team && !TEAM_LOGGED.swap(true, Ordering::Relaxed) {
        logutil::write(&format!(
            "scoreboard: team sample entry={entry} team={team}"
        ));
    } else if prev != team {
        logutil::write(&format!(
            "scoreboard: team change entry={entry} {prev}->{team}"
        ));
    }
}

unsafe extern "C" fn sample_fighter_main(fighter: &mut L2CFighterCommon) {
    let place = lock().place;
    if place != Place::Melee {
        return;
    }
    observe_loaded_stage();
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
    let team = TeamModule::team_no(module_accessor) as i32;
    let entry_u = entry as usize;
    store_slot(entry_u, y, team);
    // Refresh tags once both teams are known so doubles stacks appear mid-match.
    let should_refresh = {
        let board = lock();
        let roster = resolve_sides(&board.teams, &board.seen);
        roster.doubles
            && (board.cache.left_tags.len() < roster.left.len()
                || board.cache.right_tags.len() < roster.right.len())
    };
    if should_refresh {
        reread_tags();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn singles_when_teams_unknown() {
        let sides = resolve_sides(&[-1; MAX_ENTRIES], &[false; MAX_ENTRIES]);
        assert!(!sides.doubles);
        assert_eq!(sides.left, vec![0]);
        assert_eq!(sides.right, vec![1]);
    }

    #[test]
    fn doubles_groups_by_team_color() {
        let mut teams = [-1; MAX_ENTRIES];
        let mut seen = [false; MAX_ENTRIES];
        teams[0] = 0;
        teams[1] = 1;
        teams[2] = 0;
        teams[3] = 1;
        seen[0] = true;
        seen[1] = true;
        seen[2] = true;
        seen[3] = true;
        let sides = resolve_sides(&teams, &seen);
        assert!(sides.doubles);
        assert_eq!(sides.left, vec![0, 2]);
        assert_eq!(sides.right, vec![1, 3]);
        assert!(!roster_skips_board(&teams, &seen));
    }

    #[test]
    fn three_teams_or_ffa_skips_board() {
        let mut teams = [-1; MAX_ENTRIES];
        let mut seen = [false; MAX_ENTRIES];
        teams[0] = 0;
        teams[1] = 1;
        teams[2] = 2;
        teams[3] = 0;
        seen[0] = true;
        seen[1] = true;
        seen[2] = true;
        seen[3] = true;
        assert!(!resolve_sides(&teams, &seen).doubles);
        assert!(roster_skips_board(&teams, &seen));

        let ffa = [-1; MAX_ENTRIES];
        let mut ffa_seen = [false; MAX_ENTRIES];
        ffa_seen[0] = true;
        ffa_seen[1] = true;
        ffa_seen[2] = true;
        ffa_seen[3] = true;
        assert!(roster_skips_board(&ffa, &ffa_seen));
    }
}
