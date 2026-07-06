use std::process::Command;

use anyhow::{Result, bail};
use objc2_app_kit::{NSWorkspace, NSWorkspaceOpenConfiguration};
use objc2_foundation::NSString;

pub fn resolve(input: &str) -> Result<String> {
    if is_bundle_id(input) && bundle_exists(input) {
        return Ok(input.to_string());
    }
    let script = format!("id of app \"{}\"", input.replace('"', "\\\""));
    let out = Command::new("/usr/bin/osascript")
        .args(["-e", &script])
        .output()?;
    let id = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if !out.status.success() || id.is_empty() {
        bail!("app not found: `{input}`");
    }
    Ok(id)
}

fn is_bundle_id(input: &str) -> bool {
    input.contains('.') && !input.contains(' ')
}

fn bundle_exists(bundle_id: &str) -> bool {
    unsafe {
        NSWorkspace::sharedWorkspace()
            .URLForApplicationWithBundleIdentifier(&NSString::from_str(bundle_id))
            .is_some()
    }
}

pub fn activate(bundle_id: &str) {
    unsafe {
        let ws = NSWorkspace::sharedWorkspace();
        let Some(url) = ws.URLForApplicationWithBundleIdentifier(&NSString::from_str(bundle_id))
        else {
            eprintln!("no app for {bundle_id}");
            return;
        };
        let cfg = NSWorkspaceOpenConfiguration::new();
        cfg.setActivates(true);
        ws.openApplicationAtURL_configuration_completionHandler(&url, &cfg, None);
    }
}
