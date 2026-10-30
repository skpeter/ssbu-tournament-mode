use crate::logutil;
use once_cell::sync::OnceCell;
use std::sync::RwLock;

const CONFIG_PATH: &str = "sd:/ultimate/tournament_mode.toml";

#[derive(Clone, Debug)]
pub struct Config {
    pub enabled: bool,
    /// Character Select before Stage Select (former standalone css_first plugin).
    pub css_first: bool,
    pub strike_preset: String,
    pub auto_ban_counterpicks_after_g1: bool,
    pub auto_strike_picker_prior_wins: bool,
    pub scoreboard: bool,
    /// Exact `info_melee` pane name to bind (from hud_inventory log).
    /// Default `txt_gsp` (unused in local Versus). Empty TOML keeps this default.
    pub scoreboard_pane: String,
    pub set_callout_on_final: bool,
    pub log_level: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: true,
            css_first: true,
            strike_preset: "1-2-any".to_string(),
            auto_ban_counterpicks_after_g1: true,
            auto_strike_picker_prior_wins: true,
            scoreboard: true,
            scoreboard_pane: "txt_gsp".to_string(),
            set_callout_on_final: false,
            log_level: "info".to_string(),
        }
    }
}

static CONFIG: OnceCell<RwLock<Config>> = OnceCell::new();

fn cell() -> &'static RwLock<Config> {
    CONFIG.get_or_init(|| RwLock::new(Config::default()))
}

pub fn get() -> Config {
    cell().read().map(|g| g.clone()).unwrap_or_default()
}

/// Minimal TOML-ish parser: `key = value` lines only (bool / string).
pub fn load() {
    let mut cfg = Config::default();
    let text = match std::fs::read_to_string(CONFIG_PATH) {
        Ok(t) => t,
        Err(_) => {
            logutil::write(&format!(
                "config: missing {}, using defaults",
                CONFIG_PATH
            ));
            *cell().write().unwrap() = cfg;
            return;
        }
    };
    // Strip UTF-8 BOM if present (common on SD edits from Windows).
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text);

    for raw in text.lines() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let mut parts = line.splitn(2, '=');
        let key = parts.next().unwrap_or("").trim();
        let val = parts.next().unwrap_or("").trim().trim_matches('"');
        match key {
            "enabled" => cfg.enabled = parse_bool(val, cfg.enabled),
            "css_first" => cfg.css_first = parse_bool(val, cfg.css_first),
            // Removed: game-1 who-bans is IRL-only. Ignore stale SD keys.
            "rps_prompt_game1" => {}
            "strike_preset" => cfg.strike_preset = val.to_string(),
            "auto_ban_counterpicks_after_g1" => {
                cfg.auto_ban_counterpicks_after_g1 = parse_bool(val, cfg.auto_ban_counterpicks_after_g1)
            }
            "auto_strike_picker_prior_wins" => {
                cfg.auto_strike_picker_prior_wins = parse_bool(val, cfg.auto_strike_picker_prior_wins)
            }
            "scoreboard" => cfg.scoreboard = parse_bool(val, cfg.scoreboard),
            "scoreboard_pane" => {
                if !val.is_empty() {
                    cfg.scoreboard_pane = val.to_string();
                }
            }
            "set_callout_on_final" => {
                cfg.set_callout_on_final = parse_bool(val, cfg.set_callout_on_final)
            }
            "log_level" => cfg.log_level = val.to_string(),
            // Removed. Ignore stale SD keys.
            "first_to_override" | "set_length" => {}
            other => logutil::write(&format!("config: unknown key '{}'", other)),
        }
    }

    logutil::write(&format!(
        "config: loaded enabled={} css_first={} preset={} scoreboard={} pane='{}' set_callout={}",
        cfg.enabled,
        cfg.css_first,
        cfg.strike_preset,
        cfg.scoreboard,
        cfg.scoreboard_pane,
        cfg.set_callout_on_final
    ));
    *cell().write().unwrap() = cfg;
}

fn parse_bool(val: &str, default: bool) -> bool {
    match val.to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => true,
        "false" | "0" | "no" | "off" => false,
        _ => default,
    }
}
