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

/// Names for the stage ids this plugin records. Ids outside the list have no name.
pub fn stage_name(stage_id: i32) -> Option<&'static str> {
    Some(match stage_id {
        2 => "Town and City",
        4 => "Smashville",
        120 => "Small Battlefield",
        121 => "Battlefield",
        255 => "Final Destination",
        260 => "Northern Cave",
        318 => "Kalos Pokemon League",
        320 => "Pokemon Stadium 2",
        343 => "Hollow Bastion",
        352 => "Yoshi's Story",
        _ => return None,
    })
}

/// Zero is not a stage. Anything else can become a strip cell.
pub fn stage_for_strip(stage_id: i32) -> Option<i32> {
    if stage_id == 0 {
        None
    } else {
        Some(stage_id)
    }
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
        let cells = strip_cells(&[(4, 0), (4, 1), (0, 0), (9, 2)], Some("Red"), Some("Blue"));
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
        assert!(stage_for_strip(0).is_none());
    }

    #[test]
    fn stage_names_cover_the_manual_list() {
        assert_eq!(stage_name(121), Some("Battlefield"));
        assert_eq!(stage_name(255), Some("Final Destination"));
        assert_eq!(stage_name(318), Some("Kalos Pokemon League"));
        assert_eq!(stage_name(320), Some("Pokemon Stadium 2"));
        assert_eq!(stage_name(4), Some("Smashville"));
        assert_eq!(stage_name(2), Some("Town and City"));
        assert_eq!(stage_name(352), Some("Yoshi's Story"));
        assert_eq!(stage_name(120), Some("Small Battlefield"));
        assert_eq!(stage_name(260), Some("Northern Cave"));
        assert_eq!(stage_name(343), Some("Hollow Bastion"));
        assert_eq!(stage_name(0), None);
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
