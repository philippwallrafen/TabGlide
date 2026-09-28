# Migration validation

This file records evidence, not a claim that every third-party application was exercised.

## Automated checks

Final local checks on Windows x64 (2026-09-28), after all review fixes:

| Check | Result |
| --- | --- |
| `cargo fmt --check` | Passed |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | Passed |
| `cargo test --workspace` | 18 passed; 1 interactive test intentionally ignored |
| `cargo build --release` | Passed; `target/release/TabGlide.exe` |
| `cargo tree` and Windows application target graph | Inspected; core has no dependencies, no macOS/Linux crate in Windows app |
| `cargo check --locked --workspace --all-targets --target x86_64-unknown-linux-gnu` | Passed (cross-check, not a Linux runtime test) |
| `cargo check --locked --workspace --all-targets --target aarch64-apple-darwin` | Passed (cross-check, not a macOS runtime test) |
| Inno Setup 6.7.3 compile with `/DAppVersion=1.1.0` | Passed; local installer and SHA256SUMS generated under ignored `build/` |
| MSVC `mt.exe` manifest extraction | Embedded PerMonitorV2 and asInvoker confirmed |
| Silent installer legacy guard | Returned exit code 7 and installed no files; log confirmed nonadministrative HKCU mode |
| `TabGlide.exe --exit` with no Rust instance | Returned 0; existing AHK process remained running |
| `git status`, `git diff`, `git diff --check` | Inspected; whitespace check passed; changes left uncommitted |
| Archived AHK content | Both `git hash-object` values exactly match original HEAD blobs |

Local artifacts: `build/TabGlide-1.1.0-windows-x64-setup.exe`, `build/SHA256SUMS.txt`, `build/embedded.manifest`, `build/legacy-guard-test.log`, `build/final-cargo-tree.txt` and `build/final-diff.patch`. The installer SHA256 from this build is `9ee5f4f2c5d07d386384be08e94dff757f755f1fadee9d6edba6f0affa54d5d1`; rebuilding Inno may produce a different hash. Executables are ignored and none are tracked. GitHub-hosted CI and tagged release publication were not run; no push, tag or release was performed.

Core tests cover allowlist, activation boundary, both directions, already/unfocused targets, return disabled/enabled, first-original preservation, repeated scroll deadline/generation changes, early/stale timers, cancellation and unknown context. Config tests cover defaults, partial config, validation and byte-for-byte preservation of valid/invalid existing files. Core tests cover focus-before-input sequencing, activation-result rollback, input-result handling and multi-target focus return; Windows adapter fakes cover native focus/input result translation, vanished windows and foreground-loss safety. Logging regression coverage verifies an already-visited callsite through disabled/enabled/disabled/enabled transitions.

The interactive Windows smoke test was **attempted, not passed**: `GetCursorPos` failed with `0x80070005` (Access is denied) in the tool execution session before creating test windows or injecting input. It remains an opt-in ignored test for an unlocked interactive desktop. It exercises the real low-level hook and original-wheel delivery to an owned test window, monitor-relative context, Ctrl+Tab/Ctrl+Shift+Tab receipt, foreground activation/return, invalid-window rejection and idle shutdown.

## Manual desktop acceptance checklist

These checks remain unverified unless a result is explicitly recorded:

- Chrome/Firefox/Edge/Terminal and Explorer tabs: both wheel directions, original wheel passthrough, focused and unfocused targets.
- Burst focus return at 700 ms, delay extension, crossing several target windows, return to an original Explorer window, closing the original target before timeout.
- Allowlist rejection, pixel 50 inclusive and pixel 51 excluded, multiple monitors with negative origins and mixed DPI.
- Modified wheel gestures, denied foreground activation and an elevated target (no automatic elevation or input to an unrelated window).
- Enable/Disable, Settings, invalid/valid Reload Config, logging on/off, Diagnostics and Exit; Explorer restart restores the tray icon.
- First-run config creation, second-instance no-op, idle operation and shutdown; no persistent modifier keys after input errors.
- Per-user silent install, upgrade while running, autostart selection/deselection, silent uninstall and config/log retention. Back up and uninstall legacy AHK before creating Rust configuration.

The migration plan explicitly permits reporting manually untested runtime behavior. Implementation, tests and documentation must not be presented as proof of those desktop observations.

## Review record

- ARCHITECTURE: dependency direction approved; emphasized first original window, inclusive boundary, bounded queues and failure rollback.
- TEST: multi-target focus-return regression coverage keeps the first original window across a burst. Added valid-config preservation and destruction-during-activation coverage.
- WINDOWS: modifier filtering moved before focus mutation; TaskbarCreated handling moved into WndProc/private-message forwarding; partial-input cleanup restricted to inserted keys.
- PACKAGING: added a legacy-install guard and migration instructions, plus removal of the existing startup shortcut when autostart is deselected. Preserved the user's `AppVersion "1.1"` default.
- Fresh final REVIEW: fixed dynamic logging filtering and minimum Windows version. Follow-up inspection confirmed both fixes and no outstanding Critical/High or substantive Medium findings. Reviewers made no implementation edits.

## Requirement audit

| Migration requirement | Current evidence |
| --- | --- |
| Six-crate Cargo workspace, stable Rust, required stack | Root Cargo.toml, rust-toolchain.toml, Cargo.lock and inspected tree |
| Existing AHK archived, not deleted | `legacy/ahk/` with matching source hashes |
| Pure OS-independent decision engine; explicit clock/state | `tabglide-core/src/lib.rs`, forbid-unsafe lint, std-only tree, cross-target checks |
| Semantic SwitchTab/RestoreFocus; generation/deadline logic | Core code and deterministic burst/timer tests |
| Top-level window, application and monitor context | `native/window.rs`; actual desktop context smoke test remains unverified |
| Dedicated WH_MOUSE_LL thread, bounded nonblocking queue, wheel passthrough | `native/hook.rs`, independent Windows/final reviews; real wheel delivery remains unverified |
| No polling, per-event threads or config/UI work in hook | Event wait and callback source, bounded-batch review |
| Focus success validation, missing windows and input failures | Core/adapter tests plus native validation/SendInput code; real desktop/UIPI cases remain unverified |
| Focus return including Explorer | `docs/behavior.md`, preserved legacy source for comparison, core multi-target return regression |
| Single instance, diagnostics, lifecycle and native tray actions | `native/mod.rs`, `logging.rs`, `tray.rs`; logging test and no-instance exit test; other desktop actions remain unverified |
| TOML defaults and user config/log locations | Config tests, startup paths, README; no file shipped next to EXE |
| PerMonitorV2 / standard-user manifest | Extracted release executable manifest |
| Unsafe confined to platform crate with safety comments | Source inspection and forbid-unsafe in core/config/app/stubs |
| macOS/Linux honest stubs, no Windows dependency inflation | Stub source, target-specific manifests, cross-target checks and cargo tree |
| Per-user Inno, startup choice, safe config/log retention, silent switches | Installer source, successful compile, actual non-admin legacy guard test; install/upgrade/uninstall lifecycle remains manual |
| CI format/lint/tests/release build and optional other OS builds | `.github/workflows/ci.yml` and corresponding local checks |
| Tagged x64 installer, SHA256, GitHub artifacts | `.github/workflows/release.yml`; equivalent local installer/hash generation; no publication |
| README status and architecture topics | README, architecture.md, behavior.md, this validation record |
| Required independent review phases | Review record above, followed by primary-owned fixes |
| Existing installer user change preserved; no destructive Git/remote actions | Default version remains 1.1; worktree/status and local-only workflow |

Changed areas: root workspace/toolchain/lock/ignore files; `apps/`; five libraries under `crates/`; `legacy/ahk/`; `docs/`; README; Inno installer; GitHub workflows; replacement of the invalid placeholder WinGet JSON with submission instructions. The pre-existing untracked AGENTS.md was left unchanged. The installed legacy AHK executable and its user data were left untouched.
