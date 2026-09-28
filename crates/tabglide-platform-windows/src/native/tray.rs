use super::{Result, wide};
use std::path::Path;
use windows::{
    Win32::{
        Foundation::*,
        System::LibraryLoader::GetModuleHandleW,
        UI::{Shell::*, WindowsAndMessaging::*},
    },
    core::{PCWSTR, w},
};

const TRAY_CALLBACK: u32 = WM_APP + 1;
const SHOW_MENU: u32 = WM_APP + 2;
const RESTORE_TRAY: u32 = WM_APP + 3;
const TOGGLE: u32 = 1;
const SETTINGS: u32 = 2;
const RELOAD: u32 = 3;
const DIAGNOSTICS: u32 = 4;
const EXIT: u32 = 5;

pub(super) enum Action {
    Toggle,
    Settings,
    Reload,
    Diagnostics,
    Exit,
    Dismissed,
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // SAFETY: callback uses only value arguments; it never accesses borrowed application state.
    unsafe {
        let taskbar_created = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as u32;
        if taskbar_created != 0 && message == taskbar_created {
            let _ = PostMessageW(Some(hwnd), RESTORE_TRAY, WPARAM(0), LPARAM(0));
            return LRESULT(0);
        }
        match message {
            TRAY_CALLBACK
                if matches!(
                    lparam.0 as u32,
                    WM_RBUTTONUP | WM_LBUTTONUP | WM_CONTEXTMENU
                ) =>
            {
                let _ = PostMessageW(Some(hwnd), SHOW_MENU, WPARAM(0), LPARAM(0));
                LRESULT(0)
            }
            WM_QUERYENDSESSION => LRESULT(1),
            WM_ENDSESSION if wparam.0 != 0 => {
                PostQuitMessage(0);
                LRESULT(0)
            }
            WM_CLOSE | WM_DESTROY => {
                PostQuitMessage(0);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, message, wparam, lparam),
        }
    }
}

pub(super) struct Tray {
    hwnd: HWND,
}
impl Tray {
    pub fn new() -> Result<Self> {
        // SAFETY: class and title strings are static, callback remains loaded for window lifetime.
        unsafe {
            let module = GetModuleHandleW(PCWSTR::null())?;
            let class = WNDCLASSW {
                lpfnWndProc: Some(window_proc),
                hInstance: module.into(),
                lpszClassName: w!("TabGlide.Rust.Tray"),
                ..Default::default()
            };
            if RegisterClassW(&class) == 0 {
                return Err(windows::core::Error::from_thread().into());
            }
            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                class.lpszClassName,
                w!("TabGlide"),
                WINDOW_STYLE(0),
                0,
                0,
                0,
                0,
                None,
                None,
                Some(module.into()),
                None,
            )?;
            SetWindowLongPtrW(
                hwnd,
                GWLP_USERDATA,
                RegisterWindowMessageW(w!("TaskbarCreated")) as isize,
            );
            let tray = Self { hwnd };
            tray.add(true)?;
            Ok(tray)
        }
    }
    fn data(&self, enabled: bool) -> windows::core::Result<NOTIFYICONDATAW> {
        let mut data = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: self.hwnd,
            uID: 1,
            uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
            uCallbackMessage: TRAY_CALLBACK,
            ..Default::default()
        };
        // SAFETY: shared stock icons are retained by Windows and must not be destroyed.
        data.hIcon = unsafe {
            LoadIconW(
                None,
                if enabled {
                    IDI_APPLICATION
                } else {
                    IDI_WARNING
                },
            )?
        };
        let tip = if enabled {
            "TabGlide — Enabled"
        } else {
            "TabGlide — Disabled"
        };
        for (destination, character) in data.szTip.iter_mut().zip(tip.encode_utf16()) {
            *destination = character;
        }
        Ok(data)
    }
    fn add(&self, enabled: bool) -> Result<()> {
        let data = self.data(enabled)?;
        // SAFETY: initialized NOTIFYICONDATAW refers to this live window and shared icon.
        if !unsafe { Shell_NotifyIconW(NIM_ADD, &data) }.as_bool() {
            return Err("Could not create notification-area icon".into());
        }
        Ok(())
    }
    pub fn update(&self, enabled: bool) -> Result<()> {
        let data = self.data(enabled)?;
        // SAFETY: the registered tray icon is owned by this window.
        if !unsafe { Shell_NotifyIconW(NIM_MODIFY, &data) }.as_bool() {
            return Err("Could not update notification-area icon".into());
        }
        Ok(())
    }
    pub fn handle_message(&self, message: &MSG, enabled: bool) -> Result<Option<Action>> {
        if message.message == RESTORE_TRAY {
            self.add(enabled)?;
        }
        if message.message != SHOW_MENU {
            return Ok(None);
        }
        // SAFETY: popup menu and owner live throughout TrackPopupMenu; no app references in WndProc.
        unsafe {
            let menu = Menu(CreatePopupMenu()?);
            AppendMenuW(
                menu.0,
                MF_STRING,
                TOGGLE as usize,
                if enabled { w!("Disable") } else { w!("Enable") },
            )?;
            AppendMenuW(menu.0, MF_STRING, SETTINGS as usize, w!("Settings…"))?;
            AppendMenuW(menu.0, MF_STRING, RELOAD as usize, w!("Reload Config"))?;
            AppendMenuW(menu.0, MF_STRING, DIAGNOSTICS as usize, w!("Diagnostics…"))?;
            AppendMenuW(menu.0, MF_SEPARATOR, 0, PCWSTR::null())?;
            AppendMenuW(menu.0, MF_STRING, EXIT as usize, w!("Exit"))?;
            let mut point = POINT::default();
            GetCursorPos(&mut point)?;
            let _ = SetForegroundWindow(self.hwnd);
            let selected = TrackPopupMenu(
                menu.0,
                TPM_RETURNCMD | TPM_RIGHTBUTTON,
                point.x,
                point.y,
                None,
                self.hwnd,
                None,
            )
            .0 as u32;
            let _ = PostMessageW(Some(self.hwnd), WM_NULL, WPARAM(0), LPARAM(0));
            Ok(Some(match selected {
                TOGGLE => Action::Toggle,
                SETTINGS => Action::Settings,
                RELOAD => Action::Reload,
                DIAGNOSTICS => Action::Diagnostics,
                EXIT => Action::Exit,
                _ => Action::Dismissed,
            }))
        }
    }
}
impl Drop for Tray {
    fn drop(&mut self) {
        let data = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: self.hwnd,
            uID: 1,
            ..Default::default()
        };
        // SAFETY: delete only our icon and window; called on the creating thread.
        unsafe {
            let _ = Shell_NotifyIconW(NIM_DELETE, &data);
            let _ = DestroyWindow(self.hwnd);
        }
    }
}
struct Menu(HMENU);
impl Drop for Menu {
    fn drop(&mut self) {
        // SAFETY: exclusively owned menu; TrackPopupMenu has returned.
        unsafe {
            let _ = DestroyMenu(self.0);
        }
    }
}

pub(super) fn show_error(message: &str) {
    let text = wide(message.as_ref());
    // SAFETY: NUL-terminated text buffers live through synchronous dialog call.
    unsafe {
        MessageBoxW(
            None,
            PCWSTR(text.as_ptr()),
            w!("TabGlide"),
            MB_OK | MB_ICONERROR,
        );
    }
}
pub(super) fn show_diagnostics(
    enabled: bool,
    logging: bool,
    dropped: u64,
    config: &Path,
    logs: &Path,
) {
    let text = wide(format!("Enabled: {enabled}\nLogging: {logging}\nDropped wheel events: {dropped}\n\nConfig: {}\nLogs: {}\n\nHigher-privilege applications may reject input (UIPI).", config.display(), logs.display()).as_ref());
    // SAFETY: NUL-terminated buffers remain valid for the synchronous dialog.
    unsafe {
        MessageBoxW(
            None,
            PCWSTR(text.as_ptr()),
            w!("TabGlide Diagnostics"),
            MB_OK | MB_ICONINFORMATION,
        );
    }
}
pub(super) fn open_settings(path: &Path) -> Result<()> {
    // Notepad is an explicit native editor; a file association for TOML is not required.
    let editor = std::env::var_os("SystemRoot")
        .map(std::path::PathBuf::from)
        .ok_or("SystemRoot is missing")?
        .join("System32")
        .join("notepad.exe");
    std::process::Command::new(editor).arg(path).spawn()?;
    Ok(())
}

pub(super) fn request_exit() -> Result<()> {
    use windows::Win32::System::Threading::{
        OpenProcess, PROCESS_SYNCHRONIZE, WaitForSingleObject,
    };
    // SAFETY: find only our unique window class, obtain its owning process and wait on a
    // retained kernel handle. No image-wide termination or forced shutdown of other apps.
    unsafe {
        let Ok(hwnd) = FindWindowW(w!("TabGlide.Rust.Tray"), w!("TabGlide")) else {
            return Ok(());
        };
        let mut pid = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        let process = super::OwnedHandle(OpenProcess(PROCESS_SYNCHRONIZE, false, pid)?);
        PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0))?;
        if WaitForSingleObject(process.0, 5000) != WAIT_OBJECT_0 {
            return Err("TabGlide did not exit within five seconds".into());
        }
    }
    Ok(())
}
