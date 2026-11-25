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

This is the plan for a production update key. It is not approved. This repository has not generated that key, and no workflow reads an update secret. `nsh.json` stays without `updater`. The `updater` feature stays off.

Checked against NSH tag `v0.6.1`, commit `2f51d8611677c8a6bbad937b39756e8d0cc45350`. The machine that wrote this note has no `nsh` binary. GitHub's latest NSH release is `v0.7.0`, and crates.io's newest stable `nsh-cli` is `0.7.0`. Both are the wrong CLI for this repo. Do not run `nsh keygen` here, and do not install "latest".

### Generate

On Morris's Mac, install the pinned CLI, then check the version before generating anything:

```sh
curl -fsSL https://nsh.dev/install.sh | NSH_VERSION=v0.6.1 sh
nsh --version
```

Stop if that is not 0.6.1. The `v0.6.1` release asset for that Mac is `nsh-macos-arm64.tar.gz`. `--nightly` and `--source` build `main`, not this pin. `brew install longbridge/tap/nsh` was not checked for version. `cargo install nsh-cli` without `--version 0.6.1` installs 0.7.0.

Official command, from `crates/nsh-cli/src/sign.rs` at that tag:

```sh
nsh keygen "$HOME/Library/Application Support/Slowpage/update-keys/update"
```

The official example `nsh keygen keys/update` is a relative path. Run inside a clone and the files land in the worktree. Use the absolute path above, which is outside every clone. The command writes:

- `update.secret`, opened with mode `0o600` and `create_new`. The kernel stores `mode & ~umask`. A umask only clears bits, so the file is `0600` or narrower. It is not opened wider than `0600`. `ls -l` should show owner read and write, and no group or other bits. A narrower mode stays narrower. Do not add permissions.
- `update.pub`, written with `fs::write`. That is not mode `0o600`.

If either path already exists, the command stops with `<path> exists; keys are never overwritten` and writes nothing. The secret file is the base64 seed plus a newline. There is no passphrase. The command prints a key id and both paths to the terminal. Do not paste that output into a pull request, a log, or chat.

`gpg` and `openssl` on a Linux agent are not this command. They do not create the NSH trust root.

### Backup, recovery, rotation

The working `update.secret` may stay on the Mac that generated it. A second copy has to leave that Mac. A second file on the same disk, or a Time Machine backup of that Mac, is not this copy. Losing the Mac must not destroy every copy of the seed. `update.pub` cannot rebuild it. GitHub cannot return a secret's value. `gh secret list` fields are `name`, `numSelectedRepos`, `selectedReposURL`, `updatedAt`, and `visibility`.

This note does not assume a drive or a vault already exists. Choose one of these before `nsh keygen`, and create it if it does not exist:

- An encrypted removable volume that is not the generation Mac's disk. In Disk Utility, that is an APFS (Encrypted) volume with a password. The password is not stored only in that Mac's keychain. The drive is kept somewhere that is not the Mac.
- A secrets vault Morris already operates, if he has one. This note does not name a product and does not treat a missing vault as present.

Keep either copy out of the repo, out of a directory that syncs into a clone, and out of mail. NSH 0.6.1 has `keygen`, `certify`, `publish`, `verify`, `revoke`, and `update-manifest`. It has no recover command, no rotate command, and no command that prints the public half of a `.secret` file. `revoke` is for package certificates. It does not replace `updater.publicKey`. `SecretKey::to_base64` is standard base64 of the 32-byte seed. `writeln!` adds a newline, so the file is 45 bytes. `from_base64` trims whitespace.

The recovery drill uses the copy after it has left the Mac. Do not `cat` the file, do not run `nsh keygen`, and do not call `gh secret set`.

1. Copy `update.secret` to the chosen medium. Eject a drive and mount it again, or fetch the vault item again, so the check is not the write cache.
2. On the generation Mac, `cmp` the original and that copy. No output means the bytes match.
3. Write the SHA-256 of the file on paper and keep the paper with the medium, not on the Mac and not in git. `shasum -a 256` prints the hash only.
4. Take the medium off the Mac, or confirm the vault item is not stored on that Mac.
5. On a second machine Morris controls, copy the backup to a temporary file and `chmod 600` it. `shasum -a 256` must match the paper. `wc -c` must be 45. `openssl enc -base64 -d -A` of the file, piped to `wc -c`, must be 32. Then delete the temporary file. This checks the bytes and the encoding. It does not sign a manifest. The CLI cannot do this check by itself.

`latest.json` has `id`, `version`, `notes`, `localizedNotes`, `platforms`, and `package`. It has no next key. `Config::open` verifies with the `publicKey` already in the running app, or, when that field is absent, it does not verify. A signature from a different key fails with `the signature does not match the app's publicKey`. An OTA package cannot change this: the updater reads the shipped `nsh-updater.json`, not the package's. This app does not write that file and does not call the updater.

A later rotation is a full app build whose own `publicKey` is the new key, delivered by a manifest signed with the old key. Clients that install that build then trust only the new key. Clients that skip it keep the old key, and later manifests signed only by the new key are refused. Delete the old `.secret` only after Morris accepts that those clients cannot be updated through this mechanism. Until the bridging build exists, losing the only `.secret` ends signed updates for every install that would have trusted it. A new install, outside the updater, is the remaining path.

`updater.github` reads `releases/latest/download/latest.json`. That URL skips drafts and prereleases. A prerelease tag is not an update source. This candidate does not create a tag.

The same key material can sign packages and manifests. Manifest signatures use the context `nsh-updater-manifest\0`, so a manifest signature is not a package signature.

### Actions secret

Name: `NSH_UPDATE_SECRET`. Scope: an Actions environment secret on environment `update-signing`, application `actions` only. Not Dependabot, not Agents, not Codespaces, and not a repository-level secret.

This repo's Release and CI workflows run on `pull_request` from branches in the same repository. GitHub does not pass secrets to a workflow triggered from a forked repository, except `GITHUB_TOKEN`. A same-repo pull request is not that exception. GitHub's hardening guide says required reviewers can hold an environment secret until a reviewer approves that job. A repository-level secret has no such wait. Any workflow on `pull_request` that references it, including a pull request that edits the workflow, can receive it.

The environment does not exist yet. `GET /repos/realmorrisliu/slowpage/environments` returned no environments, and `GET .../environments/update-signing` returned 404. The owner is a user account, not an organization. Nothing below is configured. It is the protection to approve before anyone creates the environment.

This repository is public. GitHub's deployment docs say environments, environment secrets, and deployment protection rules are available on public repositories for current plans, and required reviewers are available on public repositories for GitHub Free, Pro, and Team. The repository owner configures an environment in a personal-account repository. The settings, as named in [Manage environments](https://docs.github.com/en/actions/how-tos/deploy/configure-and-manage-deployments/manage-environments):

- Required reviewers: the repository owner, and no one else.
- Leave `Prevent self-review` unselected. That setting blocks the person who started the run from approving it. The owner is the only reviewer, so selecting it and also turning off administrator bypass locks the signing job. Nobody else can approve it.
- `Allow administrators to bypass configured protection rules` may be turned off. The owner then approves through the environment review instead of skipping it. With `Prevent self-review` left off, he can still approve a run he started.
- Choosing `Prevent self-review` requires a second required reviewer who is a real person with read access. This note does not name one.
- Deployment branches: Selected branches and tags, limited to the protected branch and the version tags chosen later. A pull request head is not in that list.
- The secret is referenced only by a manual `workflow_dispatch` on one of those refs, or by a signing job on one of those refs. The `pull_request` job does not set `environment: update-signing` and does not read `NSH_UPDATE_SECRET`. Do not approve the environment for a pull request.

This agent's `gh` token cannot do that step. `GET /repos/realmorrisliu/slowpage/actions/secrets`, the secrets public-key endpoint, the Agents and Dependabot secret lists, and Actions permissions all returned `403 Resource not accessible by integration`. `gh secret set` is installed and was not run. A 403 on the public-key endpoint means this token cannot encrypt a secret for storage. Do not work around that with a browser or another token. The list API was not available, so this note does not say which secret names already exist. The workflow file reading six Apple names does not answer that either.

After Morris approves the environment and the backup, he sets the secret from his Mac. Stdin, not `--body`, so the seed is not a command argument:

```sh
gh secret set NSH_UPDATE_SECRET --app actions --env update-signing --repo realmorrisliu/slowpage \
  < "$HOME/Library/Application Support/Slowpage/update-keys/update.secret"
```

No workflow may reference `NSH_UPDATE_SECRET` until a job that is not triggered by `pull_request` sets `environment: update-signing`. Do not approve that environment for a pull request. `nsh update-manifest --key` takes the secret file's path. Do not put the seed on the command line. GitHub redacts a registered secret in logs only when the exact value appears, and the hardening guide says a transformed value can miss that redaction. Do not print the file.

`updater.publicKey` and the `updater` feature stay unchanged until the `.pub` text is in the app and a manifest is signed with the matching `.secret`. The CLI refuses a mismatch: `nsh.json has updater.publicKey: pass --key, or apps refuse this manifest`, and `--key without updater.publicKey in nsh.json: apps check nothing`. With neither, `nsh update-manifest` writes an unsigned manifest. That file is not a candidate.

Notarization returns HTTP 403 when the Apple Developer Program agreement is missing or expired. The job still uploads the signed disk image, then fails. Sign the agreement at [developer.apple.com/account](https://developer.apple.com/account) and run the workflow again. That is an account step, not a missing secret.

The app executable is the release `spike-shell` binary. Packaging reads the name, bundle id, and icon from `spikes/shell/nsh.json`, copies that binary into `Contents/MacOS`, and finishes load commands, the icon, `Info.plist`, and `zh-Hans.lproj/InfoPlist.strings` before the first `codesign`. The icon source is `assets/icon/Slowpage.svg`. `spike-icon` rasterizes every icns size from 16 through 1024, and the 16 and 32 point @1x slots use a wider-eyed slab with the page sliver and back rim left out. `scripts/check-reader-bundle.sh` fails the build when `CFBundleExecutable` is not that binary, the bundle id is not `app.slowpage.reader`, or the binary contains the text SuperReader spike. S1 removed the packaged `open` and `open -a` check from this workflow. `scripts/packaged-open.sh` is not run, and the job does not repair it.

Signing order is the inner `libpdfium.dylib`, then `Slowpage.app` with hardened runtime, a timestamp, and `scripts/entitlements.plist`, then the disk image. After the app is signed, and again for the copy inside the mounted disk image, the build runs `codesign --verify --deep --strict --verbose=2`. A failed verify fails the job. `notarytool` uses the API key. `stapler` tickets both the disk image and the app.

## Local development

`SR_SIGN_IDENTITY` defaults to `-` (ad hoc). `SR_NOTARY_PROFILE` is a `notarytool` keychain profile. Leave it unset to skip notarization. When the identity is a Developer ID, `just dmg` signs the disk image and, if the profile is set, submits it and staples it.

`just app` writes `dist/Slowpage.app`. `just dmg` writes `dist/Slowpage-<version>.dmg`, where `<version>` is the semver in `nsh.json` (`0.1.0-alpha.2` for this candidate). `just install` copies the app to `/Applications` on this Mac. The copy is a developer convenience. The release people install is the disk image.

## Finder open

`Info.plist` lists EPUB (`org.idpf.epub-container`) and PDF (`com.adobe.pdf`). The package script writes those types before it signs. `nsh.json` `fileTypes` names the same two kinds. The release does not call `nsh package`, so that field does not rewrite the plist. Switching the release to `nsh package` is a later cleanup. That command writes `CFBundleDocumentTypes` and does not write the imported type that marks an EPUB folder as a package.

NSH 0.5.0 delivers those URLs through `application:openURLs:` into the core. The shell listens with `next_open_files` and queues the paths. File > Open (⌘O) and dropping a book on the window do the same.
