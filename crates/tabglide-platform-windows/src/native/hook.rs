use super::{OwnedHandle, Result};
use std::{
    cell::RefCell,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
        mpsc::{self, Receiver, SyncSender},
    },
    thread::{self, JoinHandle},
    time::Instant,
};
use windows::{
    Win32::{
        Foundation::*,
        System::{LibraryLoader::GetModuleHandleW, Threading::*},
        UI::WindowsAndMessaging::*,
    },
    core::PCWSTR,
};

#[derive(Clone, Copy)]
pub(super) struct WheelEvent {
    pub point: POINT,
    pub delta: i16,
    pub timestamp: u32,
    pub received_at: Instant,
}

struct Signal(OwnedHandle);
// SAFETY: event handles support concurrent SetEvent/wait calls. Arc keeps the handle alive.
unsafe impl Send for Signal {}
// SAFETY: kernel synchronizes event state, with no user-space mutable fields.
unsafe impl Sync for Signal {}
impl Signal {
    fn new() -> windows::core::Result<Self> {
        // SAFETY: unnamed auto-reset event with default security, exclusively owned here.
        unsafe {
            Ok(Self(OwnedHandle(CreateEventW(
                None,
                false,
                false,
                PCWSTR::null(),
            )?)))
        }
    }
    fn set(&self) {
        // SAFETY: self owns a live event handle.
        unsafe {
            let _ = SetEvent(self.0.0);
        }
    }
}

struct CallbackBridge {
    sender: SyncSender<WheelEvent>,
    wake: Arc<Signal>,
    pending: Arc<AtomicUsize>,
    dropped: Arc<AtomicU64>,
}

struct NotifyTermination {
    stopped: Arc<AtomicBool>,
    wake: Arc<Signal>,
}
impl Drop for NotifyTermination {
    fn drop(&mut self) {
        // Publish termination before the wake, rather than racing JoinHandle::is_finished.
        self.stopped.store(true, Ordering::Release);
        self.wake.set();
    }
}
thread_local! {
    // Only the dedicated hook thread accesses this callback bridge, never application state.
    static BRIDGE: RefCell<Option<CallbackBridge>> = const { RefCell::new(None) };
}

unsafe extern "system" fn mouse_callback(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 && wparam.0 == WM_MOUSEWHEEL as usize {
        // SAFETY: Windows provides MSLLHOOKSTRUCT for nonnegative WH_MOUSE_LL callbacks.
        let input = unsafe { &*(lparam.0 as *const MSLLHOOKSTRUCT) };
        let delta = (input.mouseData >> 16) as i16;
        if delta != 0 {
            let _ = BRIDGE.try_with(|slot| {
                if let Ok(bridge) = slot.try_borrow()
                    && let Some(bridge) = bridge.as_ref()
                {
                    let event = WheelEvent {
                        point: input.pt,
                        delta,
                        timestamp: input.time,
                        received_at: Instant::now(),
                    };
                    // Increment before publishing; consumer cannot underflow the count.
                    bridge.pending.fetch_add(1, Ordering::Release);
                    if bridge.sender.try_send(event).is_err() {
                        bridge.pending.fetch_sub(1, Ordering::AcqRel);
                        bridge.dropped.fetch_add(1, Ordering::Relaxed);
                    } else {
                        bridge.wake.set();
                    }
                }
            });
        }
    }
    // SAFETY: forward unchanged parameters on EVERY path, including saturation and errors.
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

pub(super) struct WheelReceiver {
    receiver: Receiver<WheelEvent>,
    pending: Arc<AtomicUsize>,
}
impl WheelReceiver {
    pub fn try_recv(&self) -> std::result::Result<WheelEvent, mpsc::TryRecvError> {
        let event = self.receiver.try_recv()?;
        self.pending.fetch_sub(1, Ordering::AcqRel);
        Ok(event)
    }
}

pub(super) struct MouseHook {
    pub events: WheelReceiver,
    wake: Arc<Signal>,
    stop: Arc<Signal>,
    stopped: Arc<AtomicBool>,
    dropped: Arc<AtomicU64>,
    thread: Option<JoinHandle<()>>,
}

impl MouseHook {
    pub fn start() -> Result<Self> {
        let wake = Arc::new(Signal::new()?);
        let stop = Arc::new(Signal::new()?);
        let stopped = Arc::new(AtomicBool::new(false));
        let pending = Arc::new(AtomicUsize::new(0));
        let dropped = Arc::new(AtomicU64::new(0));
        let (sender, receiver) = mpsc::sync_channel(128);
        let (ready_sender, ready_receiver) = mpsc::sync_channel(1);
        let bridge = CallbackBridge {
            sender,
            wake: wake.clone(),
            pending: pending.clone(),
            dropped: dropped.clone(),
        };
        let thread_stop = stop.clone();
        let termination = NotifyTermination {
            stopped: stopped.clone(),
            wake: wake.clone(),
        };
        let thread = thread::Builder::new()
            .name("tabglide-mouse-hook".into())
            .spawn(move || {
                let _termination = termination;
                BRIDGE.with(|slot| *slot.borrow_mut() = Some(bridge));
                // SAFETY: module and callback remain loaded until unhook; thread owns the hook.
                let installed = unsafe {
                    GetModuleHandleW(PCWSTR::null()).and_then(|module| {
                        SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_callback), Some(module.into()), 0)
                    })
                };
                match installed {
                    Ok(handle) => {
                        let _ = ready_sender.send(Ok(()));
                        'hook: loop {
                            // SAFETY: live stop handle, this thread services the callback's message queue.
                            let result = unsafe {
                                MsgWaitForMultipleObjectsEx(
                                    Some(&[thread_stop.0.0]),
                                    INFINITE,
                                    QS_ALLINPUT,
                                    MWMO_INPUTAVAILABLE,
                                )
                            };
                            if result == WAIT_OBJECT_0 || result == WAIT_FAILED {
                                break;
                            }
                            for _ in 0..64 {
                                let mut message = MSG::default();
                                // SAFETY: valid buffer, messages belong to this thread.
                                unsafe {
                                    if !PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool()
                                    {
                                        break;
                                    }
                                    if message.message == WM_QUIT {
                                        break 'hook;
                                    }
                                    let _ = TranslateMessage(&message);
                                    DispatchMessageW(&message);
                                }
                            }
                        }
                        // SAFETY: handle belongs to this thread and has not been unhooked.
                        unsafe {
                            let _ = UnhookWindowsHookEx(handle);
                        }
                    }
                    Err(error) => {
                        let _ = ready_sender.send(Err(error));
                    }
                }
                BRIDGE.with(|slot| *slot.borrow_mut() = None);
            })?;
        match ready_receiver.recv() {
            Ok(Ok(())) => Ok(Self {
                events: WheelReceiver { receiver, pending },
                wake,
                stop,
                stopped,
                dropped,
                thread: Some(thread),
            }),
            Ok(Err(error)) => {
                let _ = thread.join();
                Err(error.into())
            }
            Err(error) => {
                let _ = thread.join();
                Err(error.into())
            }
        }
    }
    pub fn wake_handle(&self) -> HANDLE {
        self.wake.0.0
    }
    pub fn wake_if_pending(&self) {
        if self.events.pending.load(Ordering::Acquire) > 0 {
            self.wake.set();
        }
    }
    pub fn dropped(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }
    pub fn finished(&self) -> bool {
        self.stopped.load(Ordering::Acquire)
    }
}
impl Drop for MouseHook {
    fn drop(&mut self) {
        self.stop.set();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
