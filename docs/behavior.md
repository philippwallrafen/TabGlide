# AHK behavior and Rust migration

Source of truth: `legacy/ahk/TabGlide.ahk`, preserved from `src/TabGlide.ahk`.

| Trigger / state | Legacy behavior | Rust behavior |
| --- | --- | --- |
| Wheel up / down | Previous / next using Ctrl+Shift+Tab / Ctrl+Tab | Preserved |
| Original wheel event | `~$` passes it through | Always forwarded through `CallNextHookEx` |
| Application | Case-insensitive executable basename allowlist | Preserved, including all nine defaults |
| Activation area | Monitor-relative Y <= 50, inclusive | Preserved for valid coordinates; reject negative coordinates and failed monitor lookups |
| Unfocused target | Activate before switching | Request activation, allow a bounded foreground-confirmation delay, then send keys only after the target is confirmed |
| Focus return enabled | Remember first original window in a scrolling burst | Preserved |
| More eligible scrolls | Extend return deadline, including scrolls over an already focused target | Preserved; generation rejects stale timers |
| Unsupported app / outside region | Ignore without extending deadline | Preserved |
| Original window is Explorer | Skip return, even for ordinary Explorer windows | Preserved; exception applies to original window, not hovered target |
| Original window disappears | Ignore focus attempt | Preserved, also validate process/thread identity |
| Focus return disabled | Activate target and leave focus there | Preserved |
| Single instance | Force-replace previous instance | Keep existing instance; second launch exits successfully (intentional safer change) |
| Configuration | Compiled AHK creates INI next to EXE and opens it | TOML in roaming app data; Settings opens it in Notepad; explicit Reload Config |
| Diagnostics | Optional file logging and F12 GUI | Optional local-app-data logs and tray diagnostics; no global F12 binding |

New safety behavior: failed or still-unconfirmed activation sends no keys; an accepted-but-delayed activation retains the pending focus-return state so the original window is not forgotten; disabled/reloaded configuration cancels pending focus return; stale queued wheel events are discarded. Modified wheel gestures are skipped before focus mutation, with an additional modifier check at injection. No automatic elevation. See architecture and README for limitations and validation status.

## Dependency plan (before implementation)

- `tabglide-core`: standard library only; owns domain decisions and explicit state/time.
- `tabglide-config`: core, serde, toml, thiserror; portable parsing/defaults/file operations.
- `tabglide-platform-windows`: core/config, windows, tracing, tracing-subscriber; all Win32, unsafe, lifecycle and native shell UI.
- `tabglide-windows`: target-specific dependency on Windows platform only; safe entry point and embedded DPI manifest.
- macOS and Linux platform crates: standard-library-only explicit stubs; no dependency from the Windows application.

No runtime, shared platform trait, polling, or framework. Exact resolved versions are recorded in Cargo.lock.
