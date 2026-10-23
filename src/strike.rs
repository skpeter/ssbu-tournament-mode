//! Strike session + TournamentMode-facing helpers (Phase C).
//!
//! Game 1 ban order is IRL (no in-plugin RPS). Software turn order starts game 2+
//! (first-strike to game-1 winner).

use crate::config;
use crate::logutil;
use crate::ruleset;
use crate::set_state;
use once_cell::sync::OnceCell;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;

static ACTIVE_GAME_INDEX: AtomicU32 = AtomicU32::new(u32::MAX);

#[derive(Clone, Debug)]
pub struct StrikeStep {
    /// Port that bans this step (0/1).
    pub port: u8,
    pub bans: u32,
}

#[derive(Clone, Debug)]
pub struct StrikeSession {
    /// Port that bans first this game (0/1). `None` = ungated (game 1 / IRL).
    pub controller: Option<u8>,
    pub step_index: u32,
    pub bans_done_this_step: u32,
    pub preset: String,
    pub steps: Vec<StrikeStep>,
    pub last_prompt: String,
}

impl Default for StrikeSession {
    fn default() -> Self {
        Self {
            controller: None,
            step_index: 0,
            bans_done_this_step: 0,
            preset: "1-2-any".into(),
            steps: Vec::new(),
            last_prompt: String::new(),
        }
    }
}

static SESSION: OnceCell<Mutex<StrikeSession>> = OnceCell::new();

fn lock() -> std::sync::MutexGuard<'static, StrikeSession> {
    SESSION
        .get_or_init(|| Mutex::new(StrikeSession::default()))
        .lock()
        .unwrap()
}

fn steps_for_preset(preset: &str, first_port: u8) -> Vec<StrikeStep> {
    let a = first_port;
    let b = 1 - first_port;
    match preset {
        "3-4-any" => vec![
            StrikeStep { port: a, bans: 3 },
            StrikeStep { port: b, bans: 4 },
        ],
        _ => vec![
            // 1-2-any (default)
            StrikeStep { port: a, bans: 1 },
            StrikeStep { port: b, bans: 2 },
        ],
    }
}

pub fn reset_for_game(game_index: u32) {
    let cfg = config::get();
    let snap = set_state::snapshot();
    let mut s = lock();
    s.preset = cfg.strike_preset.clone();
    s.step_index = 0;
    s.bans_done_this_step = 0;
    if game_index == 0 {
        // Players settle who bans first IRL; do not gate SSS inputs.
        s.controller = None;
        s.steps = Vec::new();
    } else {
        s.controller = snap.game1_winner.or(Some(0));
        let first = s.controller.unwrap_or(0);
        s.steps = steps_for_preset(&s.preset, first);
    }
    s.last_prompt = prompt_inner(&s);
    ACTIVE_GAME_INDEX.store(game_index, Ordering::SeqCst);
    logutil::write(&format!(
        "strike: reset game={} controller={:?} preset={} steps={}",
        game_index,
        s.controller,
        s.preset,
        s.steps.len()
    ));
}

/// Initialize strike state once per game index (safe to call on every SSS enter).
pub fn ensure_for_game(game_index: u32) {
    if ACTIVE_GAME_INDEX.load(Ordering::SeqCst) == game_index {
        let _ = refresh_prompt();
        return;
    }
    reset_for_game(game_index);
}

/// Clear strike session when leaving versus / rules.
pub fn reset_session() {
    ACTIVE_GAME_INDEX.store(u32::MAX, Ordering::SeqCst);
    *lock() = StrikeSession::default();
}

fn prompt_inner(s: &StrikeSession) -> String {
    let score = set_state::score_line();
    let ft = ruleset::set_length();
    if let Some(step) = s.steps.get(s.step_index as usize) {
        let left = step.bans.saturating_sub(s.bans_done_this_step);
        return format!(
            "SET {}  (FT{}) — P{} bans ({} left) [{}]",
            score,
            ft,
            step.port + 1,
            left,
            s.preset
        );
    }
    if s.controller.is_none() && s.steps.is_empty() {
        return format!("SET {}  (FT{}) — strike (manual)", score, ft);
    }
    format!("SET {}  (FT{}) — pick stage", score, ft)
}

pub fn refresh_prompt() -> String {
    let mut s = lock();
    s.last_prompt = prompt_inner(&s);
    s.last_prompt.clone()
}

pub fn current_prompt() -> String {
    let s = lock();
    if s.last_prompt.is_empty() {
        prompt_inner(&s)
    } else {
        s.last_prompt.clone()
    }
}

pub fn active_controller() -> Option<u8> {
    let s = lock();
    s.steps
        .get(s.step_index as usize)
        .map(|st| st.port)
        .or(s.controller)
}

pub fn record_ban() -> bool {
    let mut s = lock();
    if s.step_index as usize >= s.steps.len() {
        return true; // picking / ungated
    }
    s.bans_done_this_step = s.bans_done_this_step.saturating_add(1);
    let need = s.steps[s.step_index as usize].bans;
    if s.bans_done_this_step >= need {
        s.step_index += 1;
        s.bans_done_this_step = 0;
    }
    s.last_prompt = prompt_inner(&s);
    logutil::write(&format!("strike: ban → {}", s.last_prompt));
    s.step_index as usize >= s.steps.len()
}

pub fn snapshot() -> StrikeSession {
    lock().clone()
}

pub fn api_is_present() -> bool {
    true
}

pub fn api_is_active() -> bool {
    config::get().enabled && ruleset::set_length() > 1 && set_state::snapshot().probe_ok
}
