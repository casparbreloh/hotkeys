mod app;
mod config;
mod daemon;
mod hotkey;
mod launchd;
mod paths;

use anyhow::{Result, anyhow};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(arg_required_else_help = true)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Bind an app to a global hotkey
    Bind {
        /// App name (e.g. "Safari") or bundle id (e.g. com.apple.Safari)
        app: String,
        /// Hotkey like cmd+shift+a or ctrl+opt+f12
        hotkey: String,
    },
    /// Remove an app's binding
    Unbind {
        /// App name or bundle id
        app: String,
    },
    /// List current bindings
    List,
    /// Install and start the daemon
    Enable,
    /// Stop and uninstall the daemon
    Disable,
    #[command(hide = true)]
    Daemon,
}

fn main() -> Result<()> {
    match Cli::parse().cmd {
        Cmd::Bind { app, hotkey } => bind(app, hotkey),
        Cmd::Unbind { app } => unbind(app),
        Cmd::List => list(),
        Cmd::Enable => launchd::install(),
        Cmd::Disable => launchd::uninstall(),
        Cmd::Daemon => daemon::run(),
    }
}

fn bind(app_input: String, hotkey: String) -> Result<()> {
    let bundle_id = app::resolve(&app_input)?;
    hotkey::parse(&hotkey)?;
    let mut bindings = config::load()?;
    let displaced: Vec<String> = bindings
        .iter()
        .filter(|(k, v)| *v == &hotkey && **k != bundle_id)
        .map(|(k, _)| k.clone())
        .collect();
    for k in &displaced {
        bindings.remove(k);
    }
    bindings.insert(bundle_id.clone(), hotkey.clone());
    config::save(&bindings)?;
    for d in &displaced {
        status("unbound", d);
    }
    status("bound", format_args!("{bundle_id} {hotkey}"));
    Ok(())
}

fn unbind(input: String) -> Result<()> {
    let mut bindings = config::load()?;
    let bundle_id = match bindings
        .keys()
        .find(|k| k.eq_ignore_ascii_case(&input))
        .cloned()
    {
        Some(b) => b,
        None => app::resolve(&input)?,
    };
    let hotkey = bindings
        .remove(&bundle_id)
        .ok_or_else(|| anyhow!("no binding for {input}"))?;
    config::save(&bindings)?;
    status("unbound", format_args!("{bundle_id} {hotkey}"));
    Ok(())
}

fn list() -> Result<()> {
    let bindings = config::load()?;
    if bindings.is_empty() {
        println!("no bindings - `hotkeys bind <app> <hotkey>`");
        return Ok(());
    }
    let width = bindings.keys().map(String::len).max().unwrap_or(0);
    for (app, hotkey) in &bindings {
        println!("{app:<width$}  {hotkey}");
    }
    Ok(())
}

fn status(verb: &str, object: impl std::fmt::Display) {
    println!("✓ {verb} {object}");
}
