//! Opt-in interactive smoke test using only windows owned by this test process.
//! Run on an unlocked desktop with no keys held; cursor/focus are restored on exit.
use super::*;
use crate::executor::WindowOperations;
use std::cell::RefCell;
use tabglide_core::{Command, TabDirection};
use windows::{
    Win32::{System::LibraryLoader::GetModuleHandleW, UI::Input::KeyboardAndMouse::*},
    core::{PCWSTR, w},
};

#[derive(Default)]
struct ObservedInput {
    wheels: u32,
    tabs: Vec<TabDirection>,
}

unsafe extern "system" fn test_window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // SAFETY: WM_NCCREATE supplies CREATESTRUCTW; lpCreateParams points to the Box kept by
    // TestWindow until after DestroyWindow. Only the creating test thread accesses that state.
    unsafe {
        if message == WM_NCCREATE {
            let create = &*(lparam.0 as *const CREATESTRUCTW);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize);
        }
        let state = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const RefCell<ObservedInput>;
        if !state.is_null()
            && let Ok(mut observed) = (*state).try_borrow_mut()
        {
            if message == WM_MOUSEWHEEL {
                observed.wheels += 1;
                return LRESULT(0);
            }
            if message == WM_KEYDOWN
                && wparam.0 == VK_TAB.0 as usize
                && GetKeyState(VK_CONTROL.0 as i32) < 0
            {
                observed.tabs.push(if GetKeyState(VK_SHIFT.0 as i32) < 0 {
                    TabDirection::Previous
                } else {
                    TabDirection::Next
                });
                return LRESULT(0);
            }
        }
        DefWindowProcW(hwnd, message, wparam, lparam)
    }
}

struct TestWindow {
    hwnd: HWND,
    observed: Box<RefCell<ObservedInput>>,
}
impl TestWindow {
    fn new(x: i32, y: i32) -> Self {
        let observed = Box::<RefCell<ObservedInput>>::default();
        // SAFETY: stable Box address, registered callback and static strings outlive the window.
        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_TOOLWINDOW,
                w!("TabGlide.SmokeTest"),
                w!("TabGlide test window"),
                WS_POPUP | WS_VISIBLE,
                x,
                y,
                300,
                150,
                None,
                None,
                Some(GetModuleHandleW(PCWSTR::null()).unwrap().into()),
                Some((&*observed as *const RefCell<ObservedInput>).cast()),
            )
            .unwrap()
        };
        Self { hwnd, observed }
    }
}
impl Drop for TestWindow {
    fn drop(&mut self) {
        // SAFETY: owned window destroyed on its thread before its Box is freed.
        unsafe {
            let _ = DestroyWindow(self.hwnd);
        }
    }
}
struct RestoreDesktop {
    cursor: POINT,
    foreground: HWND,
}
impl Drop for RestoreDesktop {
    fn drop(&mut self) {
        // SAFETY: restore saved cursor and best-effort valid original foreground handle.
        unsafe {
            let _ = SetCursorPos(self.cursor.x, self.cursor.y);
            if IsWindow(Some(self.foreground)).as_bool() {
                let _ = SetForegroundWindow(self.foreground);
            }
        }
    }
}

fn pump_until(mut condition: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        let mut message = MSG::default();
        // SAFETY: initialized buffer, same-thread owned windows; callbacks own their state.
        unsafe {
            while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        if condition() {
            return;
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        assert!(
            !remaining.is_zero(),
            "desktop input was not delivered before timeout"
        );
        // SAFETY: no handles, event-driven wait for queued messages or a finite test deadline.
        unsafe {
            MsgWaitForMultipleObjectsEx(
                None,
                remaining.as_millis().max(1) as u32,
                QS_ALLINPUT,
                MWMO_INPUTAVAILABLE,
            );
        }
    }
}

#[test]
#[ignore = "interactive desktop: briefly creates windows and moves/restores pointer and focus"]
fn real_hook_passthrough_context_keys_focus_and_shutdown() {
    assert!(
        !window::modifiers_pressed(),
        "release physical modifiers first"
    );
    let mut saved_cursor = POINT::default();
    // SAFETY: output buffer and no-argument getter.
    let desktop = unsafe {
        GetCursorPos(&mut saved_cursor).unwrap();
        RestoreDesktop {
            cursor: saved_cursor,
            foreground: GetForegroundWindow(),
        }
    };
    // SAFETY: test class with static name/callback, no retained references in class registration.
    unsafe {
        let class = WNDCLASSW {
            lpfnWndProc: Some(test_window_proc),
            hInstance: GetModuleHandleW(PCWSTR::null()).unwrap().into(),
            lpszClassName: w!("TabGlide.SmokeTest"),
            ..Default::default()
        };
        assert_ne!(RegisterClassW(&class), 0);
    }
    let first = TestWindow::new(100, 0);
    let second = TestWindow::new(450, 0);
    // SAFETY: owned live windows and known client coordinates.
    unsafe {
        let _ = SetForegroundWindow(first.hwnd);
        SetCursorPos(120, 20).unwrap();
    }
    let mut windows = window::NativeWindows;
    let first_id = window::context_at(POINT { x: 120, y: 20 })
        .unwrap()
        .hovered_window;
    let second_id = window::context_at(POINT { x: 470, y: 20 })
        .unwrap()
        .hovered_window;
    assert_eq!(
        windows.foreground(),
        Some(first_id),
        "desktop must allow test foreground activation"
    );
    let context = window::context_at(POINT { x: 120, y: 20 }).unwrap();
    assert_eq!(context.pointer_y_from_monitor_top_px, 20);
    let hook = hook::MouseHook::start().unwrap();
    let wheel = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                mouseData: 120,
                dwFlags: MOUSEEVENTF_WHEEL,
                ..Default::default()
            },
        },
    };
    // SAFETY: initialized synthetic mouse input goes to our foreground window under the cursor.
    assert_eq!(
        unsafe { SendInput(&[wheel], std::mem::size_of::<INPUT>() as i32) },
        1
    );
    pump_until(|| first.observed.borrow().wheels == 1);
    let event = hook
        .events
        .try_recv()
        .expect("real low-level hook must receive the wheel event");
    assert_eq!(event.delta, 120);
    assert_eq!(event.point.x, 120);
    assert_eq!(event.point.y, 20);
    for direction in [TabDirection::Next, TabDirection::Previous] {
        assert_eq!(
            execute(
                Command::SwitchTab {
                    target_window: second_id,
                    direction,
                    restore_focus: None
                },
                &mut windows
            ),
            Outcome::Completed
        );
        let expected_count = if direction == TabDirection::Next {
            1
        } else {
            2
        };
        pump_until(|| second.observed.borrow().tabs.len() == expected_count);
    }
    assert_eq!(
        second.observed.borrow().tabs,
        [TabDirection::Next, TabDirection::Previous]
    );
    assert_eq!(windows.foreground(), Some(second_id));
    assert_eq!(
        execute(
            Command::RestoreFocus {
                window: first_id,
                generation: 1
            },
            &mut windows
        ),
        Outcome::Completed
    );
    assert_eq!(windows.foreground(), Some(first_id));
    drop(hook); // must unhook and join without wheel traffic or a polling loop
    drop(second);
    assert!(!windows.is_valid(second_id));
    drop(first);
    drop(desktop);
}
