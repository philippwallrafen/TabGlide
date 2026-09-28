# WinGet submission

The previous JSON contained a placeholder hash and obsolete artifact URLs; it was not a valid submission. Generate a real YAML manifest only after a maintainer publishes a tagged installer.

Use `wingetcreate new <published-installer-url>` and validate with `winget validate --manifest <directory>`. Review these values:

- PackageIdentifier: `PhilippWallrafen.TabGlide` (coordinate ownership/name with the community repository before the first submission).
- PackageVersion: the `X.Y.Z` from the tag and Cargo workspace.
- InstallerType: `inno`; Scope: `user`; Architecture: `x64`.
- MinimumOSVersion: `10.0.0.0` (the installer rejects earlier Windows versions).
- InstallerUrl: `https://github.com/philippwallrafen/TabGlide/releases/download/vX.Y.Z/TabGlide-X.Y.Z-windows-x64-setup.exe` with the real version substituted.
- InstallerSha256: the exact published installer SHA256 from `SHA256SUMS.txt`.
- UpgradeBehavior: `install`; AppsAndFeaturesEntries ProductCode: `TabGlide_is1`.
- Silent switches: `/VERYSILENT /SUPPRESSMSGBOXES /NORESTART /SP-`; no administrator override.

Do not submit placeholder URLs or checksums. WinGet submission and release publication are maintainer actions, not part of local validation. See [Microsoft's manifest guide](https://learn.microsoft.com/windows/package-manager/package/manifest).
