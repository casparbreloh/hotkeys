use std::path::PathBuf;

pub const LABEL: &str = "dev.casparbreloh.hotkeys";

pub fn config_file() -> PathBuf {
    config_dir().join("bindings.toml")
}

pub fn config_dir() -> PathBuf {
    xdg("XDG_CONFIG_HOME", ".config").join("hotkeys")
}

pub fn log_file() -> PathBuf {
    state_dir().join("hotkeys.log")
}

pub fn launch_agent_dir() -> PathBuf {
    home().join("Library/LaunchAgents")
}

pub fn launch_agent() -> PathBuf {
    launch_agent_dir().join(format!("{LABEL}.plist"))
}

fn state_dir() -> PathBuf {
    xdg("XDG_STATE_HOME", ".local/state").join("hotkeys")
}

fn home() -> PathBuf {
    PathBuf::from(std::env::var_os("HOME").expect("$HOME is not set"))
}

fn xdg(var: &str, fallback: &str) -> PathBuf {
    std::env::var_os(var)
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .unwrap_or_else(|| home().join(fallback))
}
