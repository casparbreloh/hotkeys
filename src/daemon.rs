use std::fs;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::Duration;

use anyhow::{Result, anyhow};
use notify::{RecursiveMode, Watcher};
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy, NSEventMask};
use objc2_foundation::{MainThreadMarker, NSDate, NSDefaultRunLoopMode};

use crate::{app, config, hotkey, paths};

static SHUTDOWN: AtomicBool = AtomicBool::new(false);
static RELOAD: AtomicBool = AtomicBool::new(false);

pub fn run() -> Result<()> {
    fs::create_dir_all(paths::config_dir())?;
    if let Some(parent) = paths::log_file().parent() {
        fs::create_dir_all(parent)?;
    }

    install_signal_handlers();

    let mtm = MainThreadMarker::new().ok_or_else(|| anyhow!("must run on main thread"))?;
    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
    unsafe {
        app.finishLaunching();
    }

    hotkey::install_handler()?;
    rebind();

    let _watcher = start_watcher()?;

    eprintln!("started pid={}", unsafe { libc::getpid() });
    pump(&app);
    Ok(())
}

fn pump(app: &NSApplication) {
    loop {
        let until = unsafe { NSDate::dateWithTimeIntervalSinceNow(0.25) };
        let event = unsafe {
            app.nextEventMatchingMask_untilDate_inMode_dequeue(
                NSEventMask::Any,
                Some(&until),
                NSDefaultRunLoopMode,
                true,
            )
        };
        if let Some(event) = event {
            unsafe {
                app.sendEvent(&event);
            }
        }
        if SHUTDOWN.load(Ordering::SeqCst) {
            break;
        }
        if RELOAD.swap(false, Ordering::SeqCst) {
            rebind();
        }
    }
}

fn rebind() {
    let bindings = match config::load() {
        Ok(b) => b,
        Err(e) => {
            eprintln!("config load failed, keeping current bindings: {e:#}");
            return;
        }
    };
    let mut parsed = Vec::with_capacity(bindings.len());
    for (bundle_id, hotkey_str) in bindings {
        match hotkey::parse(&hotkey_str) {
            Ok(shortcut) => parsed.push((bundle_id, hotkey_str, shortcut)),
            Err(e) => {
                eprintln!("config parse failed, keeping current bindings: {hotkey_str}: {e}");
                return;
            }
        }
    }

    hotkey::unregister_all();
    eprintln!("reload ({} bindings)", parsed.len());
    for (bundle_id, hotkey_str, shortcut) in parsed {
        let bid = bundle_id.clone();
        let log = hotkey_str.clone();
        let r = hotkey::register(shortcut, move || {
            eprintln!("fired {log} → {bid}");
            app::activate(&bid);
        });
        if let Err(e) = r {
            eprintln!("skip {hotkey_str}: {e}");
        }
    }
}

fn start_watcher() -> Result<notify::RecommendedWatcher> {
    let (tx, rx) = mpsc::channel::<()>();
    let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        if res.is_ok() {
            let _ = tx.send(());
        }
    })?;
    watcher.watch(&paths::config_dir(), RecursiveMode::NonRecursive)?;

    std::thread::spawn(move || {
        while rx.recv().is_ok() {
            std::thread::sleep(Duration::from_millis(50));
            while rx.try_recv().is_ok() {}
            RELOAD.store(true, Ordering::SeqCst);
        }
    });
    Ok(watcher)
}

extern "C" fn handle_signal(_: libc::c_int) {
    SHUTDOWN.store(true, Ordering::SeqCst);
}

fn install_signal_handlers() {
    unsafe {
        for sig in [libc::SIGINT, libc::SIGTERM, libc::SIGHUP] {
            libc::signal(sig, handle_signal as *const () as libc::sighandler_t);
        }
    }
}
