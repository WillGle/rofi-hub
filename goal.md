# Goal — Các app Rofi cài đặt và chạy độc lập

- **Trạng thái:** Đã chốt goal ngày 2026-11-27. Phần code/CI hoàn tất tại PR [#1](https://github.com/WillGle/rofi-hub/pull/1); các bước còn lại là vận hành phát hành (merge, tag, ký release, cài trên host ARM).
- **Ngày:** 2026-11-26 (chốt 2026-11-27)
- **Repo:** `/mnt/vault/dev/rofi-hub`
- **Liên quan:** tích hợp Nix tại `/etc/nixos` (flake input `rofi-hub`)

## 1. Bối cảnh

Trước khi tách app, `rofi-hub` vừa là:

- installer (`install.sh`);
- runtime menu/provider manager (`rofi-hub` binary);
- nơi cài dependency dùng chung (`rofiWithPlugins`, `rofi-power-menu`, `cliphist`, `wl-clipboard`, `networkmanager`);
- nơi inject theme/env cho app (`ROFI_HUB_THEME`, `ROFI_HUB_THEME_FILE`);
- nơi chứa app logic (`network.rs`);
- nơi bundle app (`rofi-hub-apps`, `rofi-hub-all-in-one`).

Điều này khiến hub trở thành **runtime dependency** và **dependency provider** của app, trái với mục tiêu:

> Hub chỉ đóng vai trò installation management, không được tác động đến dependency của apps.

Phạm vi phân phối trước đây bao gồm Nix, `install.sh`, `.deb`, `.rpm`, `.apk`, AUR, Debian official, Gentoo, openSUSE, Void, Slackware, ALT.

## 2. Quyết định đã chốt

1. **Hai kênh phát hành hiện hành**
   - **Nix native** là kênh chính. Người dùng cài từng app từ flake và cấu hình qua NixOS hoặc Home Manager. Nghiệm thu bằng native build trên x86_64-linux và aarch64-linux.
   - **`install.sh`** là kênh thứ hai. Installer chọn tarball của từng app theo kiến trúc máy và xác minh index cùng chữ ký trước khi cài. GitHub Release lưu các artifact phục vụ kênh này.
2. **Tạm dừng mọi phương thức phân phối khác.** Phạm vi tạm dừng gồm gói riêng từng distro, AUR, community recipe và source asset dành cho recipe. Chỉ mở lại khi có quyết định mới. Các bước cleanup trong M7 ghi nhận công việc đã thực hiện. Debian, Ubuntu, Arch và Fedora trong DIST-01..05 là môi trường kiểm tra cùng một `install.sh`.
3. **Hub chỉ quản lý ba thao tác trên gói app: cài đặt, gỡ cài đặt và cập nhật.** `install.sh` dùng metadata của từng app để kiểm tra prerequisite, xác minh gói, cài, cập nhật hoặc gỡ app người dùng chọn. Mỗi app tự khai báo dependency trong gói của mình.
4. **Mỗi app có package riêng, được cài, gỡ, cập nhật và chạy độc lập.**
   - Người dùng có thể cài riêng một app qua Nix native hoặc `install.sh --app <id>`, rồi gọi executable của app trực tiếp từ terminal hoặc phím tắt. Ví dụ: cài `calendar`, chạy `rofi-calendar`.
   - Mỗi app sở hữu executable, version, metadata, cấu hình, giao diện và phần xử lý của mình. App quyết định vòng đời xử lý: Calendar tính ngày khi mở, Network gọi NetworkManager qua `nmcli`, Screen Time có service riêng để ghi nhận khi UI đóng.
   - Provider manifest cho phép `rofi-hub-menu` tùy chọn tìm và mở app qua cùng executable. Cài, gỡ hoặc bỏ qua menu đều giữ nguyên khả năng chạy trực tiếp của từng app.
5. **Theme thuộc user + app, không thuộc hub.**
   - Base theme do user/theme module quản lý.
   - App layout do app package quản lý.
   - App override do user quản lý.
   - Menu có theme riêng, không inject cho app.

## 3. Mục tiêu kiến trúc

Dependency graph phải một chiều:

```text
Apps  ──depend──►  rofi + rofi-dmenu (library versioned)
Apps  ──không depend──►  hub / installer / menu
Installer ──chỉ copy/verify/quản lý file──►  app packages
Menu (optional) ──đọc manifest, exec app──►  app packages
```

### 3.1 Vai trò

| Thành phần | Trách nhiệm | Không được làm |
|---|---|---|
| `rofi-hub-installer` | cài/gỡ/update app; verify signature; ghi registry; chạy install metadata do app cung cấp | không cài dependency; không set env; không launch app; không chứa app logic; không quản lý theme state |
| `rofi-hub-menu` (optional) | đọc provider manifest, hiển thị menu, exec app | không inject theme/env; không sở hữu dependency app; gỡ menu không làm app hỏng |
| `rofi-dmenu` (library) | dmenu protocol, theme resolver, paths, UI helpers | không phụ thuộc hub/installer/menu |
| `rofi-<app>` | binary + provider + install metadata + dependency riêng | không depend hub/installer/menu |

### 3.2 Repo layout đích

```text
crates/
  rofi-dmenu/              # shared library, version riêng
  rofi-calendar/
    Cargo.toml
    src/
    app.toml
    providers/calendar.toml
    share/rofi/calendar.rasi        # optional fallback layout
  rofi-clipboard/
  rofi-keybinds/
  rofi-network/
  screen-time/
  rofi-hub-menu/           # optional runtime menu
installer/
  install.sh
  uninstall.sh
  index/                   # release index schema
flake.nix
```

Không bắt buộc đổi tên `rofi-core` ngay; có thể giữ tên nhưng phải trở thành library độc lập có semver. Tên `rofi-dmenu` chỉ để thể hiện vai trò.

### 3.3 App package contract

Mỗi app package cung cấp:

```text
bin/<app>
share/rofi-hub/providers/<app>.toml
share/rofi/<app>.rasi            # optional layout fallback
app.toml
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

Quy tắc:

- `requires` chỉ là prerequisite check; installer không cài.
- App tự wrap dependency trong package Nix.
- App tự resolve `ROFI_BIN`, `ROFI_CONFIG`, `ROFI_THEME`, `~/.config/rofi/<app>.rasi`.
- App không đọc `ROFI_HUB_*`.
- App có thể chạy trực tiếp hoặc qua menu.

### 3.4 Installer contract

`install.sh` phải:

- đọc `index.json` và `app.toml`;
- verify checksum + signature;
- cài đúng app được chọn;
- ghi state riêng từng app;
- uninstall/update riêng từng app;
- kiểm tra prerequisite bằng `command -v`, báo lỗi nếu thiếu;
- cài service/unit do app ship nếu app khai báo;
- không hardcode `KNOWN_APPS`;
- không set `ROFI_HUB_*`;
- không wrap PATH;
- không cài `rofi`, plugin, `cliphist`, `wl-clipboard`, `networkmanager`;
- không quản lý theme;
- không có `--nixos`.

### 3.5 Menu contract (optional)

- Đọc provider manifest.
- Exec app với environment sạch của user.
- Không set `ROFI_HUB_THEME`, `ROFI_HUB_THEME_FILE`, `ROFI_HUB_PROVIDER_DIR`.
- Có theme riêng; không tác động theme app.
- Không nằm trong dependency closure của app.

### 3.6 Theme contract

Ba lớp:

| Lớp | File / owner | Nội dung |
|---|---|---|
| Base theme | `~/.config/rofi/theme.rasi` hoặc `ROFI_THEME` | màu, font, bo góc |
| App layout | `share/rofi/<app>.rasi` hoặc app code | width, columns, rows, icon |
| User app override | `~/.config/rofi/<app>.rasi` | full theme cho một app |
| Menu theme | `share/rofi/hub-menu.rasi` + user override | theme của menu |

Thứ tự resolve:

```text
1. ~/.config/rofi/<app>.rasi        # full app override
2. ROFI_THEME                       # full global override
3. ~/.config/rofi/theme.rasi        # base + app layout
4. $XDG_DATA_DIRS/rofi/<app>.rasi   # packaged layout + base
5. built-in fallback                # palette + layout mặc định
```

Không còn:

```text
ROFI_HUB_THEME
ROFI_HUB_THEME_FILE
ROFI_HUB_PROVIDER
```

### 3.7 Provider registry

- Provider manifest thuộc app package, không thuộc hub.
- Có thể giữ path `~/.local/share/rofi-hub/providers` để tương thích.
- Cân nhắc đổi sang `~/.local/share/rofi-apps/providers`; giai đoạn chuyển tiếp đọc cả hai.
- Menu chỉ đọc manifest; không sở hữu.

## 4. Nix path

### 4.1 Package outputs

```text
packages.rofi-calendar
packages.rofi-clipboard
packages.rofi-keybinds
packages.rofi-network
packages.screen-time
packages.rofi-hub-menu            # optional
packages.rofi-hub-installer       # optional CLI
```

Mỗi app package:

- build `--package <crate>`;
- cài binary, provider, layout fallback;
- wrap runtime dependency của chính app:
  ```nix
  postFixup = ''
    wrapProgram $out/bin/rofi-calendar \
      --prefix PATH : ${lib.makeBinPath [ rofi ]}
  '';
  ```
- version lấy từ crate riêng.

`rofi-hub-installer` không chứa rofi/plugin/app.

### 4.2 Module outputs

```text
nixosModules.rofi-calendar
nixosModules.rofi-clipboard
nixosModules.rofi-keybinds
nixosModules.rofi-network
nixosModules.screen-time
nixosModules.rofi-hub-menu
homeManagerModules.*
```

Hoặc module tổng:

```nix
services.rofi-apps = {
  calendar.enable = true;
  clipboard.enable = true;
  screen-time.enable = true;
  menu.enable = false;
};
```

Yêu cầu:

- không còn `runtimePackagesFor` toàn cục;
- mỗi app module cài đúng package + dependency của app;
- `screen-time` tự sở hữu service;
- menu optional;
- `services.rofi-hub.enable` cũ chỉ giữ làm alias deprecated nếu cần.

## 5. install.sh path

### 5.1 Release index

```text
dist/
  index.json
  rofi-calendar-1.2.0-x86_64.tar.gz
  rofi-clipboard-0.8.0-x86_64.tar.gz
  rofi-keybinds-1.0.0-x86_64.tar.gz
  rofi-network-0.3.0-x86_64.tar.gz
  screen-time-0.5.0-x86_64.tar.gz
  rofi-hub-menu-0.3.0-x86_64.tar.gz   # optional
  SHA256SUMS
  *.minisig
```

`index.json`:

```json
{
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

### 5.2 Installer behavior

```text
install.sh --list
install.sh --app calendar
install.sh --all
install.sh --update calendar
install.sh --uninstall calendar
install.sh --no-service
```

Không còn:

```text
install.sh --nixos
install.sh --theme auto|nord|solarized|gruvbox   # chuyển sang rofi-theme nếu cần
KNOWN_APPS=(...)
components file gộp hub + app
```

State:

```text
~/.local/state/rofi-apps/installed/<id>
~/.local/state/rofi-apps/version
```

## 6. Versioning & release

- Mỗi app có version riêng:
  ```text
  calendar 1.2.0
  clipboard 0.8.0
  keybinds 1.0.0
  network 0.3.0
  screen-time 0.5.0
  ```
- Provider version khớp app version.
- `rofi-dmenu` có semver riêng; app pin version.
- Installer version riêng.
- Release tag theo app hoặc release index.
- Update app không cần hub release; update installer không đụng app.

## 7. Migration plan

### M0 — Chốt goal và guardrails

- [x] Tạo `goal.md`.
- [x] Bổ sung testing, integration matrix, Definition of Done chi tiết.
- [ ] Review và chốt `goal.md` với stakeholder.
- [x] Chốt menu optional hay bỏ.
- [x] Chốt `rofi-dmenu` ở monorepo hay tách repo.
- [x] Chốt registry path `rofi-hub` hay `rofi-apps`.

### M1 — Tách library khỏi hub

- [x] Đổi `rofi-core` thành library độc lập có version.
- [x] Feature-gate `manifest`, `notify`, `category`.
- [x] Bỏ `ROFI_HUB_*` khỏi theme resolver.
- [x] Bỏ `hub_theme_candidates`, `hub_theme_state_path`.
- [x] Đổi runtime dir `rofi-hub` → `rofi` hoặc `rofi-dmenu`.
- [x] Thêm guard: app crate không depend `rofi-hub`.

**Exit:** `cargo tree -p rofi-calendar` không có `rofi-hub`; app build không cần hub crate.

### M2 — Per-app packages

- [x] Thêm `packages.rofi-calendar`, `rofi-clipboard`, `rofi-keybinds`, `rofi-network`, `screen-time`.
- [x] Mỗi package cài binary + provider + layout fallback.
- [x] Wrap runtime dependency từng app.
- [x] Thêm `app.toml` cho từng app.
- [x] `rofi-hub-apps` chỉ còn compatibility bundle.

**Exit:** `nix build .#rofi-calendar` không kéo app khác/hub.

### M3 — Nix modules & dependency ownership

- [x] Thêm per-app NixOS/Home Manager modules.
- [x] Bỏ `runtimePackagesFor` khỏi hub module.
- [x] Bỏ `screen-time.service` khỏi hub module.
- [x] Bỏ `home.packages = [ package ] ++ runtimePackagesFor pkgs`.
- [x] Menu optional module.
- [x] Deprecated alias `services.rofi-hub.enable` nếu cần.

**Exit:** `services.rofi-apps.calendar.enable = true` chỉ cài calendar + rofi, không kéo hub hay app khác.

### M4 — Installer generic

- [x] Tạo `index.json` schema + sign.
- [x] `install.sh` đọc index/app.toml; bỏ `KNOWN_APPS`.
- [x] Per-app artifacts.
- [x] State riêng từng app.
- [x] Uninstall/update riêng.
- [x] Bỏ `--nixos`.
- [x] Bỏ ghi theme/hub state.
- [x] Rewrite `test-bundle.sh` dùng `install.sh`, không dùng `install-tree.sh`.

**Exit:** `install.sh --app calendar` trong `$HOME` tạm chỉ cài calendar; không cài hub/rofi/cliphist/theme.

### M5 — Theme decoupling

- [x] Base theme do theme module/`rofi-theme` quản lý.
- [x] App layout do app package quản lý.
- [x] App resolve `ROFI_THEME` / `~/.config/rofi/theme.rasi` / app override.
- [x] Bỏ `rofi-hub theme` khỏi installer.
- [x] Menu có theme riêng; không inject.
- [x] Screen-time bỏ đọc `ROFI_HUB_THEME*`.

**Exit:** xóa `~/.config/rofi-hub` và `rofi-hub` binary; app vẫn đúng theme.

### M6 — Network extraction

- [x] Tạo `crates/rofi-network`.
- [x] Thêm `providers/network.toml`.
- [x] Thêm package/provider/module/install metadata.
- [x] Xóa `network.rs` khỏi hub.
- [x] Hoặc chốt network là hub-only và ghi rõ lý do. _(không áp dụng — đã chốt tách `rofi-network`)_

**Exit:** `rofi-network` cài/chạy độc lập; hub không cần cho network.

### M7 — Cleanup đã thực hiện khi tạm dừng các kênh khác

- [x] Bỏ `.deb`, `.rpm`, `.apk`, AUR, Debian official, Gentoo, openSUSE, Void, Slackware, ALT.
- [x] Xóa `packaging/{debian,fedora,arch,alpine,gentoo,opensuse,void,slackbuilds,alt}`, `nfpm.yaml`, `install-tree.sh`, `MANIFEST`, openrc/runit.
- [x] Xóa `scripts/build-package-artifacts.sh`, `package-source.sh`, `update-source-checksums.py`, `test-package-artifacts.sh`, `test-install-tree.sh`, `check-packaging-sync.py`.
- [x] CI bỏ job `install-tree`, `debian-official`, `distro-build`, `artifacts`, `apk-package`, `release-smoke`.
- [x] CI giữ `lint`, `test`, `msrv`, `installer`, `bundle`, `nix`, `nix-arm`, `release`.
- [x] Docs chỉ còn Nix + `install.sh`.

**Exit:** release của installer chứa các tarball theo app và kiến trúc, index, script cài/gỡ, public key, checksum và chữ ký.

### M8 — Validation & release gate

- [x] `scripts/check-deps.py` pass.
- [x] `scripts/test-apps.sh` pass.
- [x] `scripts/test-installer.sh` pass.
- [x] `scripts/test-bundle.sh` pass.
- [x] `scripts/test-theme.sh` pass.
- [x] `scripts/test-migration.sh` pass.
- [x] `scripts/test-release-index.sh` pass.
- [x] `scripts/test-install-matrix.sh` pass (Debian/Ubuntu/Arch/Fedora).
- [x] `scripts/test-nix.sh` pass.
- [x] `nix flake check` pass.
- [x] NIX-07: native build từng app trên x86_64-linux và aarch64-linux. CI run [45865366745](https://github.com/WillGle/rofi-hub/actions/runs/45865366745) job `NIX-07 native aarch64` pass 7m17s, build đủ `rofi-calendar`, `rofi-clipboard`, `rofi-keybinds`, `rofi-network`, `screen-time`, `rofi-hub-menu` trên `ubuntu-24.04-arm`.
- [x] `install.sh --app calendar` trong HOME tạm.
- [x] Gỡ hub/menu → app vẫn chạy.
- [x] `ROFI_THEME` và app override hoạt động.
- [x] Không còn `ROFI_HUB_*` trong app path.
- [x] Kiểm tra chữ ký index/tarball và từ chối artifact thiếu chữ ký trong fixture.
- [ ] Kiểm tra release đã ghép đủ hai kiến trúc: trên mỗi host, installer mặc định chọn đúng tarball cho từng app, xác minh chữ ký và chạy được binary đã cài. _(Đã có merge index + test chọn aarch64/x86_64; còn cài thật trên host ARM)_
- [x] Không còn distro artifact.
- [ ] Toàn bộ DoD 11.1–11.13 pass. _(CI run 45867313267 pass; CON-05 đã đóng; còn signed release key)_

**Exit:** release candidate ký đầy đủ; test matrix xanh; không còn P0; rollback đã test.

## 8. Testing strategy

### 8.1 Nguyên tắc test

- Test phải chứng minh **app chạy độc lập**, không chỉ test hub.
- Test phải chứng minh **installer không cài dependency**.
- Test phải chứng minh **Nix per-app không kéo hub/app khác**.
- Test phải chứng minh **theme precedence** và không cần `ROFI_HUB_*`.
- Test phải có **negative/failure injection**, không chỉ happy path.
- Test phải chạy được trong CI, không cần GUI thật.
- Test phải tái lập được bằng temp `HOME`/`XDG_*`, không đụng máy thật.

### 8.2 Test layers

| Layer | Phạm vi | Công cụ | Gate |
|---|---|---|---|
| Unit | parser, theme resolver, manifest, path, args | `cargo test` | mỗi PR |
| Contract | app ↔ fake `rofi` | fake-rofi fixture | mỗi PR |
| Installer | install/update/uninstall/failure | shell + temp HOME | mỗi PR |
| Nix | package/module/closure | `nix flake check`, `nix build` | mỗi PR |
| Integration | app + theme + service + menu | script end-to-end | mỗi PR |
| Release | index/tarball/signature | `test-bundle.sh` | tag/release |
| Migration | old hub → per-app | fixture cũ | trước release |
| Cross-distro | `install.sh` trên container | Debian/Ubuntu/Arch/Fedora | trước release |
| Security | signature, path traversal, permissions | test installer | mỗi PR |
| Performance | startup/theme cache | fake timer | trước release |

### 8.3 Fixtures bắt buộc

```text
tests/fixtures/
  fake-rofi                 # ghi argv/env, trả selection/cancel/custom
  fake-systemctl            # enable/disable/status
  fake-hyprctl               # monitors -j
  fake-cliphist              # list/store/decode
  fake-wl-copy               # copy
  fake-nmcli                 # device/wifi/connection
  fake-notify-send
  palette.json               # matugen fixture
  theme-base.rasi
  theme-app-override.rasi
  old-hub-install/           # fixture migration
  index-valid.json
  index-bad-signature.json
  index-bad-schema.json
```

Fake `rofi` phải:

- ghi lại `argv`, `stdin`, `env` vào file;
- hỗ trợ chế độ select/cancel/custom key;
- không mở cửa sổ;
- trả exit code theo kịch bản.

### 8.4 Test matrix

#### Dependency & architecture

| ID | Scope | Setup | Assert |
|---|---|---|---|
| DEP-01 | App không depend hub | `cargo tree -p rofi-calendar` | không có `rofi-hub` |
| DEP-02 | Installer không depend app | package metadata / `cargo tree` nếu installer là Rust | không có app crate |
| DEP-03 | Không còn env hub | `rg 'ROFI_HUB_' crates/` | không có trong app path |
| DEP-04 | Provider thuộc app | build từng app package | provider nằm trong app package |
| DEP-05 | Hub không cài dependency | `nix path-info -rSh` app package | không có rofi plugins/cliphist do hub kéo |

#### App runtime

| ID | Scope | Setup | Assert |
|---|---|---|---|
| APP-01 | Direct launch | fake `rofi`, `HOME` tạm | app chạy, `rofi` được gọi |
| APP-02 | Không cài hub | xóa `rofi-hub`/`rofi-hub-menu` | app vẫn chạy |
| APP-03 | Thiếu `rofi` | `PATH` không có rofi | lỗi rõ ràng, exit != 0 |
| APP-04 | Thiếu runtime dep | clipboard thiếu `cliphist` | lỗi rõ ràng, không panic |
| APP-05 | Cancel/custom key | fake rofi trả cancel/custom | app xử lý đúng outcome |
| APP-06 | Không cần env hub | unset toàn bộ `ROFI_HUB_*` | app chạy bình thường |

#### Theme

| ID | Scope | Setup | Assert |
|---|---|---|---|
| THM-01 | User app override | `~/.config/rofi/calendar.rasi` | file này thắng |
| THM-02 | Global override | `ROFI_THEME=/tmp/theme.rasi` | mọi app dùng theme đó |
| THM-03 | Base + layout | `~/.config/rofi/theme.rasi` | app layout vẫn đúng |
| THM-04 | Packaged fallback | không có user theme | dùng fallback app package |
| THM-05 | Menu độc lập | đổi theme menu | app theme không đổi |
| THM-06 | Screen-time độc lập | đổi theme screen-time | app khác không đổi |
| THM-07 | Theme cache | chạy app 2 lần | lần 2 không ghi lại nếu không đổi |
| THM-08 | Không có hub state | xóa `~/.config/rofi-hub` | app vẫn đúng theme |

#### Installer

| ID | Scope | Setup | Assert |
|---|---|---|---|
| INS-01 | Cài một app | `HOME` tạm, `--app calendar` | chỉ calendar + provider được cài |
| INS-02 | Cài tất cả | `--all` | tất cả app được cài, không cài dep |
| INS-03 | Không service | `--app screen-time --no-service` | không có unit enabled |
| INS-04 | Thiếu prerequisite | không có `rofi` | abort, không partial install |
| INS-05 | Sai checksum | sửa byte tarball | abort, không cài |
| INS-06 | Sai signature | index/tarball sai | abort mặc định |
| INS-07 | Interrupt | kill giữa install | state cũ còn nguyên, không half-install |
| INS-08 | Update một app | cài 2 app, update 1 | app còn lại không đổi |
| INS-09 | Uninstall một app | cài 2 app, gỡ 1 | app còn lại chạy |
| INS-10 | App lạ | `--app unknown` | lỗi rõ ràng, không cài |
| INS-11 | Index sai schema | index thiếu field | lỗi rõ ràng, không cài |
| INS-12 | Không cài dependency | snapshot `PATH`/prefix | không có rofi/plugin/cliphist mới |
| INS-13 | Quyền state | sau cài | state 0700, file 0600 |
| INS-14 | Service app-owned | fake `systemctl` | unit lấy từ app package |

#### Nix

| ID | Scope | Setup | Assert |
|---|---|---|---|
| NIX-01 | Build app riêng | `nix build .#rofi-calendar` | closure không có hub/app khác |
| NIX-02 | Module một app | `services.rofi-apps.calendar.enable` | chỉ calendar + deps |
| NIX-03 | Không runtimePackagesFor | eval module | không cài rofi plugins toàn cục |
| NIX-04 | Screen-time service | eval module | service chỉ thuộc screen-time |
| NIX-05 | Menu optional | không bật menu | app vẫn build/chạy |
| NIX-06 | Flake check | `nix flake check` | pass |
| NIX-07 | Nix native x86_64-linux và aarch64-linux | chạy `nix build` cho cả sáu package trên mỗi kiến trúc | đủ log build thành công |

#### Consumer integration (`/etc/nixos`)

| ID | Scope | Setup | Assert |
|---|---|---|---|
| CON-01 | Flake input mới | `/etc/nixos` build | eval/build pass |
| CON-02 | Chỉ bật một app | `services.rofi-apps.calendar.enable` | không cài hub/all-in-one |
| CON-03 | Alias cũ | `services.rofi-hub.enable` | deprecated warning hoặc map đúng |
| CON-04 | Screen-time + theme | bật screen-time, theme module | service/app theme đúng, không env hub |
| CON-05 | Rollback generation | switch lại generation cũ | hệ thống cũ hoạt động |

#### Menu

| ID | Scope | Setup | Assert |
|---|---|---|---|
| MENU-01 | Provider discovery | cài 3 app | list đủ provider |
| MENU-02 | Clean env | launch app qua menu | không có `ROFI_HUB_*` |
| MENU-03 | App thiếu | xóa binary | menu báo lỗi, không crash |
| MENU-04 | Menu theme | đổi theme menu | app theme không đổi |
| MENU-05 | Gỡ menu | xóa menu package | app vẫn chạy |

#### Release & security

| ID | Scope | Setup | Assert |
|---|---|---|---|
| REL-01 | Index schema | `index.json` | schema hợp lệ, version khớp |
| REL-02 | Per-app tarball | giải nén | chỉ chứa app payload |
| REL-03 | Provider version | build | khớp app version |
| REL-04 | Không distro artifact | release dir | không có `.deb/.rpm/.apk` |
| REL-05 | aarch64 bundle | build | cài và chạy được |
| SEC-01 | Unsigned | không `--allow-unsigned` | reject |
| SEC-02 | Path traversal | tarball có `../` | reject |
| SEC-03 | Shell injection | manifest `exec` có shell metachar | không chạy qua shell |
| SEC-04 | Permission | state/theme files | 0700/0600 |

#### Migration & cross-distro

| ID | Scope | Setup | Assert |
|---|---|---|---|
| MIG-01 | Old hub → per-app | fixture old install | app được migrate, không mất state |
| MIG-02 | Gỡ old hub | xóa hub | app vẫn chạy |
| MIG-03 | Provider path cũ | manifest path cũ | vẫn discover |
| DIST-01 | Debian systemd | container | `install.sh --app calendar` pass |
| DIST-02 | Arch systemd | container | pass |
| DIST-03 | Fedora systemd | container | pass |
| DIST-04 | Non-systemd | `--no-service` | pass, không unit |
| DIST-05 | Ubuntu minimal | container | pass |

### 8.5 CI gates

```text
PR:
  fmt
  clippy
  unit
  app contract
  installer integration
  dependency guard
  nix flake check
  nix build per app

Main:
  PR gates
  integration suite
  cross-distro install.sh smoke
  security/failure injection

Tag:
  full test matrix
  aarch64 bundle
  release artifact verification
  migration fixture
  signed index/tarball
```

### 8.6 Test scripts cần có

| Script | Trách nhiệm |
|---|---|
| `scripts/check-deps.py` | guard dependency graph |
| `scripts/test-apps.sh` | app runtime + fake rofi |
| `scripts/test-installer.sh` | installer happy/failure |
| `scripts/test-bundle.sh` | tarball/index/signature |
| `scripts/test-theme.sh` | theme precedence/isolation |
| `scripts/test-migration.sh` | old hub → per-app |
| `scripts/test-release-index.sh` | schema/version/signature |
| `scripts/test-install-matrix.sh` | container install.sh smoke |
| `scripts/test-nix.sh` | per-app build/module/closure |

## 9. Integration scenarios

### I-01 — Cài một app trên máy sạch bằng `install.sh`

```text
fresh HOME
  → install.sh --app calendar
  → verify:
      ~/.local/bin/rofi-calendar exists
      provider calendar.toml exists
      không có rofi-hub
      không có rofi/plugin/cliphist mới
  → fake rofi chạy rofi-calendar
```

**Pass:** calendar chạy; hub không tồn tại; dependency không bị cài.

### I-02 — Cài một app bằng Nix

```text
nix build .#rofi-calendar
  → closure check
  → chạy binary với fake rofi
```

**Pass:** closure không có hub/all-in-one/app khác; binary chạy.

### I-03 — Gỡ hub, app vẫn chạy

```text
cài calendar + clipboard
  → xóa rofi-hub/rofi-hub-menu
  → chạy calendar/clipboard
```

**Pass:** cả hai chạy; provider list của menu không còn nhưng app không quan tâm.

### I-04 — Menu optional

```text
cài calendar + menu
  → menu list calendar
  → menu launch calendar với env sạch
  → gỡ menu
  → calendar vẫn chạy
```

**Pass:** menu không nằm trong dependency closure; không inject env.

### I-05 — Theme

```text
base theme = theme-a
app override = calendar theme-b
ROFI_THEME unset
  → calendar dùng theme-b
  → clipboard dùng theme-a

ROFI_THEME=theme-c
  → app không có override dùng theme-c

xóa ~/.config/rofi-hub
  → kết quả không đổi
```

**Pass:** precedence đúng; hub không tham gia.

### I-06 — Screen-time service

```text
install.sh --app screen-time
  → unit từ screen-time package
  → fake systemctl enable
  → --no-service không enable
```

**Pass:** unit thuộc app, không thuộc hub; uninstall screen-time không đụng app khác.

### I-07 — Update độc lập

```text
cài calendar 1.2.0 + clipboard 0.8.0
  → update calendar 1.3.0
  → clipboard vẫn 0.8.0
  → hub/installer version không đổi
```

**Pass:** chỉ calendar đổi; state app khác nguyên.

### I-08 — Migration từ hub cũ

```text
fixture old install (hub + 3 app + provider dir cũ)
  → chạy installer mới --app calendar
  → đọc provider path cũ
  → app vẫn chạy
  → gỡ hub cũ
```

**Pass:** không mất config/state; app vẫn chạy; provider cũ vẫn đọc được trong giai đoạn chuyển tiếp.

### I-09 — Release verification

```text
build index + per-app tarball
  → ký
  → verify checksum/signature
  → cài từ index
  → chạy app
```

**Pass:** unsigned bị reject; sai checksum bị reject; app cài đúng version.

### I-10 — Cross-distro `install.sh`

```text
Debian/Ubuntu/Arch/Fedora container
  → install.sh --app calendar --no-service (hoặc systemd)
  → fake rofi
```

**Pass:** không cần package distro; chỉ cần prerequisite.

### I-11 — Consumer NixOS (`/etc/nixos`)

```text
/etc/nixos đổi input rofi-hub sang bản mới
  → bật services.rofi-apps.calendar
  → nixos-rebuild build
  → verify:
      chỉ rofi-calendar + rofi được cài
      rofi-hub/rofi-hub-menu không bắt buộc
      theme module vẫn tạo ~/.config/rofi/theme.rasi
      screen-time service chỉ khi bật screen-time
  → rollback generation cũ
```

**Pass:** build pass; app chạy; không kéo hub/all-in-one; rollback an toàn.

## 10. Acceptance criteria

Kiến trúc chỉ được coi là hoàn thành khi:

| # | Tiêu chí | Test ID | Evidence |
|---|---|---|---|
| 1 | Gỡ hub/menu → app vẫn chạy | APP-02, MENU-05, MIG-02 | log chạy app sau khi xóa hub |
| 2 | Nix build app không kéo hub/app khác | NIX-01, NIX-02 | `nix path-info -rSh` output |
| 3 | `install.sh --app` không cài dependency | INS-01, INS-12 | snapshot prefix + `PATH` |
| 4 | App crate không depend hub | DEP-01, DEP-02 | `cargo tree` output |
| 5 | Provider thuộc app package | DEP-04, REL-02 | nội dung tarball |
| 6 | Update/uninstall độc lập | INS-08, INS-09 | state trước/sau |
| 7 | Không cần `ROFI_HUB_*` | DEP-03, APP-06 | log env sạch |
| 8 | Theme precedence đúng | THM-01..08 | output fake rofi |
| 9 | Không còn distro artifact | REL-04 | danh sách release |
| 10 | Signature/checksum bắt buộc | SEC-01..04 | log reject |
| 11 | `install.sh` chạy trên distro khác | DIST-01..05 | CI matrix log |
| 12 | Screen-time service thuộc app | NIX-04, INS-14 | unit path/content |
| 13 | Migration old hub → per-app | MIG-01..03 | fixture log |
| 14 | CI chỉ còn Nix + installer + bundle | CI gate | workflow diff |
| 15 | Docs chỉ còn hai kênh | docs review | diff |
| 16 | Consumer `/etc/nixos` build/rollback | CON-01..05 | build log + generation switch |

## 11. Definition of Done (chi tiết)

Một phase chỉ được đánh dấu Done khi tất cả mục trong nhóm tương ứng pass, có evidence, và không còn TODO/P0.

### 11.1 Kiến trúc & dependency

- [x] App crate không depend `rofi-hub`, `rofi-hub-installer`, `rofi-hub-menu`.
- [x] Installer không depend app crate.
- [x] Menu không nằm trong dependency closure của app.
- [x] `rofi-dmenu` là library versioned; app pin version.
- [x] Không còn `runtimePackagesFor` toàn cục.
- [x] Không còn `ROFI_HUB_THEME`, `ROFI_HUB_THEME_FILE`, `ROFI_HUB_PROVIDER`.
- [x] Provider manifest nằm trong app package.
- [x] `network` đã tách thành app hoặc được ghi rõ là hub-only exception.
- [x] Không còn app logic trong hub installer.

**Evidence:** `cargo tree`, `rg`, `nix path-info -rSh`, package contents.

### 11.2 Code quality

- [x] `cargo fmt --check --all` pass.
- [x] `cargo clippy --workspace --all-targets -- -D warnings` pass.
- [x] `cargo test --workspace --locked` pass.
- [x] MSRV 1.89: `cargo check --workspace --locked --all-targets` pass.
- [x] Shell scripts pass `shellcheck` và `bash -n`.
- [x] Không có dependency thừa (ví dụ `serde_json` trong `rofi-keybinds` nếu không dùng).
- [x] Feature-gate `manifest`, `notify`, `category` được đo trước/sau.
- [x] Không còn code path đọc state theme của hub.

**Evidence:** CI lint/test logs; `cargo tree`; `cargo bloat`/size report nếu có.

### 11.3 Testing

- [x] Tất cả test ID `DEP-*`, `APP-*`, `THM-*`, `INS-*`, `NIX-*`, `CON-*`, `MENU-*`, `REL-*`, `SEC-*`, `MIG-*`, `DIST-*` pass. _(local pass; CI run 45867313267 pass; CON-05 đã verify trên máy thật ngày 2026-11-27)_
- [x] Có test negative/failure injection, không chỉ happy path.
- [x] Có test app chạy khi không có hub/menu.
- [x] Có test installer không cài dependency.
- [x] Có test theme precedence và isolation.
- [x] Có test migration từ hub cũ.
- [x] Có test cross-distro `install.sh`.
- [x] Có test aarch64 bundle. _(CI run 45850184054: job `bundle (aarch64)` pass trên native arm64)_
- [x] Test chạy trong temp HOME/XDG, không đụng máy thật.
- [x] Không có test nào phụ thuộc GUI thật.

**Evidence:** test report, CI artifacts, log từng suite.

### 11.4 Nix

- [x] `nix flake check` pass.
- [x] `nix build .#rofi-calendar` pass.
- [x] `nix build .#rofi-clipboard` pass.
- [x] `nix build .#rofi-keybinds` pass.
- [x] `nix build .#rofi-network` pass.
- [x] `nix build .#screen-time` pass.
- [x] `nix build .#rofi-hub-menu` pass (nếu giữ menu).
- [x] Per-app module chỉ cài app + dependency của app.
- [x] Screen-time module sở hữu service; hub module không.
- [x] `apps = []`/không bật menu không kéo menu.
- [x] Closure app không chứa hub/plugin/cliphist ngoài dependency app.
- [x] Hỗ trợ x86_64 và aarch64.
- [x] `/etc/nixos` build pass với per-app module.
- [x] Rollback generation cũ pass (CON-05). _(generation 1042 = closure cũ `n4lbs…` đang active; `rofi-hub 0.2.1 doctor` → `status ok`, 4 provider ok, `screen-time.service` active; generation 1041 có `rofi-network`/`rofi-hub-menu`, không có `rofi-hub` — log 2026-11-27)_

**Evidence:** `nix path-info -rSh`; eval module; CI logs.

### 11.5 install.sh

- [x] Đọc `index.json` + `app.toml`; không hardcode `KNOWN_APPS`.
- [x] `--list`, `--app`, `--all`, `--update`, `--uninstall` hoạt động.
- [x] Không có `--nixos`.
- [x] Không cài `rofi`, plugin, `cliphist`, `wl-clipboard`, `networkmanager`.
- [x] Chỉ kiểm tra prerequisite bằng `command -v`.
- [x] Verify checksum + signature mặc định.
- [x] `--allow-unsigned` chỉ cho test/local.
- [x] State riêng từng app; update/uninstall độc lập.
- [x] Không ghi theme/config user.
- [x] Không set `ROFI_HUB_*`.
- [x] Không wrap PATH của app.
- [x] Service/unit do app ship; `--no-service` hoạt động.
- [x] Refuse Nix-managed path.
- [x] Interrupt/rollback không để half-install.
- [x] Permission state 0700/file 0600.
- [x] Path traversal bị reject.
- [x] `test-bundle.sh` dùng `install.sh`, không dùng `install-tree.sh`.

**Evidence:** temp HOME log; checksum/signature failure log; diff prefix; state permissions.

### 11.6 Theme

- [x] Base theme do user/theme module quản lý.
- [x] App layout do app package/code quản lý.
- [x] `~/.config/rofi/<app>.rasi` thắng base.
- [x] `ROFI_THEME` hoạt động như full global override.
- [x] `~/.config/rofi/theme.rasi` + app layout hoạt động.
- [x] Packaged fallback hoạt động khi không có user theme.
- [x] Menu theme không ảnh hưởng app.
- [x] Screen-time theme không ảnh hưởng app khác.
- [x] Không cần `~/.config/rofi-hub`.
- [x] Theme cache không ghi lại nếu không đổi.
- [x] App direct launch và menu launch cho cùng theme.

**Evidence:** fake-rofi argv/env; theme file diff; test THM-*.

### 11.7 Menu & screen-time

- [x] Menu optional; gỡ menu app vẫn chạy.
- [x] Menu không inject env/theme.
- [x] Menu list provider đúng.
- [x] Menu xử lý app thiếu binary rõ ràng.
- [x] Screen-time unit thuộc app package.
- [x] Screen-time service không do hub module bật.
- [x] Screen-time uninstall không đụng app khác.
- [x] Screen-time theme state app-owned.

**Evidence:** menu integration log; unit path; systemctl fake.

### 11.8 Release & security

- [x] `index.json` có schema version.
- [x] Per-app tarball chỉ chứa app payload.
- [x] Provider version khớp app version.
- [x] Checksum + signature cho index/tarball.
- [x] Unsigned bị reject mặc định.
- [x] Không còn `.deb/.rpm/.apk/AUR/...`.
- [x] Không còn script build distro package.
- [x] Release aarch64 pass. _(CI run 45850184054: `bundle (aarch64)` + bundle check pass; job `release` skipped vì push không tag)_
- [x] Path traversal/control char trong manifest bị reject.
- [x] `exec` là argv, không shell interpolation.

**Evidence:** release dir listing; signature log; security test.

### 11.9 CI

- [x] `lint`, `test`, `msrv`, `installer`, `bundle`, `nix`, `release` là required. _(Đã cấu hình branch protection `main` ngày 2026-11-26 với `lint`, `test`, `MSRV (1.89)`, `installer`, `bundle (x86_64)`, `bundle (aarch64)`, `cross-distro`, `nix`, `NIX-07 native aarch64`; `release` là job tag-only nên chạy khi tag, không phải check của pull request)_
- [x] Không còn `install-tree`, `debian-official`, `distro-build`, `artifacts`, `apk-package`, `release-smoke`.
- [x] `check-deps.py` chạy trong CI.
- [x] `test-apps.sh`, `test-theme.sh`, `test-migration.sh` chạy trong CI.
- [x] Cross-distro installer smoke chạy trước release.
- [x] aarch64 bundle chạy trước release.
- [x] CI không build distro package.
- [x] Artifact retention và checksum được kiểm tra.

**Evidence:** workflow diff; CI run; required checks.

### 11.10 Docs & migration

- [x] `README.md` chỉ còn Nix + `install.sh`.
- [x] `docs/ARCHITECTURE.md` mô tả kiến trúc mới.
- [x] `docs/PACKAGING.md` chỉ còn release/install paths.
- [x] `docs/MIGRATING.md` có hướng dẫn old hub → per-app.
- [x] `docs/UI_STYLE.md` bỏ `test-install-tree.sh`.
- [x] `docs/screenshots/ui/README.md` dùng `install.sh`.
- [x] Không còn claim `.deb/.rpm/.apk` support.
- [x] `goal.md` được cập nhật nếu quyết định thay đổi.
- [x] Deprecation warning cho alias cũ.
- [x] Migration fixture pass.

**Evidence:** docs diff; migration log.

### 11.11 Performance & operations

- [x] Baseline startup từng app được ghi lại; không regression >20%.
- [x] Theme cache hit không ghi file.
- [x] Installer một app không scan/copy app khác.
- [x] Installer không cài dependency ngoài ý muốn.
- [x] App không spawn hub/menu.
- [x] Không có process hub thường trú.
- [x] Screen-time daemon vẫn chạy độc lập.
- [x] Log lỗi đủ rõ cho missing prerequisite/signature.

**Evidence:** benchmark log; `strace`/fs snapshot; process list.

### 11.12 Phase DoD map

| Phase | DoD bắt buộc | Test chính |
|---|---|---|
| M1 | 11.1, 11.2, 11.3 | DEP-01..03, THM-08 |
| M2 | 11.1, 11.4 | NIX-01..05, DEP-04 |
| M3 | 11.4, 11.9 | NIX-02..07, CON-01..05 |
| M4 | 11.5, 11.8, 11.9 | INS-01..14, REL-01..05, SEC-01..04 |
| M5 | 11.6, 11.7 | THM-01..08, MENU-04..05 |
| M6 | 11.1, 11.4, 11.5 | APP-01, DEP-01, NIX-01 |
| M7 | 11.8, 11.9, 11.10 | REL-04, CI gate |
| M8 | 11.1–11.13 | toàn bộ matrix |

### 11.13 Exit gate

Phase/PR chỉ Done khi:

- [ ] Tất cả mục trong nhóm liên quan đã check. _(còn các mục external bên dưới)_
- [x] Test ID tương ứng pass trong CI. _(CI run 45867313267 pass các job test; CON-05 đã verify manual trên máy thật)_
- [x] Không còn TODO/P0 trong scope.
- [x] Có evidence đính kèm PR/release. _(PR [#1](https://github.com/WillGle/rofi-hub/pull/1), CI run [45867313267](https://github.com/WillGle/rofi-hub/actions/runs/45867313267) pass toàn bộ job trừ `release` tag-only)_
- [x] Không làm hỏng app khác.
- [x] `goal.md` không mâu thuẫn với implementation.
- [x] Nếu thay đổi quyết định kiến trúc, cập nhật `goal.md` trước khi merge.

## 12. Validation commands

```bash
# 1. Static checks
cargo fmt --check --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --locked
python3 scripts/check-deps.py

# 2. Nix per-app
for app in rofi-calendar rofi-clipboard rofi-keybinds rofi-network screen-time; do
    nix build ".#${app}"
done
nix flake check

# 3. Dependency guard
if cargo tree -p rofi-calendar | grep -q 'rofi-hub'; then
    echo "rofi-calendar depends on rofi-hub" >&2
    exit 1
fi

# 4. App + theme
scripts/test-apps.sh
scripts/test-theme.sh

# 5. Installer/release
scripts/test-installer.sh
scripts/test-bundle.sh
scripts/test-release-index.sh

# 6. Migration
scripts/test-migration.sh

# 7. Cross-distro install.sh (nếu chốt matrix)
scripts/test-install-matrix.sh

# 8. Independence gate: test suite phải có case
#    rm -rf "$HOME/.local/bin/rofi-hub" "$HOME/.local/share/rofi-hub"
#    fake-rofi "$HOME/.local/bin/rofi-calendar"
#    fake-rofi "$HOME/.local/bin/rofi-clipboard"
```

## 13. Các câu hỏi thiết kế ban đầu (quyết định tại §16)

1. **Menu**: giữ `rofi-hub-menu` optional hay bỏ hoàn toàn?
2. **`rofi-dmenu`**: giữ trong monorepo nhưng version độc lập, hay tách repo?
3. **Artifact**: per-app tarball (khuyến nghị) hay một universal tarball?
4. **Theme tool**: tách `rofi-theme` độc lập, hay chỉ dùng user config?
5. **Registry path**: giữ `rofi-hub` hay đổi `rofi-apps`?
6. **Backward compatibility**: giữ alias trong bao lâu?
7. **Installer CLI**: shell-only hay giữ Rust CLI cho `list/status`?
8. **Cross-distro smoke**: Debian/Ubuntu/Arch/Fedora (đã chốt).
9. **Performance budget**: chốt ngưỡng startup/theme cache sau baseline.
10. **`install.sh --theme`**: bỏ hẳn hay chuyển sang optional `rofi-theme`.
11. **Signing key rotation**: overlap window và key pinning cho index.

## 14. Risks

| Risk | Mitigation |
|---|---|
| Shared library vẫn coupling build | semver riêng, guard `cargo tree`, tách repo nếu cần |
| Per-app release phức tạp | bắt đầu bằng index + per-app tarball; CI tự động |
| Theme regression khi bỏ hub injection | test precedence; app override; acceptance criteria |
| Installer generic bị lỗi metadata | schema version, signature, validation trước khi cài |
| User migration đau | alias tạm thời, migration doc, deprecation warning |
| App dependency drift | `app.toml` + Nix wrapper + CI dependency test |

## 15. File impact map

### Thêm

```text
goal.md
installer/index.schema.json
crates/rofi-network/...
crates/rofi-hub-menu/...              # nếu giữ menu
crates/<app>/app.toml
scripts/check-deps.py
scripts/test-apps.sh
scripts/test-theme.sh
scripts/test-migration.sh
scripts/test-release-index.sh
scripts/test-install-matrix.sh
scripts/test-nix.sh
tests/fixtures/...
```

### Sửa

```text
Cargo.toml
Cargo.lock
crates/rofi-core/...                  # library độc lập, feature-gate
crates/rofi-hub/...                   # chỉ còn menu/installer nếu giữ
crates/<app>/Cargo.toml
flake.nix
install.sh
providers/*.toml                      # move vào app package
scripts/check-app-sync.py
scripts/package-release.sh
scripts/test-bundle.sh
scripts/test-installer.sh
.github/workflows/release.yml
README.md
docs/ARCHITECTURE.md
docs/PACKAGING.md
docs/MIGRATING.md
docs/UI_STYLE.md
docs/screenshots/ui/README.md
```

### Xóa

```text
packaging/debian/
packaging/fedora/
packaging/arch/
packaging/alpine/
packaging/gentoo/
packaging/opensuse/
packaging/void/
packaging/slackbuilds/
packaging/alt/
packaging/nfpm.yaml
packaging/install-tree.sh
packaging/MANIFEST
packaging/openrc/
packaging/runit/
packaging/systemd/screen-time.service.in
packaging/rofi-hub.desktop
packaging/rofi-hub.svg
packaging/rofi-hub.metainfo.xml
packaging/man/
packaging/completions/       # xóa nếu install.sh không cài
packaging/README.md
scripts/build-package-artifacts.sh
scripts/package-source.sh
scripts/update-source-checksums.py
scripts/test-package-artifacts.sh
scripts/test-install-tree.sh
scripts/check-packaging-sync.py
```

### Consumer `/etc/nixos` (repo riêng)

- Cập nhật flake input `rofi-hub`.
- Chuyển `services.rofi-hub.enable` sang `services.rofi-apps.<app>.enable`.
- Bỏ `runtimePackagesFor` và dependency app khỏi module hub.
- Giữ `modules/desktop/theme` làm base theme.
- Bỏ per-app rasi không còn dùng hoặc chuyển thành user override.
- Cập nhật `README.md`/`modules/README.md` của `/etc/nixos`.
- Kiểm tra rollback generation.

---

**Nguyên tắc cuối:** nếu một thay đổi ở hub/installer/menu khiến app phải rebuild, đổi dependency, đổi theme, hoặc đổi cách chạy — thay đổi đó đang vi phạm goal này.

---

## 16. Implementation decisions (2026-11-26)

Các quyết định mở ở §13 đã chốt cho implementation này:

1. **Menu**: giữ `rofi-hub-menu` optional; `--all` bỏ qua menu (`optional = true` trong index).
2. **`rofi-dmenu`**: giữ trong monorepo, version độc lập `0.3.0`, app pin qua path + semver.
3. **Artifact**: per-app tarball + `index.json`; aarch64 dùng cùng script với `ARCH=aarch64`.
4. **Theme tool**: không tách `rofi-theme`; base theme thuộc user/theme module, menu có state riêng.
5. **Registry path**: dùng `rofi-apps`; menu đọc thêm path cũ `rofi-hub` trong giai đoạn chuyển tiếp.
6. **Backward compatibility**: giữ `services.rofi-hub.enable` và `homeManagerModules.rofi-hub`
   làm alias deprecated; không giữ `ROFI_HUB_*` trong app path.
7. **Installer CLI**: shell-only; không có Rust CLI cho `list/status`.
8. **Cross-distro smoke**: Debian, Ubuntu, Arch, Fedora container với payload shell (không phụ thuộc glibc).
9. **Performance budget**: theme cache so sánh nội dung, không ghi lại; không có process hub thường trú.
10. **`install.sh --theme`**: bỏ hẳn; theme do user/theme module quản lý.
11. **Signing**: minisign; `index.json.minisig` và `<tarball>.minisig` bắt buộc mặc định;
    `--allow-unsigned` chỉ cho test/local.

Các exit criteria của M1–M8 được chứng minh bằng `scripts/check-deps.py`,
`scripts/test-apps.sh`, `scripts/test-theme.sh`, `scripts/test-installer.sh`,
`scripts/test-bundle.sh`, `scripts/test-release-index.sh`,
`scripts/test-migration.sh`, `scripts/test-install-matrix.sh`,
`scripts/test-nix.sh`, và `nix flake check`.

### 16.1 Evidence log (local, 2026-11-26)

- `cargo fmt --check --all` ok; `cargo clippy --workspace --all-targets -- -D warnings` ok;
  `cargo test --workspace --locked` ok (24 result groups).
- MSRV 1.89 chạy thật trên máy này:
  `nix shell github:oxalica/rust-overlay#rust_1_89_0 nixpkgs#gcc -c cargo check --workspace --locked --all-targets`
  → `rustc 1.89.0 (29483883e 2025-08-04)`, `Finished dev profile in 13.11s`.
- Dependency thừa: bỏ `serde_json` khỏi `rofi-keybinds` (không dùng), lock cập nhật, check pass.
- Feature-gate đo trước/sau trên `rofi-keybinds` release:
  category-only 668504 bytes; thêm `manifest`+`notify` 672600 bytes (tiết kiệm ~4KB mỗi app).
- Startup budget (mở §13.9): chốt 400ms/app với fake rofi;
  `scripts/test-performance.sh` — calendar 6ms, clipboard 10ms, keybinds 7ms,
  network 21ms, rofi-screen-time 32ms; cache-hit 6ms và không ghi lại file theme.
- Installer: thêm case từ chối prefix `/nix/store`; `scripts/test-installer.sh`
  70/0 pass. `actionlint` pass trên workflow mới.
- Cross-distro: Debian 12, Ubuntu 24.04, Fedora, Arch pass local
  (`scripts/test-install-matrix.sh` 4/0; Docker Hub token hết hạn nên script
  fallback sang `mirror.gcr.io`).
- Aarch64: cross cargo build toàn workspace (rustc 1.95 cross wrapper + cross gcc):
  cả 6 binary `rofi-calendar`, `rofi-clipboard`, `rofi-keybinds`, `rofi-network`,
  `screen-time`, `rofi-hub-menu` đều là ELF AArch64. Flake exposes
  `packages.x86_64-linux.<app>-aarch64` qua `pkgsCross.aarch64-multiplatform`;
  `nix build .#rofi-calendar-aarch64` còn 161 derivation GTK/Rofi target sau
  ~90 phút nên đã dừng — CI native arm64 là gate cho release. `cargo build
  --target aarch64-unknown-linux-gnu` là cách kiểm tra nhanh binary Rust.
- Consumer `/etc/nixos`: `flake.lock` đã trỏ `rofi-hub` rev `5977985` (refactor
  commit), `nix flake check --no-build` pass. Live switch generation 1041
  (`/nix/store/l2lfaf297kk546wwk6ks7dhpnci106j3-nixos-system-laptop-...`) active;
  đủ 6 app ở generation 1041 (`rofi-calendar`, `rofi-clipboard`, `rofi-keybinds`,
  `rofi-network`, `screen-time`, `rofi-hub-menu`; chưa có `atomic-note`), `rofi-hub` cũ absent; `screen-time` user unit
  active. Generation cũ 1039 vẫn có `rofi-hub`, chưa có `rofi-hub-menu`/`rofi-network`
  — dùng làm mốc rollback CON-05. Link generation 1039 đã bị GC, nhưng closure
  `/nix/store/n4lbs...-nixos-system-think14gryzen-...` vẫn tồn tại và có GC root
  (`/etc/nixos/result`), nên rollback dùng store path trực tiếp.
- CI run [45850184054](https://github.com/WillGle/rofi-hub/actions/runs/45850184054)
  trên `main` (commit `5977985`): **tất cả job pass** — `lint`, `test`,
  `MSRV (1.89)`, `installer`, `cross-distro`, `bundle (x86_64)`,
  `bundle (aarch64)` (native arm64), `nix` (flake check + per-app closures).
  Job `release` skipped vì push `main` không phải tag.
- Branch protection: đã cấu hình required checks trên `main` ngày 2026-11-26:
  `lint`, `test`, `MSRV (1.89)`, `installer`, `bundle (x86_64)`,
  `bundle (aarch64)`, `cross-distro`, `nix`, `NIX-07 native aarch64`; strict
  status checks bật. `release` là job tag-only nên không đưa vào required
  checks của pull request.
- PR [#1](https://github.com/WillGle/rofi-hub/pull/1) / CI run
  [45867313267](https://github.com/WillGle/rofi-hub/actions/runs/45867313267)
  (commit `06a4ed5`): pass `lint`, `test`, `MSRV (1.89)`, `installer`,
  `cross-distro`, `bundle (x86_64)`, `bundle (aarch64)`, `nix` (7m23s) và
  `NIX-07 native aarch64` (7m34s, đủ sáu package). `release` skipped vì PR
  không phải tag.

### 16.2 Việc còn mở trong hai kênh phát hành

1. M0: review/chốt `goal.md` với stakeholder.
2. **CON-05: đã đóng (2026-11-27).** Generation 1042 đang trỏ closure cũ `n4lbs…`; `rofi-hub 0.2.1` chạy được, `rofi-hub doctor` báo `status ok` với 4 provider, `screen-time.service` active; closure mới 1041 có `rofi-network`/`rofi-hub-menu` và không có `rofi-hub`.
3. **Release mới chưa ký.** Secret `MINISIGN_SECRET_KEY` đã có trong GitHub
   Actions (set 2026-11-24); release `v0.2.1` verify được bằng `minisign.pub`,
   nên key thật đã sẵn sàng. Còn merge PR #1, tạo tag `v0.3.0` và chạy job
   `release` để ký/publish; sau đó verify `index.json`, `*.tar.gz`,
   `SHA256SUMS` bằng `minisign -Vm`.
4. CI run: PR #1 / run 45867313267 pass `lint`, `test`, `MSRV`, `installer`,
   `bundle` (x86_64 + aarch64), `cross-distro`, `nix`, `NIX-07 native aarch64`;
   required checks đã cấu hình. `release` skipped vì PR không phải tag.
5. Nix native: **đã có log**. Run 45865366745 job `NIX-07 native aarch64`
   build đủ sáu package trên `ubuntu-24.04-arm` (7m17s) và in closure.
6. `install.sh`: **đã sửa**. `scripts/package-release.sh` xuất
   `index-<arch>.json`; `scripts/merge-release-index.py` gộp hai bundle thành
   `index.json` schema 2 với `url_<arch>`/`sha256_<arch>` cho từng app;
   `install.sh` chọn cặp theo `uname -m` và từ chối kiến trúc không có
   artifact. `scripts/test-release-index.sh` kiểm tra merge + chọn aarch64 +
   lỗi thiếu kiến trúc (10/10 pass). Vẫn cần chạy cài đặt thật trên cả hai
   host trước khi ký phát hành.

## 17. Thiết kế lại giao diện Calendar và Network

Mọi thay đổi UI tuân theo [AGENTS.md](AGENTS.md#required-ui-rules) và [chuẩn UI](docs/UI_STYLE.md). Phần này ghi thiết kế và bằng chứng triển khai của từng app.

**Trạng thái** · Bản redesign đầu tiên có bằng chứng ở §17.5. Điều khiển trực tiếp và đổi theme trong phiên đã được triển khai và kiểm tra bằng Rofi thật. Kết quả ở §17.6.

### 17.1. Mục tiêu và ranh giới

Người dùng chọn Rofi để các mini app nhẹ. Calendar cần giúp mắt tìm một ngày trong tháng. Network cần giúp người dùng nhận biết kết nối hiện tại và chọn mạng từ lần nhìn đầu. Screen Time là tham chiếu cho cách đặt dữ liệu quan trọng vào trung tâm: trục giờ phục vụ việc so sánh thời gian. Lịch có trục tuần riêng, còn bộ chọn mạng cần danh sách dọc.

Áp dụng quyền sở hữu theme và khả năng chạy độc lập trong [goal.md §3.6](goal.md). Dùng palette, font người dùng, màu chọn và cơ chế override hiện hành. Với hai app này, bố cục bên dưới thay cho mẫu header ba dòng và section band trong [UI_STYLE.md](docs/UI_STYLE.md). Screen Time, Hub, Clipboard và Keybinds dùng chung khoảng cách, palette và dấu hiệu chọn hàng với bố cục riêng của mỗi app. Dùng Rust, Rofi dmenu, Pango markup và RASI sẵn có. Chỉ đọc dữ liệu khi mở màn hình hoặc thực hiện thao tác cần nó. Tránh daemon, polling, trình duyệt nhúng và dependency giao diện mới.

**Điều khiển và theme trong phiên (2026-11-27)**

- Dùng Atomic Tasks làm tham chiếu cho thao tác: đặt phím ngay cạnh tên hành động, cho gõ ngay khi mở, hiển thị rõ mục đang chọn. Calendar giữ lưới tháng và ba nút điều hướng gọn ở dưới. Network đặt hai nút Scan và Actions ngoài danh sách lọc. Bấm nút hoặc dùng phím tương ứng đều kích hoạt cùng hành động. Hàng SSID đang chọn có nền tương phản phủ toàn chiều ngang, chữ và số vẫn đọc được ở cả theme sáng lẫn tối.
- Mỗi app mở theo cấu hình theme hiện hành (§3.6). Hiện gợi ý ngắn `Ctrl+1–4 Theme`. Theo cùng thứ tự palette với Screen Time, `Ctrl+1` chọn Nord, `Ctrl+2` Solarized Light, `Ctrl+3` Gruvbox Dark và `Ctrl+4` trở về Auto theo cấu hình người dùng. Áp dụng palette ngay trong phiên. Giữ nguyên font, kích thước cửa sổ, khoảng cách và bố cục của theme đã cấu hình khi đổi màu. Lưu lựa chọn vào state riêng của app tại `~/.local/state/rofi-apps/<app>/theme-choice` (theme lock), để lần mở sau vẫn dùng palette đã chọn; `Ctrl+4` xóa khóa và trở về cấu hình. Các app khác không đọc state này và vẫn chạy độc lập. Calendar, Network, Screen Time và Hub Menu cùng dùng `Ctrl+1–4` cho theme; ba app đầu giữ khóa trong state của mình, Hub giữ trong state menu. Atomic Tasks vẫn dùng `Alt+1–4` cho mức ưu tiên.
- Đổi màu giữ nguyên tác vụ. Calendar giữ tháng, ngày tra cứu, chuỗi đang gõ và thông báo hiện tại. Network giữ màn hình, bộ lọc và SSID đang chọn theo định danh, kể cả khi tên hiển thị bị rút gọn. Chuyển theme không kích hoạt scan, connect hay submit. Phân biệt trạng thái bằng cả hình thức và nội dung: ngày tra có gạch dưới, dòng mạng có nhãn Open/Secured.

### 17.2. Calendar · Đọc tháng rồi nhảy tới ngày

[Ảnh hiện tại](docs/screenshots/ui/calendar-gruvbox.png) cho thấy lưới ngày chỉ dùng một phần bề ngang, trong khi Prev, Today và Next chiếm ba hàng toàn chiều rộng. Đích là một tờ lịch nhỏ, đọc từ trên xuống. Minh họa lấy ngày hệ thống 26/11/2026. Dấu ngoặc vuông minh họa nền của ô hôm nay.

```text
                November 2026
    Mo    Tu    We    Th    Fr    Sa    Su
                                         1
     2     3     4     5     6     7     8
     9    10    11    12    13    14    15
    16    17    18    19    20    21    22
    23    24    25   [26]   27    28    29
    30

    Go to date   DD/MM/YYYY
    [Prev ←]     [Today Ctrl+t]     [Next →]
```

- **Hình học.** Thứ tự widget cho Calendar là message chứa tháng và lưới, inputbar nhập ngày, rồi một hàng ba lựa chọn. Thử khoảng 44–48ch cho cửa sổ ở font mặc định, giới hạn theo màn hình. Căn giữa tiêu đề và cả khối lưới. Bảy cột monospace cùng độ rộng, số căn phải trong ô, tên thứ nằm trên tâm ô. Chữ số ngày ít nhất bằng cỡ chữ nội dung mặc định 12pt. Giữ khoảng cách dọc đều giữa các tuần và lề trái/phải đối xứng. Dùng số tuần thực của tháng, kể cả tháng có sáu tuần.
- **Trọng tâm.** Nền accent và chữ tương phản cho hôm nay. Ngày tìm được dùng gạch dưới và chữ đậm, kèm thứ và ngày đầy đủ dưới lưới. Khi hai ngày trùng nhau, dùng một ô nền accent và vẫn hiển thị kết quả. Sang tháng khác, chỉ tô hôm nay nếu ngày đó thuộc tháng đang xem. Cuối tuần vẫn phải đọc được. Không biến ngày thành 42 mục kết quả có thể bị bộ lọc Rofi xáo trộn.
- **Điều hướng.** Mở ở tháng hiện tại, nhập số được ngay. Giữ Left/Right đổi tháng và Ctrl+t về hôm nay. Ba lựa chọn bên dưới dùng chuột hoặc thao tác chọn của Rofi. Thứ tự nhìn và thứ tự kích hoạt đều là Prev, Today, Next. Selection của nút nhỏ gọn, không phủ chiều ngang cửa sổ. Esc đóng. Khi người dùng đang nhập, Enter xử lý chuỗi ngày kể cả khi bộ lọc làm ẩn các nút.
- **Đầu vào.** Giữ các định dạng và cách suy năm của `parse_query`, gồm ngày trong tháng đang xem, ngày/tháng, tháng/năm, ngày đầy đủ và ISO. Truy vấn chỉ có tháng/năm đưa tới tháng đó. Ví dụ ở 11/2026, nhập `15` làm nổi ngày 15, `3/2027` mở tháng 3/2027. Với `31/02/2027`, giữ tháng hiện tại, hiển thị lỗi một dòng bên cạnh vùng nhập và cho gõ lại. Dùng nhãn `Go to date` và gợi ý định dạng ở ô rỗng. Nội dung nhập phải được escape khi hiển thị.
- **Thu hẹp.** Giảm khoảng đệm giữa các ô trước khi giảm cỡ chữ. Giữ đủ bảy cột và hàng điều hướng ở chiều rộng thực tế. Khi xác minh, mở tháng bắt đầu vào Chủ nhật, tháng Hai năm nhuận và chuyển tháng 12 sang tháng 1.

### 17.3. Network · Chọn mạng trước, quản lý khi cần

[Ảnh hiện tại](docs/screenshots/ui/network-gruvbox.png) đặt thông tin IP và bốn thao tác lên trước danh sách SSID. Màn hình mặc định mới có một dòng kết nối, ô lọc, danh sách mạng và một dòng phím tắt cố định. Ví dụ dưới dùng dữ liệu từ fixture của repo. Ký hiệu `›` minh họa dòng đang chọn.

```text
  Connected · Example WiFi
  [Scan Alt+r]     [Actions Alt+m]
  Find network or action   …

  › Neighbor 5G                         92%    Secured
    IoT 2.4G                            69%    Secured
    Office WiFi                         55%    Secured
    Cafe Open                           45%    Open
    Guest Network                       41%    Secured

  ▌ NETWORK ACTIONS
  Connection details · Show
  Connect to hidden network
  …
```

- **Thứ tự và kích thước.** Hiện SSID đang dùng một lần ở dòng trạng thái. Danh sách bên dưới dành cho các mạng có thể chuyển sang. Dùng kết quả gom SSID hiện có, loại SSID đang dùng khỏi danh sách kết nối, sắp tín hiệu giảm dần và dùng tên để phân xử khi bằng nhau. Chọn dòng đầu khả dụng. Dùng một cột dọc, 74ch ở font mặc định, tối đa 94% chiều rộng. Hiện đủ năm mạng khác trong fixture trên màn hình 1280×800. Danh sách dài cuộn riêng. Gợi ý phím nằm ngoài danh sách lọc và cuộn.
- **Đọc một hàng.** SSID dùng màu chữ chính, tín hiệu căn phải, Open/Secured có vị trí cố định. Tránh dùng icon làm dấu hiệu duy nhất. Tên dài được rút gọn theo chiều rộng hiển thị sau khi dành chỗ cho hai trường bên phải. Lọc theo tên đầy đủ, kết nối bằng SSID gốc, escape markup cho tên hiển thị. Giữ loại bảo mật và băng tần của AP trong dữ liệu, hiển thị chi tiết mạng đang dùng trong khối Details.
- **Thao tác.** Gõ để lọc, ↑/↓ chọn, Enter kết nối. Mạng mở đi thẳng đến thao tác kết nối. Mạng bảo mật mở prompt mật khẩu che ký tự. Esc ở prompt quay về danh sách và hủy thao tác. Báo lỗi kết nối trong một dòng trạng thái, cho phép chọn lại. Thành công thì đọc lại trạng thái và đưa SSID vừa kết nối lên đầu màn hình. Alt+r yêu cầu scan. Alt+m đưa con trỏ tới nhóm Actions trong cùng danh sách và xóa bộ lọc để thấy các hành động. Esc tại màn hình chính đóng app.
- **Actions.** Nhóm thao tác nằm ngay sau danh sách Wi-Fi và dùng chung bộ lọc. Gồm Connection details khi có dữ liệu, bật/tắt Wi-Fi theo trạng thái, mạng ẩn, sửa profile đã lưu, ngắt Wi-Fi hiện tại, kết nối/ngắt Ethernet theo interface. Chỉ hiện hành động có đối tượng thích hợp. Kết nối Ethernet giữ cả đường chọn interface và thao tác nhanh khi có interface khả dụng. Nút Details hoặc `Alt+i` mở và thu gọn một khối thông tin phía trên danh sách. Saved connections mở rộng ngay dưới hàng tương ứng. Luồng profile giữ editor hiện có. Với mạng ẩn, hủy ở bước SSID hoặc mật khẩu đều quay lại, chỉ chuỗi mật khẩu rỗng được xác nhận chủ động mới biểu thị mạng mở.
- **Connection details.** Tiêu đề là tên kết nối. Mỗi dòng một cặp nhãn–giá trị cho interface, băng tần/bảo mật khi có, địa chỉ IP/prefix, gateway, subnet, broadcast, DNS và MAC. Cho giá trị dài xuống dòng. Dữ liệu thiếu dùng dấu gạch. Chỉ truy vấn thông số khi mở rộng Connection details.
- **Trạng thái đặc biệt.** Wi-Fi bật nhưng chưa nối hiện “Wi-Fi disconnected” và danh sách. Chỉ có mạng đang dùng thì hiện “No other networks found”. Quét rỗng hiện “No networks found” cùng phím Scan. Lọc rỗng vẫn cho sửa/xóa bộ lọc. Radio tắt hiện “Wi-Fi off” với hành động Turn Wi-Fi on. Thiếu adapter hiện “No Wi-Fi adapter” và các lựa chọn Ethernet khả dụng. Có Ethernet đang nối thì ghi interface trên dòng trạng thái, kể cả khi Wi-Fi tắt. Các câu thông báo rỗng là văn bản tĩnh. Chọn chúng tuyệt đối không kích hoạt lệnh thay đổi radio hoặc kết nối.

### 17.4. Handoff cho agent triển khai

1. Chạy từng binary với cấu hình mặc định, lưu ảnh trước ở cùng độ phân giải. Xem [cách chụp Rofi thật và fixtures](docs/screenshots/ui/README.md). Dùng fixture nmcli cho kiểm tra tương tác, kể cả biến thể lỗi/hủy trong thư mục tạm, tránh đổi mạng đang sử dụng.
2. Làm Calendar riêng. Bắt đầu từ [run, render_sheet và render_note](crates/rofi-calendar/src/main.rs). Giữ parser và quy tắc lịch. Xác minh widget order, ba cột một hàng, điều hướng qua cuối năm và nhập trực tiếp trong Rofi thật. Một lưới căn đúng trong chuỗi Rust vẫn có thể lệch khi Pango render.
3. Làm Network riêng. Bắt đầu từ [vòng lặp màn hình, build_menu và Action](crates/rofi-network/src/main.rs). Giữ Wi-Fi, hành động và chi tiết trong cùng danh sách. Kiểm tra ánh xạ lựa chọn sau lọc về đúng action. Kiểm tra hủy prompt bằng log đối số của nmcli giả với dữ liệu giả, xác nhận không có lệnh connect. Kiểm tra luồng Ethernet và profile trong nhóm Actions.
4. Đối chiếu [bộ sinh theme và ChromeProfile](crates/rofi-dmenu/src/theme.rs) với hai fallback [calendar.rasi](crates/rofi-calendar/share/rofi/calendar.rasi) và [network.rasi](crates/rofi-network/share/rofi/network.rasi). Đặt thay đổi widget order/geometry đúng app, đồng bộ theme sinh ra và fallback. Giữ thứ tự ưu tiên theme trong goal.md. Dùng lại helper khi đáp ứng bố cục.
5. Chạy kiểm tra hiện có từ root: `cargo test -p rofi-calendar -p rofi-network -p rofi-dmenu` và `bash scripts/test-theme.sh`. Nếu assertion cũ khóa một bố cục đã được thay thế, đối chiếu với hành vi mới trước khi cập nhật. Chạy trực tiếp entry point của hai app để kiểm tra tương tác.
6. Nghiệm thu bằng ảnh Rofi thật ở 1280×800, cùng font và scale cho cặp trước/sau. Xem Gruvbox, Nord, Solarized Light và Auto theo quy trình repo. Với Calendar, ảnh chính phải đọc rõ bảy cột và hôm nay, ảnh nhập ngày phải phân biệt ngày tìm được. Với Network, ảnh fixture phải hiện kết nối hiện tại và năm lựa chọn, phần chi tiết mở rộng phải đọc được giá trị dài. Thu hẹp màn hình để kiểm tra cuộn, cắt SSID và mép phải lưới lịch. Ghi riêng kết quả quan sát và ca chưa chạy. Ảnh ở nhiều theme chỉ xác minh màu sắc khi chưa thử hành vi bàn phím.

**Nghiệm thu bổ sung trong Rofi thật**

- [x] Calendar: chuyển sang tháng khác, tra một ngày rồi đổi theme. Kiểm tra tháng, ngày tra và dấu gạch dưới. Gõ tiếp một ngày khác, đổi theme trước Enter rồi xác nhận đúng chuỗi đã nhập. Thử cả ba nút bằng chuột và bằng phím ghi trên nút.
- [x] Network: dùng fixture có ít nhất hai SSID cùng khớp bộ lọc. Gõ lọc, chọn dòng thứ hai, đổi theme rồi Enter. Kiểm tra bộ lọc, nền chọn và log của nmcli giả: đúng SSID ban đầu, chỉ phát sinh yêu cầu kết nối sau Enter. Thử Scan và Manage bằng chuột lẫn phím khi danh sách đang lọc.
- [x] Cả hai app: thử đủ bốn phím theme, mở màn hình con của Network rồi quay lại. Mở đồng thời hai app, đổi theme ở một app và kiểm tra app kia. Đóng, mở lại và đối chiếu cấu hình theme cùng giao diện ban đầu. Ghi kết quả thao tác cạnh ảnh Rofi trước/sau của cùng một trạng thái.

### 17.5. Kết quả triển khai (2026-11-26)

**Calendar**

- Thứ tự widget: message (tháng + lưới) → inputbar `Go to date` → listview 3
  nút trên một hàng; theme sinh và fallback `calendar.rasi` cùng profile
  51ch, `columns: 3`, `lines: 1`.
- Lưới 7 cột monospace, số căn phải, tiêu đề/lưới canh giữa; hôm nay dùng nền
  accent; ngày tìm được dùng chữ đậm + gạch dưới và dòng `Thứ, dd/mm/yyyy`.
  Tháng 6 tuần, tháng 2 nhuận và chuyển 12→1 có unit test.
- Nhập ngày giữ nguyên `parse_query`; lỗi `31/02/2027` giữ tháng hiện tại,
  hiện một dòng `Invalid date: …` và cho gõ lại. Nội dung nhập được escape.
- Ảnh: `docs/screenshots/ui/redesign/calendar-before-*.png` và
  `calendar-after-*.png` (Gruvbox/Nord/Solarized/Auto),
  `calendar-focus-gruvbox.png`, `calendar-invalid.png`,
  `calendar-narrow.png`. Ảnh chụp Rofi thật 1280×800 (riêng narrow 1024×700)
  bằng Xvfb.

**Network**

- Màn hình chính: một dòng kết nối + dòng phím tắt trong message, inputbar
  `Find network`, danh sách chỉ còn SSID chưa dùng, sắp signal giảm dần rồi
  tên. Row giữ full SSID trong id để lọc theo tên đầy đủ dù hiển thị bị cắt;
  signal căn phải và Open/Secured cố định. Alt+r scan, Alt+m Manage hoạt động
  cả khi list rỗng/đang lọc.
- Màn hình Manage riêng: Connection details, bật/tắt Wi-Fi, mạng ẩn, sửa
  profile, ngắt Wi-Fi, quick wired và từng interface Ethernet. Hủy prompt ẩn
  quay lại Manage; mật khẩu rỗng phải xác nhận mở.
- Màn hình Connection details chỉ gọi `nmcli device show` khi mở; hiện
  Interface, Band, Security, IP/prefix, Gateway, Subnet, Broadcast, DNS, MAC,
  giá trị thiếu là `--`, giá trị dài được wrap.
- Trạng thái đặc biệt: `Wi-Fi off`, `Wi-Fi disconnected`, `No Wi-Fi adapter`,
  `No other networks found`, `No networks found`; các dòng thông báo là
  nonselectable và không chạy lệnh.
- Ảnh: `docs/screenshots/ui/redesign/network-before-*.png` và
  `network-after-*.png` (bốn theme), `network-manage-*.png`,
  `network-details-*.png`, `network-narrow.png`, `network-long-ssid.png`,
  `network-filter-long.png`, `network-only-active.png`, `network-wifi-off.png`.

**Lệnh đã chạy**

```text
cargo fmt --check --all                                  pass
cargo clippy --workspace --all-targets -- -D warnings    pass
cargo test --workspace --locked                          pass
MSRV 1.89 cargo check --workspace --locked --all-targets pass
scripts/test-apps.sh                                     31/0
scripts/test-theme.sh                                    14/0
scripts/test-installer.sh                                70/0
scripts/test-bundle.sh                                   31/0
scripts/test-release-index.sh                            10/0 (gồm merge 2 kiến trúc)
scripts/test-migration.sh                                14/0
scripts/test-install-matrix.sh                           4/0 (Debian/Ubuntu/Fedora/Arch)
scripts/test-performance.sh                              6/0 (network 36ms)
scripts/test-nix.sh                                      35/0 (flake check + per-app closures)
python3 scripts/check-deps.py                            121 checks
python3 scripts/check-app-sync.py                        69 checks
actionlint .github/workflows/release.yml                 pass
```

**Chưa chạy**: cài thật trên host aarch64 (chỉ mô phỏng chọn artifact bằng
`ROFI_APPS_ARCH=aarch64` và kiểm tra tarball ARM trong CI), chạy `nix build`
native ARM trên runner (job `nix-arm` đã thêm nhưng chưa có run), và ký release
bằng minisign key offline.

**Điểm kết thúc.** Agent bàn giao ảnh trước/sau của từng app, kết quả thao tác và kết quả các lệnh đã chạy. Nếu báo cáo cải thiện RAM hoặc thời gian mở, kèm số đo cùng máy, cùng fixture và cùng cấu hình. Mục tiêu của đợt này là khả năng đọc và thao tác trong Rofi.

### 17.6. Kiểm tra điều khiển và đổi theme (2026-11-27)

- Calendar giữ tháng 10/2026 và ngày tra 15/10 khi đổi theme, đồng thời giữ chuỗi `19` đang nhập. Enter sau khi đổi theme xác nhận 19/10. Đã dùng cả chuột và phím cho Prev, Today và Next. [Mặc định](docs/screenshots/ui/redesign/calendar-session-default.png) · [Đổi sang Nord khi đang nhập](docs/screenshots/ui/redesign/calendar-session-nord.png)
- Network lọc `Cafe`, chọn `Cafe Open` ở hàng thứ hai rồi đổi theme. Log `nmcli` giả giữ nguyên trong lúc đổi. Enter gửi đúng một lệnh kết nối cho `Cafe Open`. Với truy vấn rỗng kết quả, Enter giữ danh sách và Alt+m vẫn mở Manage. Đã dùng chuột và phím cho Scan và Manage, mở Connection details rồi đổi theme và quay lại. [Mặc định](docs/screenshots/ui/redesign/network-session-default.png) · [Đổi sang Nord với hai kết quả](docs/screenshots/ui/redesign/network-session-nord.png)
- Dùng Xvfb và Rofi 2.0.0 với hai màn hình 1280×800 và 1024×700. Đã thử đủ bốn phím trên cả hai app. Hai app chạy đồng thời trên hai display dùng cùng thư mục runtime. Đổi theme ở một app giữ nguyên theme app kia. Mở lại từng app dùng file override gốc. SHA-256 của cả hai file cấu hình giữ nguyên.
- Rust 1.89: workspace test pass 266 test trong 24 nhóm. Clippy cho ba crate liên quan pass với `-D warnings`. `scripts/test-apps.sh` pass 31/0 và `scripts/test-theme.sh` pass 14/0.

### 17.7. Bố cục một màn hình và phím Screen Time

- Các app dùng chung bộ sinh RASI cho khoảng cách, viền và trạng thái hàng được chọn. Calendar giữ lưới tháng và đặt Prev/Today/Next thành nút RASI ngoài danh sách lọc. Keybinds và Hub giữ danh sách có lọc; Clipboard dùng nút RASI cho các thao tác. Screen Time giữ header và timeline, với phần chi tiết mở rộng bên dưới. Atomic Tasks đưa thao tác task vào vùng mở rộng ngay trên danh sách.
- Network đặt danh sách Wi-Fi và Actions cùng vùng cuộn. `Alt+m` nhảy đến Actions. `Alt+i` hoặc nút Details hiện hoặc ẩn IP, gateway, VPN và DNS dưới trạng thái kết nối. Thông số kết nối chỉ xuất hiện trong khối Details. Saved connections mở rộng trong danh sách. Các prompt nhập mật khẩu và xác nhận giữ riêng.
- Clipboard dùng `Alt+d` để xóa hàng đang chọn trong danh sách đã lọc; các nút Copy/Delete/Clear/Refresh dùng cùng đường xử lý với Enter/`Alt+d`/`Alt+x`/`Alt+r`. Xóa toàn bộ lịch sử vẫn yêu cầu xác nhận.
- Screen Time dùng `Ctrl+1–4` để chọn theme và `Ctrl+PgUp/PgDn` để chuyển nhóm timeline. Enter mở chi tiết bên dưới timeline. Backspace hoặc Enter lại trên app đó thu gọn. Đổi theme giữ ngày, trang, app đang trỏ và chi tiết đang mở. Atomic Tasks giữ bộ phím riêng và mở vùng thao tác bằng Enter/Actions.

## 18. Báo cáo UI và lộ trình đạt 10/10

**Trạng thái** · Đã triển khai và xác minh bản trong working tree ngày 2026-11-27.
Mục này áp dụng cho bảy app và bổ sung tiêu chí nghiệm thu theo
[AGENTS.md](AGENTS.md#required-ui-rules) và [chuẩn UI](docs/UI_STYLE.md). Các
điểm dưới đây là điểm sau khi UI-01..08 pass trên Rofi thật; ảnh và log nằm
trong [bộ ảnh §18](docs/screenshots/ui/README.md).

### 18.1. Cách chấm và kết quả

Mỗi nhóm tối đa 2 điểm. Điểm 10 yêu cầu cả năm nhóm đạt đủ điểm trên luồng đã
xác minh.

| App | Bố cục, dễ đọc | Một màn hình | Điều khiển, header | Theme, trạng thái | Chạy độc lập | Tổng |
|---|---:|---:|---:|---:|---:|---:|
| Network | 2 | 2 | 2 | 2 | 2 | **10** |
| Calendar | 2 | 2 | 2 | 2 | 2 | **10** |
| Screen Time | 2 | 2 | 2 | 2 | 2 | **10** |
| Atomic Tasks | 2 | 2 | 2 | 2 | 2 | **10** |
| Hub Menu | 2 | 2 | 2 | 2 | 2 | **10** |
| Clipboard | 2 | 2 | 2 | 2 | 2 | **10** |
| Keybinds | 2 | 2 | 2 | 2 | 2 | **10** |

- **Bố cục, dễ đọc.** Nội dung phục vụ tác vụ chính, đủ khoảng cách, chữ đọc
  được ở cả hàng thường và hàng đang chọn.
- **Một màn hình.** Hoàn tất tác vụ thường dùng hoặc mở rộng chi tiết ngay
  trong app. Prompt riêng cho nhập liệu hoặc xác nhận được phép.
- **Điều khiển, header.** Phím ghi trên UI đúng với hành vi. Nút bấm được.
  Header giữ nguyên hình thức và không kích hoạt hành động.
- **Theme, trạng thái.** Tôn trọng cấu hình. Nếu có đổi theme, dùng `Ctrl+1–4`
  và giữ kích thước, font, bộ lọc, lựa chọn và phần chi tiết đang mở. App chưa
  có đổi theme trong phiên được chấm trên khả năng tuân thủ theme hiện có.
- **Chạy độc lập.** Khởi động được từ entry point riêng. App sở hữu cấu hình và
  dependency. Hub Menu là launcher tùy chọn.

### 18.2. Phát hiện theo mức độ ưu tiên

Bảng dưới giữ nguyên hiện tượng audit ban đầu để đối chiếu. Trạng thái hiện tại:
cả tám mục đã đóng và có ca kiểm chứng trong §18.3.

| ID | Mức | App | Hiện tượng đã kiểm tra |
|---|---|---|---|
| UI-01 | Cao | Keybinds | Fixture có hàng `Super+Enter Terminal`. Gõ `Terminal` làm danh sách rỗng. Rofi lọc chuỗi dòng đầu vào, hiện là ID số. |
| UI-02 | Cao | Clipboard, Keybinds | Theme desktop có nền chọn xanh nhạt. Màu chữ gắn cứng trong Pango vẫn sáng trên nền đó, khiến nội dung hàng đang chọn khó đọc. |
| UI-03 | Trung bình | Clipboard, Keybinds, Hub Menu | Click `HISTORY`, `BASIC`, `PRODUCTIVITY` làm header có nền hoặc viền chọn. Đánh dấu `nonselectable` chưa đủ để giữ hình thức cố định. |
| UI-04 | Trung bình | Clipboard | `Copy`, `Delete`, `Clear`, `Refresh` trông như chip thao tác, nhưng hiện được dựng bằng văn bản trong message. Bàn phím đã có luồng riêng. |
| UI-05 | Trung bình | Hub Menu | Lọc `calendar` rồi bấm `Ctrl+1` làm mất bộ lọc, danh sách quay về ba app và chiều cao tăng từ 236 lên 470 px trên fixture. |
| UI-06 | Trung bình | Screen Time | Chọn hàng `Brave`, bấm `Ctrl+1`, rồi Enter mở `Code`. Ngày và trang vẫn còn nhưng vị trí app đang chọn bị đặt lại lúc popup mở lại. |
| UI-07 | Trung bình | Atomic Tasks | Enter mở menu Task riêng với Done, Edit, Priority và Move to top. Đây là lần chọn thứ hai cho các thao tác thường dùng. |
| UI-08 | Thấp | Calendar | Khi nhập truy vấn ngày, bộ lọc Rofi ẩn hàng Prev / Today / Next. Điều hướng bằng phím vẫn hoạt động. |

**Network** đạt trên phạm vi đã kiểm tra. `Alt+i` bật/tắt một khối chi tiết,
các loại hàng đọc được, header giữ hình thức, và đổi đủ bốn theme giữ kích
thước. Hạng mục Network của đợt này là kiểm tra hồi quy sau thay đổi dùng chung.

### 18.3. Thứ tự triển khai

Các mục dưới đây đã hoàn tất. Ghi kết quả sau từng mục và giữ các thay đổi có
sẵn trong working tree.

#### Đợt 1 · Tìm kiếm, tương phản và header

- [x] **UI-01 — Keybinds.** Sửa [dựng danh sách và xử lý lựa chọn](crates/rofi-keybinds/src/main.rs).
  Đưa tổ hợp phím và mô tả vào phần được dmenu lập chỉ mục, đồng thời giữ chỉ
  số gốc hoặc ID ổn định để dispatch về đúng binding. Vẫn phân biệt hàng chạy
  được và hàng chỉ tra cứu. Kiểm tra trong Rofi bằng fixture
  [hyprland.conf](docs/screenshots/ui/fixtures/hyprland.conf). Tìm lần lượt
  `Terminal` và `Super+Enter`, xác nhận cùng một hàng. Enter với executable giả
  phải chạy đúng một lần. Header và kết quả rỗng không được chạy lệnh.
  **Kết quả:** row id dạng `<index>\t<combo>\t<description>`; Rofi thật lọc
  `Terminal` còn đúng `Super+Enter`, Enter chạy `foot` một lần; click/Enter
  trên `BASIC` giữ menu mở.
  [Ảnh](docs/screenshots/ui/redesign/ui01-keybinds-search.png).
- [x] **UI-02 — Clipboard và Keybinds.** Sửa nơi dựng dòng hoặc
  [helper hiển thị](crates/rofi-dmenu/src/ui.rs). Cho nội dung chữ và metadata
  thừa hưởng foreground của hàng, giữ trọng lượng chữ, ký hiệu và vai trò màu
  cần thiết. Xem cả hàng thường và hàng chọn trên theme nền xanh của desktop và
  ít nhất một theme sáng. Kiểm tra hàng text, URL và ảnh giả của Clipboard, tổ
  hợp phím và mô tả của Keybinds. Nếu sửa helper dùng chung, đối chiếu Hub và
  Network trước khi bàn giao.
  **Kết quả:** title/detail/metadata dùng foreground kế thừa, metadata dùng
  `alpha`; leading marker giữ vai trò màu. Đã thử nền chọn xanh nhạt trên
  Clipboard và Keybinds, cùng Network/Hub.
  [Keybinds](docs/screenshots/ui/redesign/ui02-keybinds-light-selection.png) ·
  [Clipboard](docs/screenshots/ui/redesign/ui02-clipboard-light-selection.png).
- [x] **UI-03 — Clipboard, Keybinds và Hub.** Đặt header vào trạng thái tĩnh
  như Network. Xem cách Network truyền chỉ số hàng active và áp dụng cùng style
  cho các trạng thái của header trong [màn hình Network](crates/rofi-network/src/main.rs).
  Kiểm tra cả theme mặc định và theme người dùng. Hover, click và điều hướng
  qua header phải giữ nguyên nền, chữ và viền. Enter trên header giữ app mở và
  không chạy lệnh. Hàng dữ liệu cạnh header vẫn phải có highlight dễ nhận ra.
  Shell/CLI trả về header cũng phải được bỏ qua an toàn.
  **Kết quả:** `-a` đánh dấu header, rule `element normal.active`,
  `alternate.active`, `selected.active` giống nhau; app bỏ qua id `section-*`.
  Click/Enter header trên Keybinds và Hub giữ app mở, không chạy lệnh.
  [Keybinds](docs/screenshots/ui/redesign/ui03-keybinds-header.png).
- [x] **UI-04 — Clipboard.** Đổi chip văn bản thành nút RASI thật trên màn hình
  lịch sử. Nút và phím tương ứng dùng cùng một đường xử lý, gồm Copy với Enter,
  Delete với `Alt+d`, Clear với `Alt+x`, Refresh với `Alt+r`. Mở với fixture có
  ít nhất hai mục cùng khớp bộ lọc. Sau khi chọn hàng thứ hai, bấm Copy hoặc
  Delete phải tác động đúng mục và giữ nguyên hành vi của phím tắt. Clear vẫn
  qua xác nhận, Cancel giữ toàn bộ mục. Refresh hoạt động cả khi kết quả lọc
  rỗng. Dùng `cliphist` và `wl-copy` giả để kiểm tra cả đường chuột và bàn phím.
  [Mã Clipboard](crates/rofi-clipboard/src/main.rs)
  **Kết quả:** bốn nút RASI dùng `content`/`action` trên dòng riêng; Copy
  (`kb-accept-entry`), Delete, Clear và Refresh đi đúng đường phím. Click Copy
  copy đúng hàng; Delete xóa đúng hàng thứ hai; Clear mở xác nhận và Cancel
  giữ đủ 3 mục; Refresh giữ bộ lọc khi danh sách rỗng.
  [Nút](docs/screenshots/ui/redesign/ui04-clipboard-actions.png) ·
  [Clear](docs/screenshots/ui/redesign/ui04-clipboard-clear-confirm.png) ·
  [Refresh](docs/screenshots/ui/redesign/ui04-clipboard-refresh-empty.png).

#### Đợt 2 · Giữ trạng thái khi đổi theme

- [x] **UI-05 — Hub Menu.** Sửa [vòng lặp mở launcher](crates/rofi-hub-menu/src/hub.rs)
  và [resolve theme](crates/rofi-hub-menu/src/theme.rs). Dùng cơ chế giữ
  input/lựa chọn hiện có của `rofi-dmenu`, giữ ID provider thay vì coi chỉ số
  nhìn thấy là định danh. Áp palette lên theme đã resolve. Giữ font, chiều rộng,
  chiều cao và nội dung list. `Ctrl+4` khôi phục theme theo cấu hình. Giữ quyền
  sở hữu theme ở menu. Fixture phải có ít nhất hai provider khớp cùng từ khóa.
  Chọn provider thứ hai, bấm lần lượt bốn phím, đối chiếu bộ lọc và kích thước.
  Enter chỉ gọi provider thứ hai và chỉ gọi sau khi xác nhận.
  **Kết quả:** dùng `dmenu_with_input`, giữ `selected_id` và `filter`; palette
  được ghi đè lên theme resolve (`theme-hub.rasi`), `listview { dynamic: false }`
  giữ chiều cao. Thử Nord/Solarized/Gruvbox/Auto: bộ lọc `Alpha`, provider thứ
  hai, kích thước 1024×353 giữ nguyên; `Ctrl+4` về Auto; Enter chạy provider
  thứ hai.
  [Nord](docs/screenshots/ui/redesign/ui05-hub-filter-nord.png) ·
  [Ctrl+4](docs/screenshots/ui/redesign/ui05-hub-auto-restored.png).
- [x] **UI-06 — Screen Time.** Sửa [xử lý giao diện script mode](crates/screen-time/src/ui.rs),
  [renderer](crates/screen-time/src/render.rs) và
  [launcher](crates/screen-time/share/rofi-screen-time) nếu cần. Kiểm tra Rofi
  thật trả định danh nào khi nhận phím theme trước khi chọn cách lưu. Tách mục
  đang được trỏ khỏi app đang mở chi tiết. Phục hồi bằng tên/ID của app trên
  trang tương ứng sau khi Rofi mở lại, chỉ dùng hàng app đầu tiên khi mục cũ đã
  biến mất. Trên fixture ít nhất tám app, thử từ hàng thứ ba và từ trang thứ
  hai. Kiểm tra đủ bốn theme, ngày/trang giữ nguyên, chi tiết đang mở giữ nguyên
  và Enter sau khi đổi mở đúng app. Bộ chọn activity và nền dữ liệu vẫn giữ
  layout hiện tại.
  **Kết quả:** Rofi thật trả `ROFI_INFO` = app đang trỏ; state lưu `cursor`
  riêng `app`; renderer ghi `rofi-selection`, launcher truyền `-selected-row`
  khi reopen. Fixture 8 app: chọn `Terminal` (hàng 3) và `Zoom` (trang 2), thử
  Nord/Solarized/Gruvbox/Auto; ngày, trang, chi tiết, kích thước 1024×523/733
  và lựa chọn giữ nguyên. Mục biến mất fallback hàng app đầu tiên.
  [Cursor](docs/screenshots/ui/redesign/ui06-screen-time-cursor-nord.png) ·
  [Trang 2](docs/screenshots/ui/redesign/ui06-screen-time-page2-solarized.png) ·
  [Chi tiết](docs/screenshots/ui/redesign/ui06-screen-time-detail-nord.png).

#### Đợt 3 · Điều khiển ngay trên màn hình chính

- [x] **UI-07 — Atomic Tasks.** Sửa [menu shell](crates/atomic-note/share/atomic-note).
  Đưa Done, Edit, Move to top và chọn priority về cùng màn hình, có thể hiển
  thị ở vùng hành động mở rộng theo task đang chọn. Giữ đường dùng chuột và các
  phím hiện tại. Enter có thể mở hoặc thu gọn vùng đó. Prompt nhập nội dung cho
  Add/Edit và xác nhận Clear vẫn được phép. Giữ định dạng `~/.atomic_tasks`,
  undo, lock và lệnh Waybar. Dùng HOME tạm, task fixture và `pkill` giả. Chọn
  đúng một task khi đã lọc, thực hiện Done, Edit, Move và từng priority bằng
  chuột và phím. Kiểm tra undo và render Waybar bằng dữ liệu giả. Không thêm nút
  đổi theme trong đợt này.
  **Kết quả:** panel inline mở bằng Enter hoặc nút `Actions`; Done/Edit/Top và
  bốn priority dùng đúng phím cũ; filter và selection sống qua reopen. Đã thử
  click Done, priority, Top; sửa task qua prompt; filter `Regular`; undo/Waybar
  giữ nguyên. `crates/atomic-note/tests/menu.sh` pass 22/0 (gồm undo và Waybar).
  [Ảnh](docs/screenshots/ui/redesign/ui07-atomic-actions.png).
- [x] **UI-08 — Calendar.** Tách ba nút điều hướng khỏi danh sách chịu bộ lọc
  Rofi trong [vòng lặp Calendar](crates/rofi-calendar/src/main.rs). Dùng widget
  nút có hành động tương ứng với Left, Ctrl+t và Right. Khi text ngày chưa hoàn
  thành, các nút vẫn hiện và bấm được. Enter trên chuỗi ngày chỉ xử lý một truy
  vấn. Giữ parser, cách hiển thị ngày tra, lỗi nhập và lưới bảy cột. Dùng chuột
  và phím qua ranh giới tháng 12/1. Gõ một phần ngày, đổi theme rồi xác nhận
  chuỗi, kiểm tra nút vẫn hiện, font và vị trí giữ nguyên.
  **Kết quả:** danh sách rỗng có `allow_empty`; Prev/Today/Next là RASI button
  ngoài list, action tương ứng Left/Ctrl+t/Right. Đã bấm nút qua tháng 10→11,
  December→January; gõ `19`, Ctrl+1, Enter cho đúng 19/10/2026; nút và bố cục
  giữ nguyên. Ctrl+1 khóa Nord trong `~/.local/state/rofi-apps/calendar/theme-choice`;
  mở lại vẫn Nord, Ctrl+4 xóa khóa và trở về cấu hình.
  [Nút](docs/screenshots/ui/redesign/ui08-calendar-buttons.png) ·
  [Gõ ngày + Nord](docs/screenshots/ui/redesign/ui08-calendar-typed-nord.png) ·
  [January](docs/screenshots/ui/redesign/ui08-calendar-january.png).

### 18.4. Điều kiện chốt 10/10

- [x] Chạy các kiểm tra hiện có trong **checkout chứa bản sửa**. Dùng
  `cargo fmt --check --all`, `cargo test --workspace --locked`,
  `bash scripts/test-theme.sh`, `bash scripts/test-apps.sh`,
  `python3 scripts/check-app-sync.py`, `python3 scripts/check-deps.py` và
  `git diff --check`. Với Atomic Tasks, chạy
  `bash -n crates/atomic-note/share/atomic-note` và ShellCheck nếu có.
  **Kết quả:** fmt pass; workspace 288 pass/0 fail; theme 14/0; apps 31/0;
  check-app-sync 76 checks cho 7 app; check-deps 121 checks; Atomic menu 29/0;
  `bash -n`, ShellCheck 0.11.0 và `git diff --check` pass.
- [x] Xác minh các ca UI-01..08 bằng Rofi thật ở cùng font và scale. Dùng ảnh
  trước/sau cho lỗi hiển thị và log fixture cho thao tác. Với lỗi đã xuất hiện
  trên theme desktop, lấy bản sao theme đó vào thư mục tạm và thử lại. Dùng
  binary mới build của checkout với `ROFI_BIN` hoặc wrapper thử nghiệm đã xác
  minh. Wrapper Nix có thể đặt lệnh thật trước lệnh giả trong PATH, nên không
  dùng binary hệ thống cho bài thử Clipboard.
  **Kết quả:** Xvfb 1280×800, Rofi 2.0.0, JetBrainsMono Nerd Font 12; binary
  `target/debug/*`; ảnh trong `docs/screenshots/ui/redesign/ui01..ui08-*.png`.
  Clipboard dùng `cliphist`/`wl-copy` giả; Keybinds dùng `hyprland.conf` và
  `foot` giả; Screen Time dùng SQLite 8 app; Atomic dùng HOME tạm.
- [x] Kiểm tra hồi quy Network sau thay đổi chung. Lọc hai SSID, chọn hàng thứ
  hai, đổi theme rồi Enter với `nmcli` giả. Phải giữ lọc, SSID và kích thước,
  chỉ có một yêu cầu kết nối sau Enter. Nút Details và `Alt+i` mở cùng một khối,
  tiêu đề không highlight và vùng action còn dùng được.
  **Kết quả:** lọc `Cafe`, chọn `Cafe Open`, Ctrl+1: bộ lọc và kích thước
  804×332 giữ nguyên; Enter tạo đúng một `device wifi connect Cafe Open`.
  Ctrl+1 khóa Nord trong `~/.local/state/rofi-apps/network/theme-choice`; mở
  lại vẫn Nord, Ctrl+4 xóa khóa.
  Details bấm chuột mở khối IP/gateway/VPN/DNS/MAC, header tĩnh, kích thước
  804×624 giữ sau theme.
  [Lọc](docs/screenshots/ui/redesign/network-regression-filter-nord.png) ·
  [Details](docs/screenshots/ui/redesign/network-regression-details-nord.png).
- [x] Kiểm tra các entry point vẫn chạy độc lập. Xác nhận không có dependency
  mới lên menu/installer và không có sửa đổi tới task, Clipboard hoặc Wi-Fi thật
  trong quá trình nghiệm thu.
  **Kết quả:** bảy entry point chạy trực tiếp (`rofi-calendar`,
  `rofi-clipboard`, `rofi-keybinds`, `rofi-network`, `atomic-note`,
  `rofi-screen-time`, `rofi-hub-menu`); `check-deps.py` pass; không kết nối
  Wi-Fi thật, không đọc/ghi clipboard thật, task chỉ dùng HOME tạm.
- [x] Chấm lại từng app theo năm nhóm ở §18.1. Chỉ nâng điểm khi ca tương ứng
  pass. Agent bàn giao ID đã đóng, ảnh hoặc log kiểm chứng và các ca còn thiếu.
  Bản trong repo, bản đang chạy trên desktop và trạng thái phát hành ghi riêng.
  **Kết quả:** cả bảy app đạt 10/10 trên snapshot local đã xác minh. Chưa xác
  nhận bản cài trên desktop thật hoặc mọi mức scale Wayland; các ca đó vẫn ghi
  là chưa chạy, không nâng điểm.

### 18.5. Rà soát tài liệu (đợt cuối)

- [x] Rà soát và cập nhật [README](README.md): bảng app, mô tả UI, entry
  point trực tiếp, layout repo, theme menu, và lệnh test Atomic.
- [x] Rà soát và cập nhật [UI_STYLE](docs/UI_STYLE.md): profile bảy app, hàng
  thừa hưởng foreground, id tìm kiếm, header tĩnh, nút RASI, Calendar,
  Clipboard, Hub, Screen Time và Atomic; quy tắc `content`/`action` trên dòng
  riêng cho `-theme-str`; theme menu theo lớp palette.
- [x] Rà soát và cập nhật [screenshot README](docs/screenshots/ui/README.md):
  bảy app, bảng ảnh §18, fixture, và lệnh tái tạo. Bộ ảnh bốn theme vẫn là
  baseline palette/layout; ảnh tương tác hiện tại nằm trong mục §18.
- [x] Chuẩn hóa ảnh `*-current.png` từ fixture và ghi rõ dữ liệu nhạy cảm đã
  thay bằng ví dụ: SSID/IP, clipboard, task, lệnh và đường dẫn.
  Keybinds dùng `fixtures/hyprland.conf`; Network hiển thị đủ 12 ứng viên và
  header; Clipboard dùng `fixtures/cliphist`; Atomic dùng HOME tạm.
- [x] Đồng bộ `Ctrl+1–4` cho Calendar, Clipboard, Keybinds, Network, Atomic
  Tasks và Screen Time; mỗi app giữ khóa theme riêng, `Ctrl+4` xóa khóa. Hub
  Menu giữ state riêng và vẫn giữ bộ lọc/lựa chọn/chiều cao.
- [x] Rà soát và cập nhật [ARCHITECTURE](docs/ARCHITECTURE.md): Atomic Tasks
  trong workspace, `install.sh`, theme menu theo lớp, và file
  `rofi-selection` của Screen Time.
- [x] Rà soát và cập nhật [PACKAGING](docs/PACKAGING.md) và
  [MIGRATING](docs/MIGRATING.md): tarball/package Atomic, version table, entry
  point trực tiếp.
- [x] Rà soát và cập nhật [CHANGELOG](CHANGELOG.md), [Screen Time README](crates/screen-time/README.md),
  [Screen Time user guide](crates/screen-time/docs/USER_GUIDE.md),
  [Screen Time changelog](crates/screen-time/CHANGELOG.md) và
  [developer docs](crates/screen-time/docs/README.md): phím, cursor/detail,
  kích thước, chọn theme và test tương ứng.
- [x] Rà soát [AGENTS.md](AGENTS.md), [docs README](docs/README.md) và các
  hướng dẫn còn lại; không còn tuyên bố sai về số app hiện tại, menu Task thứ hai, chip văn
  bản Clipboard, hoặc nút Calendar nằm trong danh sách lọc.
- [x] Chạy lại `git diff --check` sau khi sửa tài liệu và xác nhận ảnh mới nằm
  trong repo. Đây là bước cuối của kế hoạch §18.
