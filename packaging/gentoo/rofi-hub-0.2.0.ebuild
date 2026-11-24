# Copyright 2026 Gentoo Authors
# Distributed under the terms of the GNU General Public License v2

# Generated from Cargo.lock. Keep in sync with the release tag.
EAPI=8

CRATES="
	android_system_properties@0.1.6
	anstream@1.0.0
	anstyle-parse@1.0.0
	anstyle-query@1.1.5
	anstyle-wincon@3.0.11
	anstyle@1.0.14
	anyhow@1.0.105
	async-broadcast@0.7.2
	async-channel@2.5.0
	async-executor@1.14.1
	async-io@2.6.0
	async-lock@3.5.0
	async-process@2.5.0
	async-recursion@1.2.0
	async-signal@0.2.14
	async-task@4.7.1
	async-trait@0.1.92
	atomic-waker@1.1.2
	autocfg@1.5.1
	bitflags@2.13.2
	blocking@1.7.0
	bumpalo@3.20.4
	cc@1.11.3
	cfg-if@1.0.5
	chrono@0.4.45
	clap@4.6.9
	clap_builder@4.6.7
	clap_derive@4.6.9
	clap_lex@1.1.2
	colorchoice@1.0.5
	concurrent-queue@2.5.0
	core-foundation-sys@0.8.7
	crossbeam-utils@0.8.23
	endi@1.1.1
	enumflags2@0.7.12
	enumflags2_derive@0.7.12
	equivalent@1.0.3
	errno@0.3.15
	event-listener-strategy@0.5.4
	event-listener@5.4.2
	fallible-iterator@0.3.0
	fallible-streaming-iterator@0.1.9
	fastrand@2.5.0
	find-msvc-tools@0.1.15
	foldhash@0.2.0
	futures-core@0.3.34
	futures-io@0.3.34
	futures-lite@2.6.1
	futures-task@0.3.34
	futures-util@0.3.34
	getrandom@0.4.3
	hashbrown@0.17.2
	hashlink@0.12.2
	heck@0.5.0
	hermit-abi@0.5.3
	hex@0.4.3
	iana-time-zone-haiku@0.1.2
	iana-time-zone@0.1.65
	indexmap@2.14.2
	is_terminal_polyfill@1.70.2
	itoa@1.0.19
	js-sys@0.3.107
	libc@0.2.191
	libsqlite3-sys@0.40.0
	linux-raw-sys@0.12.1
	log@0.4.35
	memchr@2.8.3
	memoffset@0.9.1
	num-traits@0.2.19
	once_cell@1.21.4
	once_cell_polyfill@1.70.2
	ordered-stream@0.2.0
	parking@2.2.1
	pin-project-lite@0.2.18
	piper@0.2.5
	pkg-config@0.3.34
	polling@3.11.0
	proc-macro-crate@3.5.0
	proc-macro2@1.0.107
	quote@1.0.47
	r-efi@6.0.0
	rsqlite-vfs@0.2.0
	rusqlite@0.43.0
	rustix@1.1.6
	rustversion@1.0.23
	serde@1.0.229
	serde_core@1.0.229
	serde_derive@1.0.229
	serde_json@1.0.154
	serde_repr@0.1.21
	serde_spanned@0.6.9
	shlex@2.0.1
	signal-hook-registry@1.4.8
	signal-hook@0.3.18
	slab@0.4.12
	smallvec@1.16.6
	sqlite-wasm-rs@0.6.3
	strsim@0.11.1
	syn@2.0.119
	syn@3.0.10
	tempfile@3.27.0
	thiserror-impl@2.0.21
	thiserror@2.0.21
	toml@0.8.23
	toml_datetime@0.6.11
	toml_datetime@1.1.3+spec-1.1.0
	toml_edit@0.22.27
	toml_edit@0.25.20+spec-1.1.0
	toml_parser@1.1.6+spec-1.1.0
	toml_write@0.1.2
	tracing-attributes@0.1.31
	tracing-core@0.1.36
	tracing@0.1.44
	uds_windows@1.2.1
	unicode-ident@1.0.27
	utf8parse@0.2.2
	uuid@1.26.1
	vcpkg@0.2.15
	wasm-bindgen-macro-support@0.2.130
	wasm-bindgen-macro@0.2.130
	wasm-bindgen-shared@0.2.130
	wasm-bindgen@0.2.130
	windows-core@0.62.2
	windows-implement@0.60.2
	windows-interface@0.59.3
	windows-link@0.2.1
	windows-result@0.4.1
	windows-strings@0.5.1
	windows-sys@0.61.2
	winnow@0.7.15
	winnow@1.0.5
	zbus@5.19.0
	zbus_macros@5.19.0
	zbus_names@4.3.5
	zcheapstr@1.4.0
	zmij@1.0.23
	zvariant@5.15.1
	zvariant_derive@5.15.1
	zvariant_utils@4.2.0
"

inherit cargo

DESCRIPTION="Rofi launcher with screen-time, calendar, keybinds, and clipboard apps"
HOMEPAGE="https://github.com/WillGle/rofi-hub"
SRC_URI="
	https://github.com/WillGle/rofi-hub/releases/download/v${PV}/rofi-hub-${PV}-source.tar.gz -> ${P}.tar.gz
	${CARGO_CRATE_URIS}
"
S="${WORKDIR}/rofi-hub-${PV}"

LICENSE="MIT"
SLOT="0"
KEYWORDS="~amd64 ~arm64"
IUSE=""

# cliphist, wl-clipboard, Hyprland, and systemd are optional. Keep the package
# installable without them; document them in metadata.xml instead of forcing
# USE flags that may not exist in every overlay.
RDEPEND="
	rofi
	bash
"
BDEPEND="virtual/rust"

QA_FLAGS_IGNORED="usr/bin/rofi-hub usr/bin/rofi-calendar usr/bin/rofi-keybinds usr/bin/rofi-clipboard usr/bin/screen-time"

src_compile() {
	cargo_src_compile --workspace
}

src_test() {
	cargo_src_test --workspace
}

src_install() {
	DESTDIR="${D}" PREFIX=/usr TARGETDIR="${CARGO_TARGET_DIR:-${S}/target}/release" \
		"${S}/packaging/install-tree.sh" --all
}
