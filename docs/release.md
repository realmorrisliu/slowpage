# macOS release

α1 ships as a signed disk image from GitHub Actions. A person installs it by dragging Slowpage onto Applications. Local `just app` and `just dmg` stay ad hoc for development.

The workflow is `.github/workflows/release.yml`. It runs on a `v*` tag and on `workflow_dispatch`. A tag publishes a GitHub Release. A manual run does not, unless its `publish` input is turned on. This repository does not push that tag from the α1 pull request.

The Release asset is `Slowpage-<version>.dmg`. `<version>` is the semver in `spikes/shell/nsh.json`. `scripts/app-version.sh` reads it. `CFBundleShortVersionString` and the disk image use that string. `CFBundleVersion` is the `build` integer in the same file. The current candidate is `0.1.0-alpha.2`, build `1`. It does not write `latest.json` and does not set `updater`.

NSH 0.5.0 ships `nsh-updater` in the stable crate, tag `v0.5.0`. NSH 0.6.1 makes `updater` a feature of `nsh-platform-macos`. This shell leaves that feature off (`default-features = false`). `nsh-engine` still depends on the `nsh-updater` crate. The app does not call it. A GitHub source is `updater.github` set to `owner/repo`. The manifest is the `latest.json` asset of the latest GitHub Release. Drafts and prereleases are skipped. The URL is `https://github.com/<owner>/<repo>/releases/latest/download/latest.json`. `updater.publicKey` is the text of an `nsh keygen` `.pub` file. That key is Ed25519. Officially it is optional for `github`. Once it is set, `latest.json.sig` must verify, and an unsigned manifest is refused. Without it, a GitHub source accepts an unsigned manifest. A custom `updater.url` requires `publicKey`. `nsh update-manifest` writes `latest.json` and the signature. `scripts/updater-gate.sh` exits 0 when `updater` is absent and exits 1 when `updater` has no 32-byte `publicKey`. A gate result of `keyed` means the object has a key and one source. It is not a signature and not a running update. Production updates are not supported.

## Secrets

The signed job reads these GitHub Actions secrets from the environment and never prints them. A missing name fails the job with `missing secret: <NAME>`. It does not fall back to an ad hoc signature.

| Secret | Role |
| --- | --- |
| `MACOS_CERT_P12_BASE64` | Base64 of the Developer ID Application PKCS#12 |
| `MACOS_CERT_P12_PASSWORD` | Password for that PKCS#12 |
| `APPLE_TEAM_ID` | Apple team id inside the certificate name |
| `ASC_API_KEY_P8` | App Store Connect API key, PEM or base64 PEM |
| `ASC_API_KEY_ID` | Key id for `notarytool` |
| `ASC_API_ISSUER_ID` | Issuer id for `notarytool` |

The job creates a temporary keychain with a random password, imports the certificate, signs, notarizes, staples, and deletes the keychain.

## Update key

This is the plan for a production update key. Nothing in this repository has generated that key, and this release does not install it.

On a machine you control, with the NSH 0.6.1 CLI (tag `v0.6.1`, not a nightly), outside this repo:

```sh
nsh keygen keys/update
```

`nsh keygen` writes `keys/update.secret` with mode `0600` and `keys/update.pub`. It uses `create_new` and does not overwrite either file. Keep `.secret` out of the repo, out of logs, and out of chat. The same key material can sign packages and manifests. The signatures use different contexts.

Store the contents of `keys/update.secret` as a GitHub Actions secret you choose. The release workflow does not read an update-key secret today. Do not put the secret in the workflow env until a later change is ready to call `nsh update-manifest --key`.

After you approve the trust root, a later change can set `nsh.json` `updater.publicKey` to the text of `keys/update.pub` and pass `--key` to `nsh update-manifest`. Until that change, the app stays closed. An unsigned `latest.json` is not a candidate. The NSH 0.6.1 CLI refuses a mismatch: `nsh.json has updater.publicKey: pass --key, or apps refuse this manifest`, and `--key without updater.publicKey in nsh.json: apps check nothing`.

Notarization returns HTTP 403 when the Apple Developer Program agreement is missing or expired. The job still uploads the signed disk image, then fails. Sign the agreement at [developer.apple.com/account](https://developer.apple.com/account) and run the workflow again. That is an account step, not a missing secret.

The app executable is the release `spike-shell` binary. Packaging reads the name, bundle id, and icon from `spikes/shell/nsh.json`, copies that binary into `Contents/MacOS`, and finishes load commands, the icon, `Info.plist`, and `zh-Hans.lproj/InfoPlist.strings` before the first `codesign`. The icon source is `assets/icon/Slowpage.svg`. `spike-icon` rasterizes every icns size from 16 through 1024, and the 16 and 32 point @1x slots use a wider-eyed slab with the page sliver and back rim left out. `scripts/check-reader-bundle.sh` fails the build when `CFBundleExecutable` is not that binary, the bundle id is not `app.slowpage.reader`, or the binary contains the text SuperReader spike. S1 removed the packaged `open` and `open -a` check from this workflow. `scripts/packaged-open.sh` is not run, and the job does not repair it.

Signing order is the inner `libpdfium.dylib`, then `Slowpage.app` with hardened runtime, a timestamp, and `scripts/entitlements.plist`, then the disk image. After the app is signed, and again for the copy inside the mounted disk image, the build runs `codesign --verify --deep --strict --verbose=2`. A failed verify fails the job. `notarytool` uses the API key. `stapler` tickets both the disk image and the app.

## Local development

`SR_SIGN_IDENTITY` defaults to `-` (ad hoc). `SR_NOTARY_PROFILE` is a `notarytool` keychain profile. Leave it unset to skip notarization. When the identity is a Developer ID, `just dmg` signs the disk image and, if the profile is set, submits it and staples it.

`just app` writes `dist/Slowpage.app`. `just dmg` writes `dist/Slowpage-<version>.dmg`, where `<version>` is the semver in `nsh.json` (`0.1.0-alpha.2` for this candidate). `just install` copies the app to `/Applications` on this Mac. The copy is a developer convenience. The release people install is the disk image.

## Finder open

`Info.plist` lists EPUB (`org.idpf.epub-container`) and PDF (`com.adobe.pdf`). The package script writes those types before it signs. `nsh.json` `fileTypes` names the same two kinds. The release does not call `nsh package`, so that field does not rewrite the plist. Switching the release to `nsh package` is a later cleanup. That command writes `CFBundleDocumentTypes` and does not write the imported type that marks an EPUB folder as a package.

NSH 0.5.0 delivers those URLs through `application:openURLs:` into the core. The shell listens with `next_open_files` and queues the paths. File > Open (⌘O) and dropping a book on the window do the same.
