use crate::adapter::{Outcome, execute};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
use tabglide_core::{AppState, CoreConfig, Event, WheelDirection, process_event};
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

struct InstanceGuard {
    mutex: OwnedHandle,
    replace: OwnedHandle,
}
impl InstanceGuard {
    fn acquire() -> Result<Self> {
        // SAFETY: named kernel objects use default security. The first process owns the mutex;
        // later launches signal the replacement event and wait until ownership transfers.
        unsafe {
            let mutex = OwnedHandle(CreateMutexW(
                None,
                true,
                w!("Local\\TabGlide.Rust.Instance"),
            )?);
            let already_running = GetLastError() == ERROR_ALREADY_EXISTS;
            let replace = OwnedHandle(CreateEventW(
                None,
                false,
                false,
                w!("Local\\TabGlide.Rust.Replace"),
            )?);

            if already_running {
                SetEvent(replace.0)?;
                let wait = WaitForSingleObject(mutex.0, 5000);
                if wait != WAIT_OBJECT_0 && wait != WAIT_ABANDONED {
                    return Err(
                        "Previous TabGlide instance did not exit within five seconds".into(),
                    );
                }
            }

            // A replacement signal can remain pending if the previous process was already
            // shutting down. Never let the new instance consume its own stale signal.
            ResetEvent(replace.0)?;
            Ok(Self { mutex, replace })
        }
    }

    fn replacement_handle(&self) -> HANDLE {
        self.replace.0
    }
}
impl Drop for InstanceGuard {
    fn drop(&mut self) {
        // SAFETY: acquire() returns only after this thread owns the named mutex.
        unsafe {
            let _ = ReleaseMutex(self.mutex.0);
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

fn drive_core_event(
    initial_event: Event,
    initial_now: Instant,
    core_config: &CoreConfig,
    state: &mut AppState,
    windows: &mut window::NativeWindows,
    wheel_timestamp: Option<u32>,
) {
    let mut event = Some(initial_event);
    let mut now = initial_now;
    while let Some(current_event) = event.take() {
        let Some(effect) = process_event(current_event, core_config, state, now) else {
            break;
        };
        let execution = execute(effect, windows);
        if execution.outcome != Outcome::Completed {
            tracing::warn!(
                ?execution.outcome,
                ?wheel_timestamp,
                "Platform adapter effect did not complete"
            );
        }
        event = execution.feedback;
        now = Instant::now();
    }
}

fn run_application() -> Result<()> {
    let instance = InstanceGuard::acquire()?;
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
            drive_core_event(
                Event::Wheel { direction, context },
                now,
                &core_config,
                &mut state,
                &mut windows,
                Some(event.timestamp),
            );
        }
        let now = Instant::now();
        if let Some(pending) = state.pending_refocus()
            && now >= pending.deadline
        {
            drive_core_event(
                Event::RefocusTimerElapsed {
                    generation: pending.generation,
                },
                now,
                &core_config,
                &mut state,
                &mut windows,
                None,
            );
        }
        if hook.finished() {
            return Err("Mouse hook thread stopped unexpectedly".into());
        }
        // If a bounded batch left work, re-signal. The auto-reset event eliminates reset/drain races.
        // Always re-signal after a full batch via the receiver-independent producer counter.
        hook.wake_if_pending();
        let timeout = state.pending_refocus().map_or(INFINITE, |pending| {
            pending
                .deadline
                .saturating_duration_since(Instant::now())
                .as_millis()
                .saturating_add(1)
                .min(u32::MAX as u128 - 1) as u32
        });
        // SAFETY: replacement and wake handles remain live until shutdown; blocks until
        // replacement, input, UI, or the refocus deadline.
        let result = unsafe {
            MsgWaitForMultipleObjectsEx(
                Some(&[instance.replacement_handle(), hook.wake_handle()]),
                timeout,
                QS_ALLINPUT,
                MWMO_INPUTAVAILABLE,
            )
        };
        if result == WAIT_FAILED {
            return Err(windows::core::Error::from_thread().into());
        }
        if result == WAIT_OBJECT_0 {
            tracing::info!("Replacement instance requested shutdown");
            break 'application;
        }
    }
    tracing::info!("TabGlide shutting down");
    drop(hook);
    drop(tray);
    drop(instance);
    Ok(())
}
