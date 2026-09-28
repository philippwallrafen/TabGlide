use super::OwnedHandle;
use crate::executor::WindowOperations;
use std::time::{Duration, Instant};
use tabglide_core::{ApplicationId, TabDirection, WindowContext, WindowId};
use windows::{
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        System::Threading::*,
        UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
    },
    core::PWSTR,
};

fn identity(hwnd: HWND) -> Option<WindowId> {
    // SAFETY: these APIs validate foreign window handles; output pid is a valid local buffer.
    unsafe {
        if !IsWindow(Some(hwnd)).as_bool() {
            return None;
        }
        let mut pid = 0;
        let tid = GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == 0 || tid == 0 {
            return None;
        }
        Some(WindowId(
            (hwnd.0 as usize as u128) | ((pid as u128) << 64) | ((tid as u128) << 96),
        ))
    }
}

fn handle(window: WindowId) -> HWND {
    HWND(window.0 as usize as *mut std::ffi::c_void)
}

fn application(window: WindowId) -> Option<String> {
    // SAFETY: limited query permission only, with an owned process handle and bounded output buffer.
    unsafe {
        let process = OwnedHandle(
            OpenProcess(
                PROCESS_QUERY_LIMITED_INFORMATION,
                false,
                (window.0 >> 64) as u32,
            )
            .ok()?,
        );
        let mut buffer = [0u16; 32768];
        let mut length = buffer.len() as u32;
        QueryFullProcessImageNameW(
            process.0,
            PROCESS_NAME_WIN32,
            PWSTR(buffer.as_mut_ptr()),
            &mut length,
        )
        .ok()?;
        let path = String::from_utf16_lossy(&buffer[..length as usize]);
        Some(path.rsplit('\\').next()?.to_lowercase())
    }
}

pub(super) fn context_at(point: POINT) -> Option<WindowContext> {
    // SAFETY: value POINT and initialized MONITORINFO; borrowed foreign windows are never destroyed.
    unsafe {
        let hovered_window = identity(GetAncestor(WindowFromPoint(point), GA_ROOT))?;
        let hovered_application = ApplicationId::new(&application(hovered_window)?);
        let monitor = MonitorFromPoint(point, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if !GetMonitorInfoW(monitor, &mut info).as_bool() {
            return None;
        }
        let focused_window = identity(GetForegroundWindow());
        Some(WindowContext {
            hovered_window,
            focused_window,
            hovered_application,
            pointer_y_from_monitor_top_px: point.y.checked_sub(info.rcMonitor.top)?,
        })
    }
}

pub(super) struct NativeWindows;

pub(super) fn modifiers_pressed() -> bool {
    // SAFETY: virtual key constants only, no pointers.
    unsafe {
        [VK_CONTROL, VK_SHIFT, VK_MENU, VK_LWIN, VK_RWIN]
            .iter()
            .any(|key| GetAsyncKeyState(key.0 as i32) < 0)
    }
}

impl WindowOperations for NativeWindows {
    fn is_valid(&self, window: WindowId) -> bool {
        identity(handle(window)) == Some(window)
    }
    fn foreground(&self) -> Option<WindowId> {
        // SAFETY: getter takes no pointers and returns a borrowed handle, validated by identity.
        identity(unsafe { GetForegroundWindow() })
    }
    fn activate(&mut self, window: WindowId) -> bool {
        // SAFETY: executor validated window; Win32 tolerates destruction between check and call.
        unsafe { SetForegroundWindow(handle(window)).as_bool() }
    }
    fn wait_for_foreground(&self, window: WindowId, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        loop {
            if self.foreground() == Some(window) {
                return true;
            }
            if !self.is_valid(window) || Instant::now() >= deadline {
                return false;
            }
            // This is a bounded confirmation wait only after a requested focus transition, not
            // background polling. Yielding avoids burning CPU while Windows completes activation.
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    fn send_tab(&mut self, direction: TabDirection) -> bool {
        // Do not interfere with physically held modifiers. AHK's implicit modifier rewriting is
        // intentionally replaced by skipping modified wheel gestures for predictable key state.
        if modifiers_pressed() {
            return false;
        }
        let key = |code, release| INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: code,
                    dwFlags: if release {
                        KEYEVENTF_KEYUP
                    } else {
                        KEYBD_EVENT_FLAGS(0)
                    },
                    ..Default::default()
                },
            },
        };
        let mut inputs = [INPUT::default(); 6];
        let mut count = 0;
        inputs[count] = key(VK_CONTROL, false);
        count += 1;
        if direction == TabDirection::Previous {
            inputs[count] = key(VK_SHIFT, false);
            count += 1;
        }
        inputs[count] = key(VK_TAB, false);
        count += 1;
        inputs[count] = key(VK_TAB, true);
        count += 1;
        if direction == TabDirection::Previous {
            inputs[count] = key(VK_SHIFT, true);
            count += 1;
        }
        inputs[count] = key(VK_CONTROL, true);
        count += 1;
        // SAFETY: initialized INPUT array has the correct byte size and no external pointers.
        let sent = unsafe { SendInput(&inputs[..count], std::mem::size_of::<INPUT>() as i32) };
        if sent != count as u32 {
            // A partial insertion may leave our synthetic keys held; best-effort release only
            // keys from this sequence. A zero result (including UIPI) did not press any keys.
            if sent > 0 {
                let mut releases = [INPUT::default(); 3];
                let mut release_count = 0;
                for input in inputs[..sent as usize].iter().rev() {
                    // SAFETY: every input above was constructed as INPUT_KEYBOARD.
                    let keyboard = unsafe { input.Anonymous.ki };
                    if keyboard.dwFlags == KEYBD_EVENT_FLAGS(0) {
                        releases[release_count] = key(keyboard.wVk, true);
                        release_count += 1;
                    }
                }
                // SAFETY: same initialized layout, key-up-only cleanup after partial insertion.
                unsafe {
                    SendInput(
                        &releases[..release_count],
                        std::mem::size_of::<INPUT>() as i32,
                    );
                }
            }
            tracing::warn!(
                sent,
                expected = count,
                "SendInput failed; UIPI may block higher-integrity targets"
            );
            return false;
        }
        true
    }
}
