//! Windows adapter and native shell integration.
#[cfg(any(windows, test))]
mod adapter;

#[cfg(windows)]
mod native;
#[cfg(windows)]
pub use native::run;
