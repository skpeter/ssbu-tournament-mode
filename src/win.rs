//! Attribute game wins into set_state (Phase B/C).

use crate::logutil;
use crate::ruleset;
use crate::scoreboard;
use crate::set_state;
use crate::strike;
use crate::FIGHTER_MANAGER_ADDR;
use skyline_smash::app::{self, lua_bind::*};
use std::sync::atomic::{AtomicBool, Ordering};

static ZERO_STAGE_LOGGED: AtomicBool = AtomicBool::new(false);

fn log_zero_stage_once() {
    if !ZERO_STAGE_LOGGED.swap(true, Ordering::Relaxed) {
        logutil::write("win: stage id missing, no strip cell");
    }
}

/// Best-effort: after a game, decide winner from stocks (stock matches).
/// Singles compares entry 0 vs 1. Doubles sums stocks per team color.
pub unsafe fn try_attribute_from_fighter_manager() {
    if !strike::api_is_active() {
        return;
    }
    if FIGHTER_MANAGER_ADDR == 0 {
        return;
    }
    let mgr = *(FIGHTER_MANAGER_ADDR as *mut *mut app::FighterManager);
    if mgr.is_null() {
        return;
    }

    let entry_count = FighterManager::entry_count(mgr);
    if entry_count < 2 {
        return;
    }

    let roster = scoreboard::side_roster();
    let winner = if roster.doubles {
        attribute_doubles(mgr, entry_count as usize, &roster)
    } else {
        attribute_singles(mgr)
    };

    if let Some(w) = winner {
        let selected = app::sv_information::stage_id();
        let loaded = app::stage::get_stage_id();
        let cached = scoreboard::cached_stage();
        let stage_id = crate::board_visibility::pick_played_stage(selected, loaded, cached);
        if let Some(stage_id) = stage_id {
            set_state::push_played(stage_id, w);
        } else {
            log_zero_stage_once();
        }
        let done = set_state::record_win(w, None);
        logutil::write(&format!(
            "win: side{} doubles={} selected={} loaded={} cached={:?} used={:?} set={} ft={} done={}",
            w + 1,
            roster.doubles,
            selected,
            loaded,
            cached,
            stage_id,
            set_state::score_line(),
            ruleset::set_length(),
            done
        ));
    }
}

unsafe fn attribute_singles(mgr: *mut app::FighterManager) -> Option<u8> {
    let mut stocks = [0i32; 2];
    for side in 0..2 {
        let info = FighterManager::get_fighter_information(mgr, app::FighterEntryID(side));
        if info.is_null() {
            return None;
        }
        stocks[side as usize] = FighterInformation::stock_count(info) as i32;
    }
    let winner = if stocks[0] > stocks[1] {
        Some(0u8)
    } else if stocks[1] > stocks[0] {
        Some(1u8)
    } else {
        None
    };
    if winner.is_none() {
        logutil::write(&format!(
            "win: inconclusive stocks {}-{}",
            stocks[0], stocks[1]
        ));
    } else {
        logutil::write(&format!(
            "win: singles stocks {}-{}",
            stocks[0], stocks[1]
        ));
    }
    winner
}

unsafe fn attribute_doubles(
    mgr: *mut app::FighterManager,
    entry_count: usize,
    roster: &scoreboard::SideRoster,
) -> Option<u8> {
    let mut stocks = [0i32; 2];
    for entry in 0..entry_count {
        let info = FighterManager::get_fighter_information(mgr, app::FighterEntryID(entry as i32));
        if info.is_null() {
            continue;
        }
        let count = FighterInformation::stock_count(info) as i32;
        if roster.left.contains(&entry) {
            stocks[0] = stocks[0].saturating_add(count);
        } else if roster.right.contains(&entry) {
            stocks[1] = stocks[1].saturating_add(count);
        }
    }
    let winner = if stocks[0] > stocks[1] {
        Some(0u8)
    } else if stocks[1] > stocks[0] {
        Some(1u8)
    } else {
        None
    };
    if winner.is_none() {
        logutil::write(&format!(
            "win: doubles inconclusive stocks {}-{} left={:?} right={:?}",
            stocks[0], stocks[1], roster.left, roster.right
        ));
    } else {
        logutil::write(&format!(
            "win: doubles stocks {}-{} left={:?} right={:?}",
            stocks[0], stocks[1], roster.left, roster.right
        ));
    }
    winner
}
