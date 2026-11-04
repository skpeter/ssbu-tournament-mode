//! Show/hide for the in-match scoreboard. No skyline imports.
//!
//! ```text
//! enabled, set_length > 1, probe_ok, ruleset name contains FT?
//!         | no -> HIDE
//!         v yes
//! set_complete AND place != Results?
//!         | yes -> HIDE until the next melee enter resets to 0-0
//!         v no
//! place == Melee AND a this-frame sample is in the bottom band?
//!         | yes -> HIDE
//!         v no
//! place is Css, Sss, Melee, or Results?
//!         | yes -> SHOW
//!         v no (Menu)
//!         HIDE
//! ```
//!
//! Sample path: `should_sample` writes one slot per entry with a frame stamp.
//! Death, a failed `should_sample`, or leaving `Melee` clears that slot.
//! `band_this_frame` hides only from a slot stamped this frame whose world Y
//! is under `WORLD_Y_BAND`. Tag strings rebuild when a read changes. Each
//! side's blocks are `score_marks`: one filled cell per win, one open cell
//! per win still needed, and `first_to` cells in total.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    Menu,
    Css,
    Sss,
    Melee,
    Results,
}

/// World Y under this value hides the line during melee. Equal does not hide.
/// Standing on a floor near 0 stays visible. Tune on hardware.
pub const WORLD_Y_BAND: f32 = 0.0;

#[derive(Clone, Copy, Debug)]
pub struct BoardView {
    pub enabled: bool,
    pub set_length: u32,
    pub probe_ok: bool,
    /// True only when the selected Versus preset name parsed as `FT2` / `FT3` / …
    pub from_ruleset_name: bool,
    pub set_complete: bool,
    pub place: Place,
    pub in_band: bool,
}

/// Filled cells are games won. Open cells are games still needed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScoreMarks {
    pub filled: u32,
    pub open: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LineParts {
    pub tag0: Option<String>,
    pub tag1: Option<String>,
    pub left: ScoreMarks,
    pub right: ScoreMarks,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeightSample {
    pub y: f32,
    pub frame: u64,
}

pub fn board_visible(view: BoardView) -> bool {
    if !view.enabled || view.set_length <= 1 || !view.probe_ok || !view.from_ruleset_name {
        return false;
    }
    if view.set_complete && view.place != Place::Results {
        return false;
    }
    if view.place == Place::Melee && view.in_band {
        return false;
    }
    matches!(
        view.place,
        Place::Css | Place::Sss | Place::Melee | Place::Results
    )
}

/// Doubles / FFA / 3+ fighter lobbies: no scoreboard.
pub fn too_many_players(entry_count: u32) -> bool {
    entry_count > 3
}

/// `None` means `MeleeScene`, which the melee-enter path owns.
pub fn place_for_scene(scene: &str) -> Option<Place> {
    match scene {
        "MeleeScene" => None,
        "CharaSelectScene" => Some(Place::Css),
        "StageSelectScene" => Some(Place::Sss),
        _ => Some(Place::Menu),
    }
}

/// `first_to` is how many wins end the set. Wins above that still fill every cell.
pub fn score_marks(wins: u32, first_to: u32) -> ScoreMarks {
    let slots = first_to.max(1);
    let filled = wins.min(slots);
    ScoreMarks {
        filled,
        open: slots - filled,
    }
}

pub fn line_parts(
    tag0: Option<&str>,
    tag1: Option<&str>,
    w0: u32,
    w1: u32,
    first_to: u32,
) -> LineParts {
    LineParts {
        tag0: present(tag0),
        tag1: present(tag1),
        left: score_marks(w0, first_to),
        right: score_marks(w1, first_to),
    }
}

fn present(tag: Option<&str>) -> Option<String> {
    match tag {
        Some(text) if !text.is_empty() => Some(text.to_string()),
        _ => None,
    }
}

pub fn should_sample(place: Place, dead: bool, entry_id: i32) -> bool {
    place == Place::Melee && !dead && (entry_id == 0 || entry_id == 1)
}

pub fn in_band(y: f32, constant: f32) -> bool {
    y < constant
}

pub fn band_this_frame(slots: [Option<HeightSample>; 2], frame: u64, constant: f32) -> bool {
    for slot in slots {
        if let Some(sample) = slot {
            if sample.frame == frame && in_band(sample.y, constant) {
                return true;
            }
        }
    }
    false
}

/// Wins to keep when `MeleeScene` starts. A finished set goes back to 0-0.
pub fn wins_after_melee_enter(set_complete: bool, wins: [u32; 2]) -> [u32; 2] {
    if set_complete {
        [0, 0]
    } else {
        wins
    }
}

/// The stage strip is on stage select, and only while the one-line board is up.
pub fn strip_visible(board_up: bool, place: Place) -> bool {
    board_up && place == Place::Sss
}

/// Names for `app::sv_information::stage_id`.
/// Each arm is the normal form, then its omega form, then its battlefield form.
/// Ids outside the list have no name.
pub fn stage_name(stage_id: i32) -> Option<&'static str> {
    Some(match stage_id {
        0x0 | 0x1 => "Battlefield",
        0x3 | 0x4 => "Final Destination",
        0xE | 0xF | 0x10 => "Yoshi's Story",
        0x5F | 0x60 | 0x61 => "Smashville",
        0x6B | 0x6C | 0x6D => "Pokemon Stadium 2",
        0xF2 | 0xF3 | 0xF4 => "Kalos Pokemon League",
        0x101 | 0x102 | 0x103 => "Town and City",
        0x15B => "Small Battlefield",
        0x15F | 0x160 | 0x161 => "Northern Cave",
        0x169 | 0x16A | 0x16B => "Hollow Bastion",
        _ => return None,
    })
}

/// A negative id is not a stage. Zero is Battlefield.
pub fn stage_for_strip(stage_id: i32) -> Option<i32> {
    if stage_id < 0 {
        None
    } else {
        Some(stage_id)
    }
}

/// Training, results, and other non-match ids. Random stays invalid (`-1`)
/// until the loaded stage replaces it.
fn is_system_stage(stage_id: i32) -> bool {
    matches!(stage_id, 0x134..=0x147 | 0x14B | 0x162)
}

/// A stage id worth keeping for the strip. Negative and system ids are not.
pub fn playable_stage(stage_id: i32) -> Option<i32> {
    let stage_id = stage_for_strip(stage_id)?;
    if is_system_stage(stage_id) {
        None
    } else {
        Some(stage_id)
    }
}

/// Keep the first named stage. A later invalid or results id does not replace it.
/// An unnamed id is replaced once a named stage shows up.
pub fn remember_stage(kept: Option<i32>, observed: i32) -> Option<i32> {
    let Some(observed) = playable_stage(observed) else {
        return kept;
    };
    match kept {
        Some(id) if stage_name(id).is_some() => Some(id),
        _ => Some(observed),
    }
}

/// The stage that was played. `loaded` is the stage resource. `selected` is
/// `sv_information::stage_id`, which stays invalid when Random was picked.
/// `cached` is the stage seen while the match was running.
fn named_stage(stage_id: i32) -> Option<i32> {
    if stage_name(stage_id).is_some() {
        Some(stage_id)
    } else {
        None
    }
}

pub fn pick_played_stage(selected: i32, loaded: i32, cached: Option<i32>) -> Option<i32> {
    named_stage(loaded)
        .or_else(|| named_stage(selected))
        .or_else(|| cached.and_then(named_stage))
        .or_else(|| playable_stage(loaded))
        .or_else(|| playable_stage(selected))
        .or_else(|| cached.and_then(playable_stage))
}

/// A win cell. `from_right` fills the left player's grid toward the center.
pub fn cell_filled(index: u32, filled: u32, total: u32, from_right: bool) -> bool {
    if total == 0 || index >= total {
        return false;
    }
    if from_right {
        index + filled >= total
    } else {
        index < filled
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SideColumn {
    pub name_x: f32,
    pub grid_x: f32,
    pub width: f32,
}

/// Name above the grid. `align_end` pins both to the inner edge (player 1).
pub fn side_column(name_w: f32, grid_w: f32, align_end: bool) -> SideColumn {
    let width = name_w.max(grid_w);
    if align_end {
        SideColumn {
            name_x: width - name_w,
            grid_x: width - grid_w,
            width,
        }
    } else {
        SideColumn {
            name_x: 0.0,
            grid_x: 0.0,
            width,
        }
    }
}

/// `nn::oe::GetOperationMode`: handheld and tabletop are 0, docked TV is 1.
/// 2x only on a docked framebuffer (about 1080p). A 720p image stays at 1x
/// even when an emulator reports console mode.
pub fn hud_font_scale(operation_mode: i32, display_width: f32) -> f32 {
    if operation_mode == 1 && display_width >= 1600.0 {
        2.0
    } else {
        1.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StageBoxPos {
    pub x: f32,
    pub y: f32,
}

/// One box per result, centered as a row, bottoms `gap` above the scoreboard.
pub fn stage_box_positions(
    sizes: &[(f32, f32)],
    score_top: f32,
    center_x: f32,
    gap: f32,
    box_gap: f32,
) -> Vec<StageBoxPos> {
    if sizes.is_empty() {
        return Vec::new();
    }
    let total_w = sizes.iter().map(|(w, _)| *w).sum::<f32>()
        + box_gap * (sizes.len().saturating_sub(1)) as f32;
    let mut x = center_x - total_w * 0.5;
    let mut out = Vec::with_capacity(sizes.len());
    for (w, h) in sizes {
        out.push(StageBoxPos {
            x,
            y: score_top - gap - h,
        });
        x += w + box_gap;
    }
    out
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StripCell {
    pub stage_id: i32,
    pub winner: u8,
    pub tag: Option<String>,
}

/// One cell per played game, in order. A missing tag is omitted. Repeats stay.
pub fn strip_cells(games: &[(i32, u8)], tag0: Option<&str>, tag1: Option<&str>) -> Vec<StripCell> {
    games
        .iter()
        .filter_map(|(stage_id, winner)| {
            let stage_id = stage_for_strip(*stage_id)?;
            let tag = match *winner {
                0 => present(tag0),
                1 => present(tag1),
                _ => None,
            };
            Some(StripCell {
                stage_id,
                winner: *winner,
                tag,
            })
        })
        .collect()
}

/// Smallest stage-select picture in the game archive.
/// The file is a BNTX. `igImage` needs a decoded texture, not these bytes.
#[cfg(test)]
pub fn thumb_arc_path(stage_key: &str) -> String {
    format!("ui/replace/stage/stage_0_{stage_key}.bntx")
}

/// Smash archive path hash: length in the high byte, CRC-32 in the low 32 bits.
#[cfg(test)]
pub fn hash40(text: &str) -> u64 {
    ((text.len() as u64) << 32) | u64::from(crc32(text.as_bytes()))
}

#[cfg(test)]
fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shown_set(place: Place, set_complete: bool, in_band: bool) -> BoardView {
        BoardView {
            enabled: true,
            set_length: 3,
            probe_ok: true,
            from_ruleset_name: true,
            set_complete,
            place,
            in_band,
        }
    }

    #[test]
    fn hides_when_more_than_three_players() {
        assert!(!too_many_players(0));
        assert!(!too_many_players(2));
        assert!(!too_many_players(3));
        assert!(too_many_players(4));
        assert!(too_many_players(8));
    }

    #[test]
    fn board_visible_gates() {
        assert!(!board_visible(BoardView {
            enabled: false,
            ..shown_set(Place::Css, false, false)
        }));
        assert!(!board_visible(BoardView {
            set_length: 1,
            ..shown_set(Place::Melee, false, false)
        }));
        assert!(!board_visible(BoardView {
            probe_ok: false,
            ..shown_set(Place::Css, false, false)
        }));
        assert!(!board_visible(BoardView {
            from_ruleset_name: false,
            ..shown_set(Place::Css, false, false)
        }));
        assert!(!board_visible(BoardView {
            from_ruleset_name: false,
            ..shown_set(Place::Melee, false, false)
        }));

        assert!(board_visible(shown_set(Place::Css, false, false)));
        assert!(board_visible(shown_set(Place::Sss, false, false)));
        assert!(board_visible(shown_set(Place::Melee, false, false)));
        assert!(!board_visible(shown_set(Place::Melee, false, true)));
        assert!(board_visible(shown_set(Place::Css, false, true)));
        assert!(board_visible(shown_set(Place::Results, false, false)));
        assert!(board_visible(shown_set(Place::Results, true, false)));
        assert!(!board_visible(shown_set(Place::Css, true, false)));
        assert!(!board_visible(shown_set(Place::Menu, false, false)));
        assert!(!board_visible(shown_set(Place::Menu, true, false)));
    }

    #[test]
    fn board_visible_melee_enter_reset() {
        assert_eq!(wins_after_melee_enter(true, [2, 1]), [0, 0]);
        assert!(board_visible(shown_set(Place::Melee, false, false)));

        assert_eq!(wins_after_melee_enter(false, [1, 0]), [1, 0]);
        assert_eq!(wins_after_melee_enter(true, [1, 0]), [0, 0]);
    }

    #[test]
    fn board_visible_scene_map() {
        assert_eq!(place_for_scene("CharaSelectScene"), Some(Place::Css));
        assert_eq!(place_for_scene("StageSelectScene"), Some(Place::Sss));
        assert_eq!(place_for_scene("MeleeRuleScene"), Some(Place::Menu));
        assert_eq!(place_for_scene("MainMenuScene"), Some(Place::Menu));
        assert_eq!(place_for_scene("LocalTopScene"), Some(Place::Menu));
        assert_eq!(place_for_scene("MenuSequenceScene"), Some(Place::Menu));
        assert_eq!(place_for_scene("MeleeScene"), None);
    }

    #[test]
    fn board_visible_line_parts() {
        let both = line_parts(Some("Red"), Some("Blue"), 1, 0, 3);
        assert_eq!(both.tag0.as_deref(), Some("Red"));
        assert_eq!(both.left, ScoreMarks { filled: 1, open: 2 });
        assert_eq!(both.right, ScoreMarks { filled: 0, open: 3 });
        assert_eq!(both.tag1.as_deref(), Some("Blue"));

        let no_tags = line_parts(None, Some(""), 0, 0, 2);
        assert!(no_tags.tag0.is_none());
        assert!(no_tags.tag1.is_none());
        assert_eq!(no_tags.left, score_marks(0, 2));

        let after_win = line_parts(Some("Red"), None, 1, 0, 3);
        let after_reset = line_parts(Some("Red"), None, 0, 0, 3);
        assert_eq!(after_win.left.filled, 1);
        assert_eq!(after_reset.left, ScoreMarks { filled: 0, open: 3 });
    }

    #[test]
    fn score_marks_follow_first_to() {
        assert_eq!(score_marks(0, 3), ScoreMarks { filled: 0, open: 3 });
        assert_eq!(score_marks(1, 2), ScoreMarks { filled: 1, open: 1 });
        assert_eq!(score_marks(3, 3), ScoreMarks { filled: 3, open: 0 });
        assert_eq!(score_marks(5, 3), ScoreMarks { filled: 3, open: 0 });
    }

    #[test]
    fn board_visible_should_sample() {
        assert!(should_sample(Place::Melee, false, 0));
        assert!(should_sample(Place::Melee, false, 1));
        assert!(!should_sample(Place::Melee, true, 0));
        assert!(!should_sample(Place::Melee, false, 2));
        assert!(!should_sample(Place::Results, false, 0));
        assert!(!should_sample(Place::Css, false, 1));
    }

    #[test]
    fn board_visible_in_band_and_slots() {
        assert!(!in_band(0.0, 0.0));
        assert!(in_band(-0.1, 0.0));
        assert!(!in_band(1.0, 0.0));

        let constant = 0.0;
        let frame = 4;
        assert!(!band_this_frame([None, None], frame, constant));
        let stale = HeightSample {
            y: -5.0,
            frame: frame - 1,
        };
        assert!(!band_this_frame([Some(stale), None], frame, constant));
        let low = HeightSample { y: -5.0, frame };
        let high = HeightSample { y: 10.0, frame };
        assert!(band_this_frame([Some(high), Some(low)], frame, constant));
        assert!(!band_this_frame([Some(high), None], frame, constant));
    }

    #[test]
    fn strip_is_stage_select_only() {
        assert!(strip_visible(true, Place::Sss));
        assert!(!strip_visible(true, Place::Css));
        assert!(!strip_visible(true, Place::Melee));
        assert!(!strip_visible(true, Place::Results));
        assert!(!strip_visible(true, Place::Menu));
        assert!(!strip_visible(false, Place::Sss));
    }

    #[test]
    fn strip_cells_keep_order_and_repeat_stages() {
        let cells = strip_cells(
            &[(4, 0), (4, 1), (-1, 0), (9, 2)],
            Some("Red"),
            Some("Blue"),
        );
        assert_eq!(
            cells,
            vec![
                StripCell {
                    stage_id: 4,
                    winner: 0,
                    tag: Some("Red".into()),
                },
                StripCell {
                    stage_id: 4,
                    winner: 1,
                    tag: Some("Blue".into()),
                },
                StripCell {
                    stage_id: 9,
                    winner: 2,
                    tag: None,
                },
            ]
        );

        let missing = strip_cells(&[(3, 0)], None, Some(""));
        assert_eq!(missing.len(), 1);
        assert!(missing[0].tag.is_none());
        assert!(stage_for_strip(-1).is_none());
        assert_eq!(stage_for_strip(0), Some(0));
    }

    #[test]
    fn random_stage_uses_the_loaded_or_cached_id() {
        assert_eq!(pick_played_stage(-1, 3, None), Some(3));
        assert_eq!(pick_played_stage(-1, 0x136, Some(4)), Some(4));
        assert_eq!(pick_played_stage(-1, -1, None), None);
        assert_eq!(pick_played_stage(0xE, 0xE, None), Some(0xE));
        assert_eq!(remember_stage(None, -1), None);
        assert_eq!(remember_stage(None, 0x136), None);
        assert_eq!(remember_stage(None, 3), Some(3));
        assert_eq!(remember_stage(Some(3), 0x136), Some(3));
        assert_eq!(remember_stage(Some(3), 0), Some(3));
        assert_eq!(remember_stage(Some(200), 4), Some(4));
    }

    #[test]
    fn left_grid_fills_toward_the_center() {
        assert!(!cell_filled(0, 2, 3, true));
        assert!(cell_filled(1, 2, 3, true));
        assert!(cell_filled(2, 2, 3, true));
        assert!(cell_filled(0, 1, 3, false));
        assert!(!cell_filled(1, 1, 3, false));
        assert!(!cell_filled(0, 0, 3, true));
        assert!(cell_filled(0, 3, 3, true));
    }

    fn names_sit_on_the_inner_edge() {
        assert_eq!(
            side_column(40.0, 30.0, true),
            SideColumn {
                name_x: 0.0,
                grid_x: 10.0,
                width: 40.0,
            }
        );
        assert_eq!(
            side_column(20.0, 30.0, true),
            SideColumn {
                name_x: 10.0,
                grid_x: 0.0,
                width: 30.0,
            }
        );
        assert_eq!(
            side_column(50.0, 30.0, false),
            SideColumn {
                name_x: 0.0,
                grid_x: 0.0,
                width: 50.0,
            }
        );
    }

    #[test]
    fn docked_text_is_double_handheld_stays() {
        assert_eq!(hud_font_scale(0, 1920.0), 1.0);
        assert_eq!(hud_font_scale(1, 1920.0), 2.0);
        assert_eq!(hud_font_scale(1, 1280.0), 1.0);
        assert_eq!(hud_font_scale(2, 1920.0), 1.0);
    }

    #[test]
    fn stage_boxes_sit_centered_above_the_scoreboard() {
        let row = stage_box_positions(&[(100.0, 40.0), (80.0, 40.0)], 1000.0, 500.0, 10.0, 20.0);
        assert_eq!(
            row,
            vec![
                StageBoxPos { x: 400.0, y: 950.0 },
                StageBoxPos { x: 520.0, y: 950.0 },
            ]
        );
        assert!(stage_box_positions(&[], 0.0, 0.0, 0.0, 0.0).is_empty());
    }

    #[test]
    fn stage_names_cover_the_manual_list() {
        assert_eq!(stage_name(0), Some("Battlefield"));
        assert_eq!(stage_name(1), Some("Battlefield"));
        assert_eq!(stage_name(3), Some("Final Destination"));
        assert_eq!(stage_name(4), Some("Final Destination"));
        assert_eq!(stage_name(0xE), Some("Yoshi's Story"));
        assert_eq!(stage_name(0x5F), Some("Smashville"));
        assert_eq!(stage_name(0x6B), Some("Pokemon Stadium 2"));
        assert_eq!(stage_name(0xF2), Some("Kalos Pokemon League"));
        assert_eq!(stage_name(0x101), Some("Town and City"));
        assert_eq!(stage_name(0x15B), Some("Small Battlefield"));
        assert_eq!(stage_name(0x15F), Some("Northern Cave"));
        assert_eq!(stage_name(0x169), Some("Hollow Bastion"));
        assert_eq!(stage_name(-1), None);
        assert_eq!(stage_name(215), None);
    }

    #[test]
    fn thumb_path_hash_matches_smash_arc() {
        assert_eq!(
            thumb_arc_path("battle_field"),
            "ui/replace/stage/stage_0_battle_field.bntx"
        );
        assert_eq!(
            hash40("fighter/mario/model/body/c00/model.numatb"),
            0x29954022ed
        );
    }
}
