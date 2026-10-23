//! Between-game SSS bookkeeping (strike session + log). No RPS / who-bans UI.

use crate::logutil;
use crate::set_state;
use crate::strike;

/// Call **after** StageSelectScene transition completes.
pub fn on_stage_select() {
    if !strike::api_is_active() {
        return;
    }

    let games = set_state::snapshot().games_played;
    strike::ensure_for_game(games);
    let prompt = strike::refresh_prompt();
    logutil::write(&format!("ui: {}", prompt));
}

pub fn on_leave_versus() {
    strike::reset_session();
}
