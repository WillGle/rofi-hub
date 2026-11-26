# Architecture

`rofi-apps` is a set of independent Rofi apps with an optional menu and an
installer. The dependency graph is one-way:

```text
Apps ──depend──► rofi-dmenu (library, semver)
Apps ──depend──► rofi (runtime, wrapped by each app package)
Apps ──never depend──► rofi-hub-menu / rofi-hub-installer
Installer ──copies/verifies──► app packages
Menu (optional) ──reads manifests, execs apps──► app packages
```

## Roles

| Component | Responsibility | Must not do |
|---|---|---|
| `installer/install.sh` | verify index/tarballs, copy files, record per-app state, uninstall/update one app | install dependencies, set env/PATH, launch apps, own themes |
| `rofi-hub-menu` | read provider manifests, show a menu, exec apps | inject theme/env, own app dependencies, break apps when removed |
| `rofi-dmenu` | dmenu protocol, theme resolver, paths, UI helpers | depend on the menu or installer |
| `rofi-<app>` | binary + provider + app.toml + own dependencies | depend on the menu or installer |

## Workspace

| Path | Contents |
|---|---|
| `crates/rofi-dmenu` | shared library, version `0.3.0` |
| `crates/rofi-calendar` | Calendar 1.2.0 |
| `crates/rofi-clipboard` | Clipboard 0.8.0 |
| `crates/rofi-keybinds` | Keybinds 1.0.0 |
| `crates/rofi-network` | Network 0.3.0 |
| `crates/screen-time` | Screen Time 0.5.0, daemon and Rofi UI |
| `crates/rofi-hub-menu` | optional menu 0.3.0 |
| `modules/nixos` | per-app and umbrella modules |
| `modules/home-manager` | per-app and umbrella modules |
| `installer/index.schema.json` | release index schema |

## App package contract

Each app package contains:

```text
app.toml
bin/<app>
share/rofi/<id>.rasi
share/rofi-apps/providers/<id>.toml
```

`app.toml`:

```toml
id = "calendar"
version = "1.2.0"
bin = "rofi-calendar"
provider = "providers/calendar.toml"
requires = ["rofi"]
optional_requires = []
service = false
```

Rules:

- `requires` is a prerequisite check (`command -v`); the installer never
  installs anything.
- The Nix package wraps the app's own runtime dependencies on `PATH`.
- The app resolves `ROFI_BIN`, `ROFI_CONFIG`, `ROFI_THEME`, the user base
  theme, its user override, and its packaged layout itself.
- The app never reads hub-owned environment variables.
- The app runs directly or through the optional menu.

## Provider registry

Provider manifests belong to app packages and are installed under
`$XDG_DATA_HOME/rofi-apps/providers` (Nix: `$out/share/rofi-apps/providers`).
The menu also reads the legacy `$XDG_DATA_HOME/rofi-hub/providers` path during
the migration window. Manifests are static: `exec` is an argument vector, not
a shell string.

The menu launches a provider with the user's inherited environment. It does
not set a theme, provider id, config, or `PATH` variable.

## Theme resolution

```text
1. ~/.config/rofi/<app>.rasi        full app override
2. ROFI_THEME                       full global override
3. ~/.config/rofi/theme.rasi        user base + app layout
4. $XDG_DATA_DIRS/rofi/<app>.rasi   packaged fallback
5. built-in fallback                palette + layout
```

Generated runtime themes live in `$XDG_RUNTIME_DIR/rofi/theme-<app>.rasi`
(mode 0600, directory 0700). The generator compares content before writing, so
a cache hit does not rewrite the file. The menu has its own theme and state
under `~/.config/rofi-hub-menu`; it never touches an app's theme.

## Installer

`install.sh` reads a signed `index.json` and per-app tarballs. State is stored
per app:

```text
~/.local/state/rofi-apps/installed/<id>
~/.local/state/rofi-apps/version
```

The installer:

- verifies the index and each tarball with minisign and SHA-256 by default;
- rejects unsigned input unless `--allow-unsigned` is passed for local tests;
- rejects absolute paths, `..` components, links, and devices in tarballs;
- checks `requires` with `command -v` before touching files;
- preflights every selected app before installing any file;
- backs up and restores files if an install step fails;
- installs app-owned service units only when the app declares one;
- refuses to write into a Nix-managed path;
- keeps state files at 0600 and state directories at 0700.

It does not install Rofi, plugins, `cliphist`, `wl-clipboard`,
NetworkManager, themes, or `PATH` wrappers.

## Release index

```json
{
  "schema": 1,
  "installer": { "version": "0.3.0" },
  "apps": [
    {
      "id": "calendar",
      "version": "1.2.0",
      "url": "rofi-calendar-1.2.0-x86_64.tar.gz",
      "sha256": "...",
      "requires": ["rofi"]
    }
  ]
}
```

Optional apps (the menu) carry `"optional": true` and are skipped by `--all`.
The schema is in `installer/index.schema.json`. Tarballs and the index are
signed with minisign (`.minisig` files next to each artifact).

## Nix

Packages:

```text
packages.rofi-calendar
packages.rofi-clipboard
packages.rofi-keybinds
packages.rofi-network
packages.screen-time
packages.rofi-hub-menu
packages.rofi-hub-installer
```

Each package builds exactly one crate and installs that app's binary, provider,
layout, and app.toml. `rofi-hub-apps` remains only as a compatibility bundle.

Modules:

```nix
services.rofi-apps.calendar.enable = true;
services.rofi-apps.clipboard.enable = true;
services.rofi-apps.screen-time.enable = true;
services.rofi-apps.menu.enable = false;
```

There is no global `runtimePackagesFor`. Each module installs its own package.
The `screen-time` module owns the user service. The menu module is optional.
`services.rofi-hub.enable` remains a deprecated alias that maps the old `apps`
list to per-app options.

## Versions

Each app has an independent version, and its provider manifest uses the same
version. `rofi-dmenu` has its own semver (`0.3.0`). The installer has its own
version. Updating one app does not require a menu or installer release.
