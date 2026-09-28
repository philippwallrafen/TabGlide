# TabGlide

TabGlide switches tabs when you scroll at the top edge of a monitor. The shared behavior core is written in Rust. The current Windows host is a thin Rust/Win32 adapter around that core and uses native Windows input and tray APIs; the adapter boundary is designed so Windows UI can move to C# without moving product policy out of Rust. AutoHotkey is no longer required.

| Platform | Status |
| --- | --- |
| Windows 10/11 x64 | Supported implementation; see [validation status](docs/validation.md) for tested scope |
| macOS | Planned / stub |
| Linux X11 | Planned / stub |
| Linux Wayland | Planned / stub; compositor restrictions apply |

## Use

Hover over an allowed application's window within the first 50 physical pixels from the top of its monitor (including pixel 50). Scroll up for the previous tab or down for the next tab. This is a monitor-edge region, not tab-bar detection. The original wheel event still reaches Windows/the application.

TabGlide activates an unfocused target before sending the shortcut. By default it returns focus to the original window 700 ms after the last eligible scroll. Further eligible scrolls preserve the first original window and restart the delay. The Rust version returns to any still-valid original window, including Windows Explorer; Explorer itself also remains an allowed tab-switching target on systems with Explorer tabs.

Use the notification-area icon for **Enable/Disable**, **Settings**, **Reload Config**, **Diagnostics**, or **Exit**. Settings opens the TOML file in Notepad; save it, then choose Reload Config. A bad reload leaves the last valid configuration active and displays the error. Disable and successful reload cancel pending focus return. Another launch leaves the existing instance running. `TabGlide.exe --exit` requests graceful shutdown and waits up to five seconds.

## Installation and AHK migration

Build locally using the instructions below, or use a maintainer-published Windows x64 installer when available. The installer runs per user without an administrator prompt:

| Item | Location |
| --- | --- |
| Executable | `%LOCALAPPDATA%\Programs\TabGlide\TabGlide.exe` |
| Configuration | `%APPDATA%\TabGlide\config.toml` |
| Optional logs | `%LOCALAPPDATA%\TabGlide\logs\` |

Autostart is a per-user Startup-folder shortcut and can be selected during installation. Updates and the Rust uninstaller preserve configuration and logs. The installer never recursively deletes the installation or data directories.

**Before migrating from AHK:** exit the old script/executable, disable its startup entry, and copy its INI and any other files you need to a safe backup outside `%APPDATA%\TabGlide`. Uninstall the old version **before first running Rust TabGlide**: the legacy uninstaller recursively deletes its old `%APPDATA%\TabGlide` folder, which is now the Rust configuration directory. An elevated legacy installation may require its original elevated uninstaller. Do not run both implementations together. Rust does not automatically import INI; transfer customized allowlist, height, focus return, delay and logging settings to TOML. Both AHK sources remain in `legacy/ahk/`.

Silent commands (use the actual installer version):

```powershell
.\TabGlide-1.1.0-windows-x64-setup.exe /VERYSILENT /SUPPRESSMSGBOXES /NORESTART /SP-
& "$env:LOCALAPPDATA\Programs\TabGlide\unins000.exe" /VERYSILENT /SUPPRESSMSGBOXES /NORESTART
```

Silent install does not launch the app immediately. Add `/TASKS="autostart"` or `/TASKS=""` to explicitly select startup behavior. See [WinGet submission notes](winget-manifest/README.md); there is no placeholder manifest claiming a published package.

## Configuration

Defaults are created on first launch outside the install directory. Existing files are never overwritten. All fields are optional; unknown keys and invalid values are rejected.

```toml
activation_region_height = 50
focus_unfocused_window = true
focus_return = true
focus_return_delay_ms = 700

[windows]
allowed_applications = [
  "brave.exe", "chrome.exe", "chromium.exe", "explorer.exe",
  "firefox.exe", "msedge.exe", "opera.exe", "opera_gx.exe",
  "WindowsTerminal.exe"
]

[diagnostics]
logging = false
```

Application names are case-insensitive executable basenames. An empty list disables all targets. Height accepts 0–10000 pixels; delay accepts 0–60000 ms. Set `focus_unfocused_window = false` to switch only the currently focused window. Logs rotate at about 2 MiB, retaining the current and previous file. The tray Diagnostics dialog shows paths, enable/logging state and queue-overflow count; no global F12 shortcut is registered.

## Limitations and intentional changes

- Windows may reject foreground activation. No shortcut is sent unless the target is validated as foreground. Foreground ownership can still change between the last check and `SendInput`; Windows has no atomic target-specific `SendInput` operation.
- TabGlide runs as the user, without automatic elevation. UIPI can prevent sending input to administrator applications. Do not expect elevated targets to work.
- Ctrl/Shift/Alt/Windows-modified wheel gestures are skipped to preserve physical key state; the original wheel still passes through. AHK's implicit modifier rewriting is not reproduced.
- Queue overflow and wheel events older than 250 ms are dropped rather than replayed late. Native menus/dialogs pause the consumer; events captured during them are discarded on return. The hook continues forwarding all original wheel input.
- Window identity includes the native handle plus process/thread IDs. Reuse of all three identifiers remains a rare limitation; destroyed windows and ordinary handle reuse are rejected.
- Single-instance behavior keeps the existing process instead of AHK's forced replacement. Config is TOML, diagnostics are tray/log based, and invalid monitor lookups are ignored. See the complete [behavior matrix](docs/behavior.md).

## Build and validation

Install stable Rust and the MSVC C++ build tools/Windows SDK. From the repository root:

```powershell
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo build --release
cargo tree
```

Run `target\release\TabGlide.exe`. Windows builds use the MSVC toolchain to embed the Per-Monitor-V2, `asInvoker` manifest. Compile `installer\TabGlideExe.iss` with Inno Setup 6.3 or later after building. The default installer version remains the user's `1.1`; release CI passes `/DAppVersion=X.Y.Z` from a validated tag. No built executables are tracked.

CI runs format, lint, tests and release builds on Windows/macOS/Linux, with an Inno compile on Windows. Only the Windows binary is implemented. A manually pushed `vX.Y.Z` tag triggers the Windows installer, SHA256 and GitHub release-artifact workflow. This migration does not push tags or publish releases.

The opt-in interactive smoke test briefly creates its own windows and restores cursor/focus:

```powershell
cargo test -p tabglide-platform-windows real_hook_passthrough_context_keys_focus_and_shutdown -- --ignored --nocapture --test-threads=1
```

Run it on an unlocked desktop with no keys held. Ordinary `cargo test` never injects desktop input. See [validation evidence and manual checks](docs/validation.md) and [architecture](docs/architecture.md).

Licensed under [GPL-3.0](LICENSE).
