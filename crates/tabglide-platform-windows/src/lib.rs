//! Windows input, execution and native shell integration.
#[cfg(any(windows, test))]
mod executor;

#[cfg(windows)]
mod native;
#[cfg(windows)]
pub use native::run;
