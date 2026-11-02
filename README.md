<p align="center"><img src="logo.png" width="128" alt="Actions Lights"></p>

<h1 align="center">Actions Lights</h1>

<p align="center">
  Your GitHub Actions workflows as a traffic light in the menu bar or system tray.
  <br>
  <a href="README.es.md">Leer en español</a>
</p>

<p align="center">
  <a href="https://github.com/luyz2000/actions-lights/releases/latest"><img src="https://img.shields.io/github/v/release/luyz2000/actions-lights?label=release" alt="Latest release"></a>
  <a href="https://github.com/luyz2000/actions-lights/actions/workflows/ci.yml"><img src="https://github.com/luyz2000/actions-lights/actions/workflows/ci.yml/badge.svg?branch=main" alt="CI"></a>
  <a href="LICENSE"><img src="https://img.shields.io/github/license/luyz2000/actions-lights" alt="MIT License"></a>
  <img src="https://img.shields.io/badge/platforms-macOS%20%7C%20Linux%20%7C%20Windows-blue" alt="macOS, Linux and Windows">
  <a href="https://github.com/luyz2000/actions-lights/releases"><img src="https://img.shields.io/github/downloads/luyz2000/actions-lights/total" alt="Downloads"></a>
</p>

Merge to your release branch, keep working, and glance at the menu bar: red if something failed, yellow while it runs, green when it's done.

## Features

- **Traffic light in the menu bar:** red if something failed, yellow while running (with a progress ring), green when everything finished. When nothing is going on it shows the app icon and "All clear".
- **Short text next to the light** (macOS): the running workflow, its current step and elapsed time.
- **Status panel on click:** workflows grouped by repo, with the current step, progress against the usual duration, the step that failed, the PR title or commit, who triggered it, the branch and the event. One click opens the run on GitHub.
- **Account detected on its own:** your GitHub CLI (`gh`) session, or a personal token stored in the system keychain.
- **In-app updates:** the app checks GitHub Releases every 6 hours; one click installs and restarts.
- **Six languages:** English, Spanish, Russian, French, Japanese and Chinese. It follows the system language, or you can pick one in **Settings → General**.
- **macOS, Linux and Windows.**
- **Notifications (coming soon):** paused until the macOS app is signed by Apple.

## Install

Download the file for your system from the [latest release](https://github.com/luyz2000/actions-lights/releases/latest).

| System | File | Notes |
|---|---|---|
| macOS (Apple Silicon and Intel) | `Actions-Lights_X.Y.Z_universal.dmg` | Drag the app to **Applications**. |
| Debian / Ubuntu (x86_64) | `Actions.Lights_X.Y.Z_amd64.deb` | `sudo apt install ./Actions.Lights_X.Y.Z_amd64.deb` |
| Other Linux (x86_64) | `Actions.Lights_X.Y.Z_amd64.AppImage` | `chmod +x` it and run it. |
| Windows (x86_64) | `Actions.Lights_X.Y.Z_x64-setup.exe` | Installs for your user, no admin rights needed. |

Then turn on the workflows you want to watch in **Settings → Repos & workflows**. Usually that's the deploy of your release branch.

## Connect your GitHub account

The app uses the first account it finds:

1. **GitHub CLI:** if `gh auth login` was done on this machine, there's nothing to set up.
2. **Personal token:** paste it in **Settings → Account**. Use a classic token with the `repo` scope, or a fine-grained token with **Actions: read** (and **Metadata: read**) on the repos you want to watch.

Without an account the app can still watch public repos, refreshing every 5 minutes.

## How it works

- **Polling:** every 15 s while something is running, every 60 s when idle, and every 5 min without an account. It spaces requests out on its own when the API rate limit runs low. It uses conditional requests (ETags), so unchanged answers don't count against the limit.
- **Offline:** if GitHub can't be reached (Wi-Fi coming back, just logged in), it retries after 5, 10, 20 and 40 seconds, then goes back to the normal pace.
- **Progress:** estimated from the job's steps and the median duration of the last successful runs.
- **Storage:** the token lives in the system keychain (macOS Keychain, Windows Credential Manager, or Secret Service on Linux). Settings are a JSON file in the app's config folder.

## Privacy

- No telemetry, analytics or accounts of our own.
- The app only talks to `api.github.com` (your workflows) and to GitHub Releases of this repo (updates).
- Your token never leaves your machine except in requests to GitHub.

## Troubleshooting

<details>
<summary><b>macOS says the app can't be opened or "is damaged"</b></summary>

The app isn't signed with an Apple Developer certificate yet. Open it once with **right click → Open**, or run:

```sh
xattr -cr "/Applications/Actions Lights.app"
```
</details>

<details>
<summary><b>Windows SmartScreen blocks the installer</b></summary>

The installer isn't code-signed yet. Click **More info → Run anyway**. Once installed, the light lives in the tray; it may be hidden behind the **^** arrow on the taskbar.
</details>

<details>
<summary><b>No icon in the tray on Linux (GNOME)</b></summary>

GNOME hides tray icons by default. Install the **AppIndicator and KStatusNotifierItem Support** extension. On Linux the status panel opens from the icon's menu (**Show status**).
</details>

<details>
<summary><b>"Can't reach GitHub" stays on screen</b></summary>

The app retries on its own within seconds once the network is back. If it persists, check your connection or proxy, then use **Refresh now** from the icon's menu.
</details>

<details>
<summary><b>"Invalid or expired token"</b></summary>

Run `gh auth login` again, or replace the token in **Settings → Account**. For organizations with SSO, the banner links to the page where you authorize the token.
</details>

<details>
<summary><b>A workflow shows "Not found"</b></summary>

The token can't see that repo: it's private and the token lacks access, or the repo was renamed. Check the token's scopes or repository access.
</details>

## Contributing

Contributions are welcome through **pull requests**. Every PR is reviewed and merged by the maintainer; nothing is merged automatically. For larger changes, please open an issue first to discuss the idea.

To add or fix a translation, edit the files in [`src/lib/i18n/`](src/lib/i18n/). Every language must keep the same keys and `{placeholders}` as `en.json`; the tests check it.

## License

[MIT](LICENSE) © 2026 luyz2000
