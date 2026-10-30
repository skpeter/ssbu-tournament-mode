//! Attribute game wins into set_state (Phase B/C).

use crate::logutil;
use crate::ruleset;
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

    let mut stocks = [0i32; 2];
    for side in 0..2 {
        let info = FighterManager::get_fighter_information(
            mgr,
            app::FighterEntryID(side),
        );
        if info.is_null() {
            return;
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

    if let Some(w) = winner {
        let stage_id = app::sv_information::stage_id();
        if crate::board_visibility::stage_for_strip(stage_id).is_none() {
            log_zero_stage_once();
        } else {
            set_state::push_played(stage_id, w);
        }
        let done = set_state::record_win(w, None);
        logutil::write(&format!(
            "win: P{} (stocks {}-{}) stage={} set={} ft={} done={}",
            w + 1,
            stocks[0],
            stocks[1],
            stage_id,
            set_state::score_line(),
            ruleset::set_length(),
            done
        ));
    } else {
        logutil::write(&format!(
            "win: inconclusive stocks {}-{}",
            stocks[0], stocks[1]
        ));
    }
}
