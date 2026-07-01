use std::fs;
use std::io::ErrorKind;
use std::process::Command;
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};

use crate::paths;

const PLIST_TEMPLATE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{LABEL}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{BINARY}</string>
        <string>daemon</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>ProcessType</key>
    <string>Interactive</string>
    <key>StandardOutPath</key>
    <string>{LOG}</string>
    <key>StandardErrorPath</key>
    <string>{LOG}</string>
</dict>
</plist>
"#;

pub fn install() -> Result<()> {
    let binary = std::env::current_exe()?;
    fs::create_dir_all(paths::launch_agent_dir())?;
    if let Some(parent) = paths::log_file().parent() {
        fs::create_dir_all(parent)?;
    }

    let plist = PLIST_TEMPLATE
        .replace("{LABEL}", paths::LABEL)
        .replace(
            "{BINARY}",
            &xml_escape(
                binary
                    .to_str()
                    .ok_or_else(|| anyhow!("non-utf8 binary path"))?,
            ),
        )
        .replace(
            "{LOG}",
            &xml_escape(
                paths::log_file()
                    .to_str()
                    .ok_or_else(|| anyhow!("non-utf8 log path"))?,
            ),
        );
    fs::write(paths::launch_agent(), plist)?;

    let plist_path = paths::launch_agent();
    let plist_str = plist_path
        .to_str()
        .ok_or_else(|| anyhow!("non-utf8 plist path"))?;
    let _ = launchctl(&["bootout", &service_target()]);

    let (mut rc, mut err) = launchctl(&["bootstrap", &gui_domain(), plist_str]);
    if matches!(rc, LAUNCHCTL_EIO | LAUNCHCTL_EBUSY) {
        thread::sleep(Duration::from_millis(200));
        (rc, err) = launchctl(&["bootstrap", &gui_domain(), plist_str]);
    }
    if rc != 0 {
        bail!("launchctl bootstrap exited {rc}: {}", err.trim());
    }
    println!("✓ enabled");
    Ok(())
}

pub fn uninstall() -> Result<()> {
    let _ = launchctl(&["bootout", &service_target()]);
    match fs::remove_file(paths::launch_agent()) {
        Ok(()) => {}
        Err(e) if e.kind() == ErrorKind::NotFound => {}
        Err(e) => return Err(e).context("removing LaunchAgent plist"),
    }
    println!("✓ disabled");
    Ok(())
}

const LAUNCHCTL_EIO: i32 = 5;
const LAUNCHCTL_EBUSY: i32 = 16;

fn launchctl(args: &[&str]) -> (i32, String) {
    match Command::new("/bin/launchctl")
        .args(args)
        .stdout(std::process::Stdio::null())
        .output()
    {
        Ok(out) => (
            out.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&out.stderr).into_owned(),
        ),
        Err(e) => (-1, e.to_string()),
    }
}

fn gui_domain() -> String {
    format!("gui/{}", unsafe { libc::getuid() })
}

fn service_target() -> String {
    format!("{}/{}", gui_domain(), paths::LABEL)
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
