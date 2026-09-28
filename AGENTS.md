# TabGlide development rules

- Preserve established user behavior unless a change is explicitly documented.
- `tabglide-core` must remain completely OS-independent.
- Windows is currently implemented; macOS/Linux remain platform stubs.
- Do not introduce a shared platform trait until multiple real backends justify it.
- Keep unsafe code inside platform-specific crates.
- Do not introduce Tokio unless technically justified.
- Prefer event-driven designs; avoid polling.
- Preserve WheelUp/WheelDown passthrough semantics.
- Prefer descriptive code and small, explicit abstractions.
