use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::{Arc, LazyLock, Mutex};

use anyhow::{Result, anyhow, bail};

pub const CMD: u32 = 1 << 8;
pub const SHIFT: u32 = 1 << 9;
pub const OPT: u32 = 1 << 11;
pub const CTRL: u32 = 1 << 12;

#[derive(Clone, Copy)]
pub struct Shortcut {
    pub key_code: u32,
    pub modifiers: u32,
}

pub fn parse(s: &str) -> Result<Shortcut> {
    let normalized = s
        .replace('\u{2303}', "ctrl+")
        .replace('\u{2318}', "cmd+")
        .replace('\u{2325}', "opt+")
        .replace('\u{21E7}', "shift+")
        .to_lowercase();

    let mut modifiers = 0u32;
    let mut key_code: Option<u32> = None;
    for token in normalized
        .split('+')
        .map(str::trim)
        .filter(|t| !t.is_empty())
    {
        if let Some(m) = modifier(token) {
            modifiers |= m;
        } else if let Some(k) = key_code_for(token) {
            if key_code.is_some() {
                bail!("multiple keys in `{s}`");
            }
            key_code = Some(k);
        } else {
            bail!("unknown token `{token}` in `{s}`");
        }
    }
    Ok(Shortcut {
        key_code: key_code.ok_or_else(|| anyhow!("no key in `{s}`"))?,
        modifiers,
    })
}

fn modifier(t: &str) -> Option<u32> {
    Some(match t {
        "ctrl" | "control" => CTRL,
        "cmd" | "command" => CMD,
        "opt" | "option" | "alt" => OPT,
        "shift" => SHIFT,
        _ => return None,
    })
}

fn key_code_for(t: &str) -> Option<u32> {
    Some(match t {
        "a" => 0x00,
        "s" => 0x01,
        "d" => 0x02,
        "f" => 0x03,
        "h" => 0x04,
        "g" => 0x05,
        "z" => 0x06,
        "x" => 0x07,
        "c" => 0x08,
        "v" => 0x09,
        "b" => 0x0B,
        "q" => 0x0C,
        "w" => 0x0D,
        "e" => 0x0E,
        "r" => 0x0F,
        "y" => 0x10,
        "t" => 0x11,
        "1" => 0x12,
        "2" => 0x13,
        "3" => 0x14,
        "4" => 0x15,
        "6" => 0x16,
        "5" => 0x17,
        "=" => 0x18,
        "9" => 0x19,
        "7" => 0x1A,
        "-" => 0x1B,
        "8" => 0x1C,
        "0" => 0x1D,
        "]" => 0x1E,
        "o" => 0x1F,
        "u" => 0x20,
        "[" => 0x21,
        "i" => 0x22,
        "p" => 0x23,
        "return" | "enter" => 0x24,
        "l" => 0x25,
        "j" => 0x26,
        "'" => 0x27,
        "k" => 0x28,
        ";" => 0x29,
        "\\" => 0x2A,
        "," => 0x2B,
        "/" => 0x2C,
        "n" => 0x2D,
        "m" => 0x2E,
        "." => 0x2F,
        "tab" => 0x30,
        "space" => 0x31,
        "delete" | "backspace" => 0x33,
        "escape" | "esc" => 0x35,
        "f1" => 0x7A,
        "f2" => 0x78,
        "f3" => 0x63,
        "f4" => 0x76,
        "f5" => 0x60,
        "f6" => 0x61,
        "f7" => 0x62,
        "f8" => 0x64,
        "f9" => 0x65,
        "f10" => 0x6D,
        "f11" => 0x67,
        "f12" => 0x6F,
        "left" => 0x7B,
        "right" => 0x7C,
        "down" => 0x7D,
        "up" => 0x7E,
        "home" => 0x73,
        "end" => 0x77,
        "pageup" => 0x74,
        "pagedown" => 0x79,
        _ => return None,
    })
}

type OSStatus = i32;
type OSType = u32;
type EventRef = *mut c_void;
type EventTargetRef = *mut c_void;
type EventHotKeyRef = *mut c_void;
type EventHandlerRef = *mut c_void;

#[repr(C)]
#[derive(Copy, Clone)]
struct EventHotKeyID {
    signature: OSType,
    id: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct EventTypeSpec {
    event_class: OSType,
    event_kind: u32,
}

const EVENT_CLASS_KEYBOARD: OSType = u32::from_be_bytes(*b"keyb");
const EVENT_HOTKEY_PRESSED: u32 = 5;
const EVENT_PARAM_DIRECT_OBJECT: OSType = u32::from_be_bytes(*b"----");
const TYPE_EVENT_HOTKEY_ID: OSType = u32::from_be_bytes(*b"hkid");
const HK_SIGNATURE: OSType = u32::from_be_bytes(*b"hk1 ");
const NO_ERR: OSStatus = 0;

type EventHandlerProc = unsafe extern "C" fn(*mut c_void, EventRef, *mut c_void) -> OSStatus;

#[link(name = "Carbon", kind = "framework")]
unsafe extern "C" {
    fn RegisterEventHotKey(
        in_hot_key_code: u32,
        in_hot_key_modifiers: u32,
        in_hot_key_id: EventHotKeyID,
        in_target: EventTargetRef,
        in_options: u32,
        out_ref: *mut EventHotKeyRef,
    ) -> OSStatus;
    fn UnregisterEventHotKey(ref_: EventHotKeyRef) -> OSStatus;
    fn GetApplicationEventTarget() -> EventTargetRef;
    fn InstallEventHandler(
        in_target: EventTargetRef,
        in_handler: EventHandlerProc,
        in_num_types: u32,
        in_list: *const EventTypeSpec,
        in_user_data: *mut c_void,
        out_ref: *mut EventHandlerRef,
    ) -> OSStatus;
    fn GetEventParameter(
        in_event: EventRef,
        in_name: OSType,
        in_desired_type: OSType,
        out_actual_type: *mut OSType,
        in_buffer_size: u32,
        out_actual_size: *mut u32,
        out_data: *mut c_void,
    ) -> OSStatus;
}

struct Handle(EventHotKeyRef);
unsafe impl Send for Handle {}

struct Registry {
    callbacks: HashMap<u32, Arc<dyn Fn() + Send + Sync>>,
    handles: Vec<Handle>,
    next_id: u32,
    handler_installed: bool,
}

unsafe impl Send for Registry {}

static REGISTRY: LazyLock<Mutex<Registry>> = LazyLock::new(|| {
    Mutex::new(Registry {
        callbacks: HashMap::new(),
        handles: Vec::new(),
        next_id: 1,
        handler_installed: false,
    })
});

unsafe extern "C" fn dispatch(_next: *mut c_void, event: EventRef, _user: *mut c_void) -> OSStatus {
    let mut id = EventHotKeyID {
        signature: 0,
        id: 0,
    };
    let status = unsafe {
        GetEventParameter(
            event,
            EVENT_PARAM_DIRECT_OBJECT,
            TYPE_EVENT_HOTKEY_ID,
            std::ptr::null_mut(),
            std::mem::size_of::<EventHotKeyID>() as u32,
            std::ptr::null_mut(),
            &mut id as *mut _ as *mut c_void,
        )
    };
    if status != NO_ERR {
        return status;
    }
    let cb = REGISTRY
        .lock()
        .unwrap()
        .callbacks
        .get(&id.id)
        .map(Arc::clone);
    if let Some(cb) = cb {
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| cb()));
    }
    NO_ERR
}

pub fn install_handler() -> Result<()> {
    let mut reg = REGISTRY.lock().unwrap();
    if reg.handler_installed {
        return Ok(());
    }
    let spec = EventTypeSpec {
        event_class: EVENT_CLASS_KEYBOARD,
        event_kind: EVENT_HOTKEY_PRESSED,
    };
    let mut out: EventHandlerRef = std::ptr::null_mut();
    let status = unsafe {
        InstallEventHandler(
            GetApplicationEventTarget(),
            dispatch,
            1,
            &spec,
            std::ptr::null_mut(),
            &mut out,
        )
    };
    if status != NO_ERR {
        bail!("InstallEventHandler failed ({status})");
    }
    reg.handler_installed = true;
    Ok(())
}

pub fn register(shortcut: Shortcut, callback: impl Fn() + Send + Sync + 'static) -> Result<()> {
    let mut reg = REGISTRY.lock().unwrap();
    let id = reg.next_id;
    reg.next_id += 1;
    let mut handle: EventHotKeyRef = std::ptr::null_mut();
    let status = unsafe {
        RegisterEventHotKey(
            shortcut.key_code,
            shortcut.modifiers,
            EventHotKeyID {
                signature: HK_SIGNATURE,
                id,
            },
            GetApplicationEventTarget(),
            0,
            &mut handle,
        )
    };
    if status != NO_ERR {
        bail!("RegisterEventHotKey failed ({status})");
    }
    reg.callbacks.insert(id, Arc::new(callback));
    reg.handles.push(Handle(handle));
    Ok(())
}

pub fn unregister_all() {
    let mut reg = REGISTRY.lock().unwrap();
    for handle in reg.handles.drain(..) {
        unsafe {
            UnregisterEventHotKey(handle.0);
        }
    }
    reg.callbacks.clear();
}
