use crate::logutil;
use once_cell::sync::OnceCell;
use std::sync::Mutex;

/// One finished game. `stage_id` is `app::stage::get_stage_id`. `winner` is 0 or 1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlayedGame {
    pub stage_id: i32,
    pub winner: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SetPhase {
    Idle,
    InSet,
    BetweenGames,
    FinalGame,
    Complete,
}

#[derive(Clone, Debug)]
pub struct SetState {
    pub phase: SetPhase,
    /// Intended First-to from ruleset (N). 1 means plugin inert for set logic.
    pub first_to: u32,
    pub wins: [u32; 2],
    pub games_played: u32,
    /// Port index (0/1) that won game 1; used for strike control later.
    pub game1_winner: Option<u8>,
    /// Stages (panel ids) each side has won on this set (DSR).
    pub won_stages: [Vec<i32>; 2],
    /// Games in this set, in play order. Repeats stay. Not the strike list.
    pub played: Vec<PlayedGame>,
    pub last_scene: String,
    pub probe_ok: bool,
    pub inert_reason: String,
}

impl Default for SetState {
    fn default() -> Self {
        Self {
            phase: SetPhase::Idle,
            first_to: 1,
            wins: [0, 0],
            games_played: 0,
            game1_winner: None,
            won_stages: [Vec::new(), Vec::new()],
            played: Vec::new(),
            last_scene: String::new(),
            probe_ok: false,
            inert_reason: "boot".to_string(),
        }
    }
}

static STATE: OnceCell<Mutex<SetState>> = OnceCell::new();

fn lock() -> std::sync::MutexGuard<'static, SetState> {
    STATE
        .get_or_init(|| Mutex::new(SetState::default()))
        .lock()
        .unwrap()
}

pub fn snapshot() -> SetState {
    lock().clone()
}

pub fn set_probe(result: Result<(), &'static str>) {
    let mut s = lock();
    match result {
        Ok(()) => {
            s.probe_ok = true;
            s.inert_reason.clear();
            logutil::write("probe: ok");
        }
        Err(reason) => {
            s.probe_ok = false;
            s.inert_reason = reason.to_string();
            logutil::write(&format!("probe: FAIL — {}", reason));
        }
    }
}

pub fn set_first_to(n: u32) {
    let mut s = lock();
    s.first_to = n.max(1);
    if s.first_to <= 1 {
        s.phase = SetPhase::Idle;
        s.inert_reason = "first_to<=1".to_string();
    } else if s.phase == SetPhase::Idle {
        s.phase = SetPhase::InSet;
        s.inert_reason.clear();
    }
    logutil::write(&format!("set: first_to={} phase={:?}", s.first_to, s.phase));
}

pub fn note_scene(scene: &str) {
    let mut s = lock();
    s.last_scene = scene.to_string();
    if scene == "MeleeRuleScene" || scene == "MainMenuScene" {
        reset_inner(&mut s, "left_versus_or_rules");
    }
}

#[cfg(test)]
pub fn reset(reason: &str) {
    let mut s = lock();
    reset_inner(&mut s, reason);
}

/// Next set after first-to. Keeps the ruleset length and the probe.
pub fn begin_next_set() {
    let mut s = lock();
    s.wins = [0, 0];
    s.games_played = 0;
    s.game1_winner = None;
    s.won_stages = [Vec::new(), Vec::new()];
    s.played.clear();
    s.phase = SetPhase::InSet;
    s.inert_reason.clear();
    logutil::write("set: next set 0-0");
}

fn reset_inner(s: &mut SetState, reason: &str) {
    let first_to = s.first_to;
    let probe_ok = s.probe_ok;
    *s = SetState::default();
    s.first_to = first_to;
    s.probe_ok = probe_ok;
    if first_to > 1 && probe_ok {
        s.phase = SetPhase::Idle;
    }
    s.inert_reason = reason.to_string();
    logutil::write(&format!("set: reset ({})", reason));
}

/// Append one played game. Does not write `won_stages`.
/// A zero stage id is ignored. Returns false when the set is not tracking games.
pub fn push_played(stage_id: i32, winner: u8) -> bool {
    if stage_id == 0 || winner > 1 {
        return false;
    }
    let mut s = lock();
    if s.first_to <= 1 || !s.probe_ok {
        return false;
    }
    let cap = (s.first_to.saturating_mul(2).saturating_sub(1)) as usize;
    if s.played.len() >= cap {
        return false;
    }
    s.played.push(PlayedGame { stage_id, winner });
    true
}

/// Record a game win for `side` (0 or 1). Returns true if the set is now complete.
pub fn record_win(side: u8, stage_panel: Option<i32>) -> bool {
    let mut s = lock();
    if s.first_to <= 1 || !s.probe_ok {
        return false;
    }
    let idx = if side > 0 { 1 } else { 0 };
    s.wins[idx] = s.wins[idx].saturating_add(1);
    s.games_played = s.games_played.saturating_add(1);
    if s.game1_winner.is_none() {
        s.game1_winner = Some(idx as u8);
    }
    if let Some(panel) = stage_panel {
        if !s.won_stages[idx].contains(&panel) {
            s.won_stages[idx].push(panel);
        }
    }
    let done = s.wins[idx] >= s.first_to;
    s.phase = if done {
        SetPhase::Complete
    } else if s.wins[0] + 1 == s.first_to || s.wins[1] + 1 == s.first_to {
        SetPhase::FinalGame
    } else {
        SetPhase::BetweenGames
    };
    logutil::write(&format!(
        "set: score {}-{} games={} phase={:?} winner_side={}",
        s.wins[0], s.wins[1], s.games_played, s.phase, idx
    ));
    done
}

pub fn score_line() -> String {
    let s = lock();
    format!("{}-{}", s.wins[0], s.wins[1])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tracking_set() {
        reset("test");
        set_probe(Ok(()));
        set_first_to(2);
    }

    #[test]
    fn played_list_keeps_order_and_leaves_strike_list() {
        tracking_set();
        assert!(!push_played(0, 0));
        assert!(push_played(4, 0));
        assert!(push_played(4, 1));
        let snap = snapshot();
        assert_eq!(
            snap.played,
            vec![
                PlayedGame {
                    stage_id: 4,
                    winner: 0
                },
                PlayedGame {
                    stage_id: 4,
                    winner: 1
                },
            ]
        );
        assert!(snap.won_stages[0].is_empty());
        assert!(snap.won_stages[1].is_empty());

        let _ = record_win(0, None);
        let after_win = snapshot();
        assert!(after_win.won_stages[0].is_empty());
        assert_eq!(after_win.played.len(), 2);

        begin_next_set();
        assert!(snapshot().played.is_empty());
        reset("test-done");
    }
}
