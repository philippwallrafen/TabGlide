use crate::executor::{Outcome, execute};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
use tabglide_core::{AppState, InputEvent, RefocusState, WheelDirection, process_event};
use windows::{
    Win32::{Foundation::*, System::Threading::*, UI::WindowsAndMessaging::*},
    core::w,
};

mod hook;
mod logging;
#[cfg(test)]
mod smoke_tests;
mod tray;
mod window;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub(super) struct OwnedHandle(pub HANDLE);
impl Drop for OwnedHandle {
    fn drop(&mut self) {
        // SAFETY: each OwnedHandle exclusively owns a successfully created kernel handle.
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

pub(super) fn wide(text: &std::ffi::OsStr) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    text.encode_wide().chain(Some(0)).collect()
}

pub fn run() -> Result<()> {
    if std::env::args_os()
        .skip(1)
        .any(|argument| argument == "--exit")
    {
        // Automation must receive an exit code without a blocking error dialog.
        return tray::request_exit();
    }
    let result = run_application();
    if let Err(error) = &result {
        tray::show_error(&error.to_string());
    }
    result
}

fn user_path(variable: &str) -> Result<PathBuf> {
    let path = std::env::var_os(variable)
        .map(PathBuf::from)
        .ok_or_else(|| format!("{variable} is not set"))?;
    if !path.is_absolute() {
        return Err(format!("{variable} must be an absolute path").into());
    }
    Ok(path.join("TabGlide"))
}

fn run_application() -> Result<()> {
    // SAFETY: constant terminated name, no custom security attributes; retained until shutdown.
    let instance = unsafe {
        OwnedHandle(CreateMutexW(
            None,
            false,
            w!("Local\\TabGlide.Rust.Instance"),
        )?)
    };
    // SAFETY: inspect last error immediately after CreateMutexW, before any other Win32 call.
    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        return Ok(());
    }
    let config_path = user_path("APPDATA")?.join("config.toml");
    let logs_path = user_path("LOCALAPPDATA")?.join("logs");
    let mut config = tabglide_config::load_or_create(&config_path)?;
    let diagnostics = logging::Diagnostics::new(logs_path.clone(), config.diagnostics.logging)?;
    let mut core_config = config.core_config();
    let tray = tray::Tray::new()?;
    let hook = hook::MouseHook::start()?;
    let mut state = AppState::default();
    let mut enabled = true;
    let mut windows = window::NativeWindows;
    let mut accept_events_since = Instant::now();
    tracing::info!("TabGlide started");

    'application: loop {
        // Bound both batches so a wheel burst cannot starve commands or the return deadline.
        for _ in 0..64 {
            let mut message = MSG::default();
            // SAFETY: valid output buffer; all application windows belong to this thread.
            if !unsafe { PeekMessageW(&mut message, None, 0, 0, PM_REMOVE) }.as_bool() {
                break;
            }
            if message.message == WM_QUIT {
                break 'application;
            }
            if let Some(action) = tray.handle_message(&message, enabled)? {
                // Native menus may have blocked this consumer. Never replay their wheel backlog.
                match action {
                    tray::Action::Toggle => {
                        enabled = !enabled;
                        state.cancel_refocus();
                        tray.update(enabled)?;
                    }
                    tray::Action::Settings => tray::open_settings(&config_path)?,
                    tray::Action::Reload => match tabglide_config::load(&config_path) {
                        Ok(reloaded) => {
                            match diagnostics.set_enabled(reloaded.diagnostics.logging) {
                                Ok(()) => {
                                    config = reloaded;
                                    core_config = config.core_config();
                                    state.cancel_refocus();
                                    tracing::info!("Configuration reloaded");
                                }
                                Err(error) => tray::show_error(&error.to_string()),
                            }
                        }
                        Err(error) => tray::show_error(&error.to_string()),
                    },
                    tray::Action::Diagnostics => tray::show_diagnostics(
                        enabled,
                        config.diagnostics.logging,
                        hook.dropped(),
                        &config_path,
                        &logs_path,
                    ),
                    tray::Action::Exit => break 'application,
                    tray::Action::Dismissed => {}
                }
                accept_events_since = Instant::now();
            }
            // SAFETY: message came from this thread's queue; callback does not borrow app state.
            unsafe {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        for _ in 0..64 {
            let Ok(event) = hook.events.try_recv() else {
                break;
            };
            let now = Instant::now();
            if !enabled
                || event.received_at < accept_events_since
                || now.duration_since(event.received_at) > Duration::from_millis(250)
                || window::modifiers_pressed()
            {
                continue;
            }
            let Some(context) = window::context_at(event.point) else {
                continue;
            };
            let direction = if event.delta > 0 {
                WheelDirection::Up
            } else {
                WheelDirection::Down
            };
            let previous_state = state;
            if let Some(command) = process_event(
                InputEvent::Wheel { direction },
                Some(&context),
                &core_config,
                &mut state,
                now,
            ) {
                let outcome = execute(command, &mut windows);
                if matches!(outcome, Outcome::InvalidWindow | Outcome::FocusDenied) {
                    state = previous_state;
                }
                if outcome != Outcome::Completed {
                    tracing::warn!(
                        ?outcome,
                        wheel_timestamp = event.timestamp,
                        "Tab switch failed"
                    );
                }
            }
        }
        let now = Instant::now();
        if let RefocusState::Pending(pending) = state.refocus
            && let Some(command) = process_event(
                InputEvent::RefocusTimerElapsed {
                    generation: pending.generation,
                },
                None,
                &core_config,
                &mut state,
                now,
            )
        {
            let outcome = execute(command, &mut windows);
            if outcome != Outcome::Completed {
                tracing::debug!(?outcome, "Focus return skipped or denied");
            }
        }
        if hook.finished() {
            return Err("Mouse hook thread stopped unexpectedly".into());
        }
        // If a bounded batch left work, re-signal. The auto-reset event eliminates reset/drain races.
        // Always re-signal after a full batch via the receiver-independent producer counter.
        hook.wake_if_pending();
        let timeout = match state.refocus {
            RefocusState::Idle => INFINITE,
            RefocusState::Pending(pending) => pending
                .deadline
                .saturating_duration_since(Instant::now())
                .as_millis()
                .saturating_add(1)
                .min(u32::MAX as u128 - 1) as u32,
        };
        // SAFETY: wake event remains alive until the hook has joined; blocks until input/UI/deadline.
        let result = unsafe {
            MsgWaitForMultipleObjectsEx(
                Some(&[hook.wake_handle()]),
                timeout,
                QS_ALLINPUT,
                MWMO_INPUTAVAILABLE,
            )
        };
        if result == WAIT_FAILED {
            return Err(windows::core::Error::from_thread().into());
        }
    }
    tracing::info!("TabGlide shutting down");
    drop(hook);
    drop(tray);
    drop(instance);
    Ok(())
}
