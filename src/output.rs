use anyhow::{bail, Result};
use windows::Win32::{
    Foundation::HWND,
    UI::{
        Input::KeyboardAndMouse::*,
        WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId},
    },
};
pub fn target() -> Option<usize> {
    unsafe {
        let window = GetForegroundWindow();
        let mut process = 0;
        GetWindowThreadProcessId(window, Some(&mut process));
        if window.0.is_null() || process == std::process::id() {
            None
        } else {
            Some(window.0 as usize)
        }
    }
}
pub fn type_text(target: usize, text: &str) -> Result<()> {
    unsafe {
        if GetForegroundWindow() != HWND(target as *mut std::ffi::c_void) {
            bail!("Text is ready in Whispering. The destination changed, so automatic typing was skipped.");
        }
        for key in [VK_CONTROL, VK_SHIFT, VK_MENU, VK_LWIN, VK_RWIN] {
            if GetAsyncKeyState(key.0 as i32) < 0 {
                bail!("Text is ready in Whispering. Release modifier keys before the next transcription finishes.");
            }
        }
        let inputs: Vec<INPUT> = text
            .encode_utf16()
            .flat_map(|unit| {
                [KEYEVENTF_UNICODE, KEYEVENTF_UNICODE | KEYEVENTF_KEYUP].map(|flags| INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: VIRTUAL_KEY(0),
                            wScan: unit,
                            dwFlags: flags,
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                })
            })
            .collect();
        if SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) != inputs.len() as u32 {
            bail!("Windows blocked some or all automatic typing (for example, in an administrator app). Your full transcript is saved in History.");
        }
        Ok(())
    }
}

// Clicking Retry temporarily focuses our indicator. Return to the dictation
// destination; type_text still verifies focus when inference finishes.
pub fn restore_target(target: usize) {
    unsafe {
        let _ = windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow(HWND(
            target as *mut std::ffi::c_void,
        ));
    }
}
