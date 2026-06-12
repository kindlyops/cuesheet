# Releasing Cuesheet

Releases are fully automated from a version tag.

## One-time setup (repo secrets)

### Updater signing (required before the first release)

The in-app updater verifies update bundles with a keypair you own:

```sh
npx @tauri-apps/cli signer generate -w ~/.tauri/cuesheet.key
```

Add to the repo's Actions secrets:

- `TAURI_SIGNING_PRIVATE_KEY` — contents of the generated private key file
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` — the password you chose (may be empty)

Then paste the **public** key into `src-tauri/tauri.conf.json` under
`plugins.updater.pubkey`.

### macOS signing + notarization (optional, activates automatically)

- `APPLE_CERTIFICATE` — base64 of the Developer ID Application .p12
- `APPLE_CERTIFICATE_PASSWORD`
- `APPLE_SIGNING_IDENTITY` — e.g. "Developer ID Application: Your Name (TEAMID)"
- `APPLE_ID`, `APPLE_PASSWORD` (app-specific), `APPLE_TEAM_ID`

Without these the macOS build is unsigned (Gatekeeper warns on first open).

### Windows Authenticode (optional, activates automatically)

- `WINDOWS_CERTIFICATE` — base64 of the .pfx
- `WINDOWS_CERTIFICATE_PASSWORD`

Without these the Windows installers are unsigned (SmartScreen warns).

## Cutting a release

1. Bump versions (keep in sync): root `Cargo.toml` `[workspace.package] version`,
   `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, root `package.json`.
2. Commit, tag, push:

   ```sh
   git tag v0.2.0 && git push origin v0.2.0
   ```

3. The `release` workflow builds macOS (universal) and Windows (x64)
   installers, generates `latest.json` for the updater, and publishes a
   **draft** GitHub Release with all assets attached.
4. Review the draft and click **Publish**. Publishing is the only manual step;
   the updater serves users from `releases/latest/download/latest.json`.
