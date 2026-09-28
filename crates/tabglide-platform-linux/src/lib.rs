//! Planned only. X11 can use XInput2, EWMH focus/window metadata and XTest.
//! Wayland global input and injection are compositor/portal-dependent; unrestricted
//! enumeration, focus and synthetic input cannot be assumed. No fake implementation.
#![forbid(unsafe_code)]

/// Reserved namespace for future X11 implementation; no backend can be instantiated.
pub enum X11Backend {}
/// Reserved namespace for future Wayland implementation.
pub enum WaylandBackend {}
pub const IMPLEMENTED: bool = false;
