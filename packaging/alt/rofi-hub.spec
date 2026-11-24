#
# ALT Sisyphus spec. Build with rpmbuild -ba rofi-hub.spec.
#
# OpenRC/runit files are contrib examples, not distribution dependencies.
%global __requires_exclude_from ^/usr/share/rofi-hub/contrib/.*

Name:           rofi-hub
Version:        0.2.0
Release:        alt1
Summary:        Rofi launcher with screen-time, calendar, keybinds, and clipboard apps
License:        MIT
Group:          Graphical desktop/Applications/Utilities
URL:            https://github.com/WillGle/rofi-hub
Source0:        https://github.com/WillGle/rofi-hub/releases/download/v%{version}/rofi-hub-%{version}-source.tar.gz
BuildRequires:  cargo
BuildRequires:  rust
BuildRequires:  gcc
BuildRequires:  make
Requires:       rofi
Requires:       bash
Recommends:     cliphist
Recommends:     wl-clipboard
Recommends:     systemd
Suggests:       hyprland

%description
Rofi Hub is a keyboard-driven launcher for Rofi apps. It ships a calendar, a
Hyprland keybinding reference, a cliphist picker, and the Screen Time tracker
with a systemd user service.

%prep
%autosetup -n rofi-hub-%{version}

%build
cargo build --release --locked --workspace

%check
cargo test --workspace --locked

%install
DESTDIR=%{buildroot} PREFIX=/usr packaging/install-tree.sh --all

%files
%{_bindir}/rofi-hub
%{_bindir}/rofi-calendar
%{_bindir}/rofi-keybinds
%{_bindir}/rofi-clipboard
%{_bindir}/screen-time
%{_bindir}/rofi-screen-time
%{_datadir}/rofi-hub
%{_datadir}/rofi/screen-time.rasi
%{_datadir}/screen-time
%{_datadir}/applications/rofi-hub.desktop
%{_datadir}/icons/hicolor/scalable/apps/rofi-hub.svg
%{_datadir}/metainfo/rofi-hub.metainfo.xml
%{_datadir}/bash-completion/completions/rofi-hub
%{_datadir}/bash-completion/completions/screen-time
%{_datadir}/zsh/site-functions/_rofi-hub
%{_datadir}/zsh/site-functions/_screen-time
%{_datadir}/fish/vendor_completions.d/rofi-hub.fish
%{_datadir}/fish/vendor_completions.d/screen-time.fish
%{_datadir}/licenses/rofi-hub/LICENSE
%{_datadir}/doc/rofi-hub
%{_mandir}/man1/rofi-hub.1.gz
%{_mandir}/man1/rofi-calendar.1.gz
%{_mandir}/man1/rofi-keybinds.1.gz
%{_mandir}/man1/rofi-clipboard.1.gz
%{_mandir}/man1/screen-time.1.gz
%{_mandir}/man1/rofi-screen-time.1.gz
%{_prefix}/lib/systemd/user/screen-time.service

%changelog
* Wed Nov 25 2026 WillGle <willgle@users.noreply.github.com> - 0.2.0-alt1
- Initial package.
