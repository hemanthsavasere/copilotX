use crate::protocol::Message;
use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;

const INACTIVITY_TIMEOUT_MS: u32 = 2500;

pub struct HookHandle {
    pub thread_id: u32,
    pub join_handle: Option<std::thread::JoinHandle<()>>,
    pub stop_flag: Arc<AtomicBool>,
}

#[cfg(target_os = "windows")]
thread_local! {
    static HOOK_TX: RefCell<Option<Sender<Message>>> = RefCell::new(None);
    static HOOK_STOP_FLAG: RefCell<Arc<AtomicBool>> = RefCell::new(Arc::new(AtomicBool::new(false)));
    static INACTIVITY_TIMER: RefCell<std::time::Instant> = RefCell::new(std::time::Instant::now());
}

#[cfg(target_os = "windows")]
fn is_input_key(vk_code: u32) -> bool {
    matches!(
        vk_code,
        0x30..=0x39
            | 0x41..=0x5A
            | 0x20
            | 0x08
            | 0x0D
            | 0x1B
            | 0xBA..=0xC0
            | 0xDB..=0xDD
            | 0xDE
    )
}

#[cfg(target_os = "windows")]
fn map_key_event(
    vk_code: u32,
    flags: windows::Win32::UI::WindowsAndMessaging::KBDLLHOOKSTRUCT_FLAGS,
) -> Option<(String, bool, bool)> {
    use windows::Win32::UI::WindowsAndMessaging::{KBDLLHOOKSTRUCT_FLAGS, LLKHF_UP};

    if (flags & LLKHF_UP) != KBDLLHOOKSTRUCT_FLAGS(0) {
        return None;
    }

    let shift_pressed = {
        use windows::Win32::UI::Input::KeyboardAndMouse::GetKeyState;
        use windows::Win32::UI::Input::KeyboardAndMouse::VK_SHIFT;
        unsafe { GetKeyState(VK_SHIFT.0 as i32) < 0 }
    };
    let ctrl_pressed = {
        use windows::Win32::UI::Input::KeyboardAndMouse::GetKeyState;
        use windows::Win32::UI::Input::KeyboardAndMouse::VK_CONTROL;
        unsafe { GetKeyState(VK_CONTROL.0 as i32) < 0 }
    };

    if ctrl_pressed {
        return None;
    }

    match vk_code {
        0x10 | 0x11 | 0x12 => return None,
        _ => {}
    }

    let key: String = match vk_code {
        0x0D => "Enter".into(),
        0x08 => "Backspace".into(),
        0x1B => "Escape".into(),
        0x20 => " ".into(),
        0x30..=0x39 => {
            let c = (vk_code as u8 - 0x30 + b'0') as char;
            c.to_string()
        }
        0x41..=0x5A => {
            let c = if shift_pressed {
                (vk_code as u8 - 0x41 + b'A') as char
            } else {
                (vk_code as u8 - 0x41 + b'a') as char
            };
            c.to_string()
        }
        0xBA => (if shift_pressed { ':' } else { ';' }).to_string(),
        0xBB => (if shift_pressed { '+' } else { '=' }).to_string(),
        0xBC => (if shift_pressed { '<' } else { ',' }).to_string(),
        0xBD => (if shift_pressed { '_' } else { '-' }).to_string(),
        0xBE => (if shift_pressed { '>' } else { '.' }).to_string(),
        0xBF => (if shift_pressed { '?' } else { '/' }).to_string(),
        0xC0 => (if shift_pressed { '~' } else { '`' }).to_string(),
        0xDB => (if shift_pressed { '{' } else { '[' }).to_string(),
        0xDC => (if shift_pressed { '|' } else { '\\' }).to_string(),
        0xDD => (if shift_pressed { '}' } else { ']' }).to_string(),
        0xDE => (if shift_pressed { '"' } else { '\'' }).to_string(),
        _ => return None,
    };

    Some((key, shift_pressed, ctrl_pressed))
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn keyboard_hook_callback(
    code: i32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::UI::WindowsAndMessaging::*;

    if code < 0 {
        return CallNextHookEx(None, code, wparam, lparam);
    }

    let kb_struct: &KBDLLHOOKSTRUCT = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
    let vk_code = kb_struct.vkCode;
    let flags = kb_struct.flags;

    let should_pass_through = HOOK_STOP_FLAG.with(|f| f.borrow().load(Ordering::SeqCst));
    if should_pass_through {
        return CallNextHookEx(None, code, wparam, lparam);
    }

    if is_input_key(vk_code) {
        if let Some((key, shift, ctrl)) = map_key_event(vk_code, flags) {
            HOOK_TX.with(|tx_cell| {
                if let Some(tx) = tx_cell.borrow().as_ref() {
                    tx.send(Message::KeyEvent { key, shift, ctrl }).ok();
                }
            });
            INACTIVITY_TIMER.with(|t| *t.borrow_mut() = std::time::Instant::now());
        }
        return windows::Win32::Foundation::LRESULT(1);
    }

    CallNextHookEx(None, code, wparam, lparam)
}

#[cfg(target_os = "windows")]
pub fn start_keyboard_hook(tx: Sender<Message>) -> Result<HookHandle, anyhow::Error> {
    use windows::Win32::Foundation::WAIT_TIMEOUT;
    use windows::Win32::System::Threading::GetCurrentThreadId;
    use windows::Win32::UI::WindowsAndMessaging::*;

    let stop_flag = Arc::new(AtomicBool::new(false));
    let stop_flag_clone = stop_flag.clone();
    let (thread_id_tx, thread_id_rx) = std::sync::mpsc::channel::<u32>();

    let join_handle = std::thread::spawn(move || {
        INACTIVITY_TIMER.with(|t| *t.borrow_mut() = std::time::Instant::now());

        HOOK_TX.with(|tx_cell| *tx_cell.borrow_mut() = Some(tx.clone()));
        HOOK_STOP_FLAG.with(|f_cell| *f_cell.borrow_mut() = stop_flag_clone.clone());

        let hook = match unsafe {
            SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook_callback), None, 0)
        } {
            Ok(h) => h,
            Err(_) => {
                thread_id_tx.send(0).ok();
                return;
            }
        };

        let thread_id = unsafe { GetCurrentThreadId() };

        if hook.is_invalid() {
            thread_id_tx.send(0).ok();
            return;
        }

        thread_id_tx.send(thread_id).ok();

        let mut msg = MSG::default();
        loop {
            let remaining_ms = INACTIVITY_TIMEOUT_MS.saturating_sub(
                INACTIVITY_TIMER.with(|t| t.borrow().elapsed().as_millis() as u32),
            );

            let wait_result = unsafe {
                MsgWaitForMultipleObjectsEx(None, remaining_ms, QS_ALLINPUT, MWMO_ALERTABLE)
            };

            if wait_result == WAIT_TIMEOUT {
                tx.send(Message::InputModeState {
                    state: "inactive".into(),
                })
                .ok();
                unsafe { UnhookWindowsHookEx(hook).ok() };
                return;
            }

            while unsafe { PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE) }.as_bool() {
                if msg.message == WM_QUIT {
                    unsafe { UnhookWindowsHookEx(hook).ok() };
                    return;
                }
                unsafe {
                    let _ = TranslateMessage(&msg);
                    let _ = DispatchMessageW(&msg);
                }
            }
        }
    });

    let thread_id = thread_id_rx.recv()?;
    if thread_id == 0 {
        anyhow::bail!("Failed to register keyboard hook");
    }

    Ok(HookHandle {
        thread_id,
        join_handle: Some(join_handle),
        stop_flag,
    })
}

#[cfg(target_os = "windows")]
pub fn stop_keyboard_hook(handle: HookHandle) {
    use windows::Win32::Foundation::{LPARAM, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::*;

    handle.stop_flag.store(true, Ordering::SeqCst);
    unsafe {
        PostThreadMessageW(
            handle.thread_id,
            WM_QUIT,
            WPARAM::default(),
            LPARAM::default(),
        )
        .ok();
    }
    if let Some(jh) = handle.join_handle {
        let _ = jh.join();
    }
}

#[cfg(not(target_os = "windows"))]
pub fn start_keyboard_hook(_tx: Sender<Message>) -> Result<HookHandle, anyhow::Error> {
    anyhow::bail!("Input mode is not supported on this platform")
}

#[cfg(not(target_os = "windows"))]
pub fn stop_keyboard_hook(_handle: HookHandle) {}
