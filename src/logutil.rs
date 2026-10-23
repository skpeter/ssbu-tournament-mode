use std::io::Write;

pub const LOG_PATH: &str = "sd:/tournament_mode.log";

pub fn write(line: &str) {
    let mut file = match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(LOG_PATH)
    {
        Ok(f) => f,
        Err(_) => return,
    };
    let _ = writeln!(file, "{}", line);
}

pub fn reset(banner: &str) {
    let _ = std::fs::write(LOG_PATH, format!("{}\n", banner));
}
