use crate::protocol::Message;
use std::sync::mpsc::Sender;

pub struct HookHandle {
    _private: (),
}

#[cfg(not(target_os = "windows"))]
pub fn start_keyboard_hook(_tx: Sender<Message>) -> Result<HookHandle, anyhow::Error> {
    anyhow::bail!("Input mode is not supported on this platform")
}

#[cfg(not(target_os = "windows"))]
pub fn stop_keyboard_hook(_handle: HookHandle) {}
