{
  description = "rofi-apps: independent Rofi apps, an optional menu, and an installer";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";
  };

  outputs =
    { self, nixpkgs }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forAllSystems =
        f:
        nixpkgs.lib.genAttrs systems (
          system:
          f {
            pkgs = import nixpkgs { inherit system; };
          }
        );
      crateVersion =
        crate: (builtins.fromTOML (builtins.readFile (./crates + "/${crate}/Cargo.toml"))).package.version;
      installerVersion = "0.3.0";
    in
    {
      packages = forAllSystems (
        { pkgs }:
        let
          lib = pkgs.lib;
          src = lib.cleanSourceWith {
            src = lib.cleanSource ./.;
            filter =
              path: type:
              let
                name = baseNameOf path;
              in
              !(lib.elem name [
                "target"
                "dist"
                ".git"
                ".direnv"
                "result"
              ]);
          };

          # Build one app as an independent package. The package carries only
          # its own binary, provider manifest, layout fallback, and declared
          # runtime dependencies. `targetPkgs` is the (possibly cross) pkgs
          # set, so the same builder produces the aarch64 release artifacts.
          mkApp =
            targetPkgs:
            {
              crate,
              id,
              bin,
              extraRuntime ? [ ],
              hasProvider ? true,
              hasLayout ? true,
              hasAppToml ? true,
              extraInstall ? "",
              description,
            }:
            let
              runtime = [ targetPkgs.rofi ] ++ extraRuntime;
            in
            targetPkgs.rustPlatform.buildRustPackage ({
              pname = bin;
              version = crateVersion crate;
              inherit src;

              cargoLock.lockFile = ./Cargo.lock;
              cargoBuildFlags = [
                "--package"
                "${crate}"
              ];
              cargoTestFlags = [
                "--package"
                "${crate}"
              ];

              nativeBuildInputs = [ targetPkgs.makeWrapper ];
              preCheck = ''
                export TZ=UTC
                export TZDIR=${targetPkgs.tzdata}/share/zoneinfo
              '';

              postInstall = ''
                ${lib.optionalString hasProvider ''
                  install -Dm644 crates/${crate}/providers/${id}.toml \
                    "$out/share/rofi-apps/providers/${id}.toml"
                ''}
                ${lib.optionalString hasLayout ''
                  install -Dm644 crates/${crate}/share/rofi/${id}.rasi \
                    "$out/share/rofi/${id}.rasi"
                ''}
                ${lib.optionalString hasAppToml ''
                  install -Dm644 crates/${crate}/app.toml \
                    "$out/share/rofi-apps/apps/${id}/app.toml"
                ''}
                ${extraInstall}
              '';

              postFixup = ''
                wrapProgram "$out/bin/${bin}" \
                  --prefix PATH : "${lib.makeBinPath runtime}"
                ${lib.optionalString (bin == "screen-time") ''
                  wrapProgram "$out/bin/rofi-screen-time" \
                    --prefix PATH : "${lib.makeBinPath runtime}"
                ''}
              '';

              meta = {
                inherit description;
                license = lib.licenses.mit;
                mainProgram = bin;
                platforms = systems;
              };
            });

          atomicNoteMetadata = builtins.fromTOML (builtins.readFile ./crates/atomic-note/app.toml);
          atomicNote = pkgs.stdenvNoCC.mkDerivation {
            pname = "atomic-note";
            inherit (atomicNoteMetadata) version;
            src = ./crates/atomic-note;
            nativeBuildInputs = [ pkgs.makeWrapper ];
            dontBuild = true;
            installPhase = ''
              runHook preInstall
              install -Dm755 share/atomic-note "$out/bin/atomic-note"
              install -Dm644 app.toml "$out/share/rofi-apps/apps/atomic-note/app.toml"
              install -Dm644 providers/atomic-note.toml "$out/share/rofi-apps/providers/atomic-note.toml"
              install -Dm644 share/rofi/atomic-note.rasi "$out/share/rofi/atomic-note.rasi"
              runHook postInstall
            '';
            postFixup = ''
              wrapProgram "$out/bin/atomic-note" --prefix PATH : "${
                lib.makeBinPath (
                  with pkgs;
                  [
                    rofi
                    coreutils
                    gawk
                    jq
                    procps
                    util-linux
                  ]
                )
              }"
            '';
            meta = {
              description = "Rofi task queue with optional Waybar integration";
              license = lib.licenses.mit;
              mainProgram = "atomic-note";
              platforms = systems;
            };
          };

          rofiCalendar = mkApp pkgs {
            crate = "rofi-calendar";
            id = "calendar";
            bin = "rofi-calendar";
            description = "Month sheet with a date search box for Rofi";
          };

          rofiClipboard = mkApp pkgs {
            crate = "rofi-clipboard";
            id = "clipboard";
            bin = "rofi-clipboard";
            extraRuntime = [
              pkgs.cliphist
              pkgs.wl-clipboard
            ];
            description = "Clipboard history picker for cliphist and wl-copy";
          };

          rofiKeybinds = mkApp pkgs {
            crate = "rofi-keybinds";
            id = "keybinds";
            bin = "rofi-keybinds";
            description = "Hyprland keyboard shortcut reference for Rofi";
          };

          rofiNetwork = mkApp pkgs {
            crate = "rofi-network";
            id = "network";
            bin = "rofi-network";
            extraRuntime = [ pkgs.networkmanager ];
            description = "NetworkManager Wi-Fi and Ethernet menu for Rofi";
          };

          screenTime = mkApp pkgs {
            crate = "screen-time";
            id = "screen-time";
            bin = "screen-time";
            description = "Local, per-user screen-time tracker with a Rofi timeline";
            extraInstall = ''
              install -Dm755 crates/screen-time/share/rofi-screen-time \
                "$out/bin/rofi-screen-time"
              install -Dm644 crates/screen-time/systemd/screen-time.service \
                "$out/systemd/screen-time.service"
            '';
          };

          mkMenu =
            targetPkgs:
            targetPkgs.rustPlatform.buildRustPackage {
              pname = "rofi-hub-menu";
              version = crateVersion "rofi-hub-menu";
              inherit src;

              cargoLock.lockFile = ./Cargo.lock;
              cargoBuildFlags = [
                "--package"
                "rofi-hub-menu"
              ];
              cargoTestFlags = [
                "--package"
                "rofi-hub-menu"
              ];

              nativeBuildInputs = [ targetPkgs.makeWrapper ];

              postInstall = ''
                install -Dm644 crates/rofi-hub-menu/share/rofi/hub-menu.rasi \
                  "$out/share/rofi/hub-menu.rasi"
              '';

              postFixup = ''
                # Append, not prepend: a session may provide a plugin-enabled
                # Rofi (calc, emoji). Plain rofi is only a fallback so the menu
                # stays self-contained without shadowing the user build.
                wrapProgram "$out/bin/rofi-hub-menu" \
                  --suffix PATH : "${targetPkgs.rofi}/bin"
              '';

              meta = {
                description = "Optional Rofi menu that lists and launches installed rofi-apps";
                license = lib.licenses.mit;
                mainProgram = "rofi-hub-menu";
                platforms = systems;
              };
            };

          rofiHubMenu = mkMenu pkgs;

          # Cross-build outputs: the aarch64 release artifacts can be produced
          # on an x86_64 host without emulation. The aarch64 CI runner still
          # builds natively for the published release.
          crossAarch64 = pkgs.pkgsCross.aarch64-multiplatform;
          crossApps =
            if pkgs.stdenv.hostPlatform.isx86_64 then
              {
                rofi-calendar-aarch64 = mkApp crossAarch64 {
                  crate = "rofi-calendar";
                  id = "calendar";
                  bin = "rofi-calendar";
                  description = "Month sheet with a date search box for Rofi (aarch64 cross-build)";
                };
                rofi-clipboard-aarch64 = mkApp crossAarch64 {
                  crate = "rofi-clipboard";
                  id = "clipboard";
                  bin = "rofi-clipboard";
                  extraRuntime = [
                    crossAarch64.cliphist
                    crossAarch64.wl-clipboard
                  ];
                  description = "Clipboard history picker for cliphist and wl-copy (aarch64 cross-build)";
                };
                rofi-keybinds-aarch64 = mkApp crossAarch64 {
                  crate = "rofi-keybinds";
                  id = "keybinds";
                  bin = "rofi-keybinds";
                  description = "Hyprland keyboard shortcut reference for Rofi (aarch64 cross-build)";
                };
                rofi-network-aarch64 = mkApp crossAarch64 {
                  crate = "rofi-network";
                  id = "network";
                  bin = "rofi-network";
                  extraRuntime = [ crossAarch64.networkmanager ];
                  description = "NetworkManager Wi-Fi and Ethernet menu for Rofi (aarch64 cross-build)";
                };
                screen-time-aarch64 = mkApp crossAarch64 {
                  crate = "screen-time";
                  id = "screen-time";
                  bin = "screen-time";
                  description = "Local, per-user screen-time tracker with a Rofi timeline (aarch64 cross-build)";
                  extraInstall = ''
                    install -Dm755 crates/screen-time/share/rofi-screen-time \
                      "$out/bin/rofi-screen-time"
                    install -Dm644 crates/screen-time/systemd/screen-time.service \
                      "$out/systemd/screen-time.service"
                  '';
                };
                rofi-hub-menu-aarch64 = mkMenu crossAarch64;
              }
            else
              { };

          rofiHubInstaller = pkgs.runCommandLocal "rofi-hub-installer-${installerVersion}" { } ''
            mkdir -p "$out/bin" "$out/share/rofi-hub-installer"
            install -m755 ${./install.sh} "$out/bin/rofi-hub-install"
            install -m755 ${./uninstall.sh} "$out/bin/rofi-hub-uninstall"
            install -m644 ${./minisign.pub} "$out/share/rofi-hub-installer/minisign.pub"
            install -Dm644 ${./installer/index.schema.json} \
              "$out/share/rofi-hub-installer/index.schema.json"
          '';

          # Compatibility bundle: independent apps joined into one profile.
          # No app depends on this derivation.
          rofiHubApps = pkgs.symlinkJoin {
            name = "rofi-hub-apps-${crateVersion "rofi-calendar"}";
            paths = [
              rofiCalendar
              rofiClipboard
              rofiKeybinds
              rofiNetwork
              atomicNote
              screenTime
            ];
            meta.description = "Compatibility bundle of every rofi-app";
          };

          allInOne = pkgs.symlinkJoin {
            name = "rofi-hub-all-in-one-${crateVersion "rofi-hub-menu"}";
            paths = [
              rofiHubMenu
              rofiHubApps
            ];
            meta.description = "Compatibility bundle: optional menu plus every app";
          };
        in
        {
          default = rofiHubMenu;
          inherit
            rofiCalendar
            rofiClipboard
            rofiKeybinds
            rofiNetwork
            screenTime
            rofiHubMenu
            rofiHubInstaller
            rofiHubApps
            ;
          rofi-calendar = rofiCalendar;
          rofi-clipboard = rofiClipboard;
          rofi-keybinds = rofiKeybinds;
          rofi-network = rofiNetwork;
          atomic-note = atomicNote;
          screen-time = screenTime;
          rofi-hub-menu = rofiHubMenu;
          rofi-hub-installer = rofiHubInstaller;
          rofi-hub-apps = rofiHubApps;
          rofi-apps = rofiHubApps;
          rofi-hub = allInOne;
          rofi-hub-only = rofiHubMenu;
          rofi-hub-all-in-one = allInOne;
        }
        // crossApps
      );

      devShells = forAllSystems (
        { pkgs }: {
          default = pkgs.mkShell {
            packages = with pkgs; [
              cargo
              clippy
              jq
              minisign
              python3
              rofi
              rustc
              rustfmt
              shellcheck
              util-linux
            ];
          };
        }
      );

      nixosModules = {
        rofi-calendar = import ./modules/nixos/app.nix {
          inherit self;
          app = "calendar";
          package = "rofi-calendar";
        };
        rofi-clipboard = import ./modules/nixos/app.nix {
          inherit self;
          app = "clipboard";
          package = "rofi-clipboard";
        };
        rofi-keybinds = import ./modules/nixos/app.nix {
          inherit self;
          app = "keybinds";
          package = "rofi-keybinds";
        };
        rofi-network = import ./modules/nixos/app.nix {
          inherit self;
          app = "network";
          package = "rofi-network";
        };
        atomic-note = import ./modules/nixos/app.nix {
          inherit self;
          app = "atomic-note";
          package = "atomic-note";
        };
        screen-time = import ./modules/nixos/screen-time.nix { inherit self; };
        rofi-hub-menu = import ./modules/nixos/menu.nix { inherit self; };
        rofi-apps = import ./modules/nixos/default.nix { inherit self; };
        default = import ./modules/nixos/default.nix { inherit self; };
        # Deprecated aliases: map services.rofi-hub.* to per-app options.
        rofi-hub = {
          imports = [
            self.nixosModules.rofi-apps
            (import ./modules/nixos/compat.nix { inherit self; })
          ];
        };
      };

      homeManagerModules = {
        rofi-calendar = import ./modules/home-manager/app.nix {
          inherit self;
          app = "calendar";
          package = "rofi-calendar";
        };
        rofi-clipboard = import ./modules/home-manager/app.nix {
          inherit self;
          app = "clipboard";
          package = "rofi-clipboard";
        };
        rofi-keybinds = import ./modules/home-manager/app.nix {
          inherit self;
          app = "keybinds";
          package = "rofi-keybinds";
        };
        rofi-network = import ./modules/home-manager/app.nix {
          inherit self;
          app = "network";
          package = "rofi-network";
        };
        atomic-note = import ./modules/home-manager/app.nix {
          inherit self;
          app = "atomic-note";
          package = "atomic-note";
        };
        screen-time = import ./modules/home-manager/screen-time.nix { inherit self; };
        rofi-hub-menu = import ./modules/home-manager/menu.nix { inherit self; };
        rofi-apps = import ./modules/home-manager/default.nix { inherit self; };
        default = import ./modules/home-manager/default.nix { inherit self; };
        rofi-hub = {
          imports = [
            self.homeManagerModules.rofi-apps
            (import ./modules/home-manager/compat.nix { inherit self; })
          ];
        };
      };

      checks = forAllSystems (
        { pkgs }:
        let
          packages = self.packages.${pkgs.stdenv.hostPlatform.system};
          moduleFor =
            module: settings:
            (nixpkgs.lib.nixosSystem {
              system = pkgs.stdenv.hostPlatform.system;
              modules = [
                module
                settings
                { system.stateVersion = "26.05"; }
              ];
            }).config;
          nixos = moduleFor self.nixosModules.rofi-apps {
            services.rofi-apps.calendar.enable = true;
            services.rofi-apps.screen-time.enable = true;
            services.rofi-apps.menu.enable = true;
          };
          calendarOnly = moduleFor self.nixosModules.rofi-calendar {
            services.rofi-apps.calendar.enable = true;
          };
          screenTimeOnly = moduleFor self.nixosModules.screen-time {
            services.rofi-apps.screen-time.enable = true;
          };
          menuOnly = moduleFor self.nixosModules.rofi-hub-menu {
            services.rofi-apps.menu.enable = true;
          };
          compat = nixpkgs.lib.nixosSystem {
            system = pkgs.stdenv.hostPlatform.system;
            modules = [
              self.nixosModules.rofi-hub
              {
                services.rofi-hub.enable = true;
                services.rofi-hub.apps = [ "calendar" ];
              }
            ];
          };
        in
        {
          default = packages.rofi-hub-menu;
          inherit (packages)
            rofi-calendar
            rofi-clipboard
            rofi-keybinds
            rofi-network
            screen-time
            rofi-hub-menu
            rofi-hub-installer
            ;
          module-eval = pkgs.writeText "rofi-apps-module-eval" (
            builtins.concatStringsSep "\n" (map (package: package.outPath) nixos.environment.systemPackages)
          );
          module-calendar-eval = pkgs.writeText "rofi-apps-calendar-eval" (
            builtins.concatStringsSep "\n" (
              map (package: package.outPath) calendarOnly.environment.systemPackages
            )
          );
          module-paths-eval =
            assert builtins.elem "/share/rofi" calendarOnly.environment.pathsToLink;
            assert builtins.elem "/share/rofi-apps" calendarOnly.environment.pathsToLink;
            assert builtins.elem "/share/rofi" menuOnly.environment.pathsToLink;
            assert builtins.elem "/share/rofi-apps" menuOnly.environment.pathsToLink;
            pkgs.writeText "rofi-apps-paths-eval" (
              builtins.concatStringsSep "\n" calendarOnly.environment.pathsToLink
            );
          module-screen-time-eval = pkgs.writeText "rofi-apps-screen-time-eval" (
            builtins.concatStringsSep "\n" (
              map (package: package.outPath) screenTimeOnly.environment.systemPackages
            )
            + "\n"
            + screenTimeOnly.systemd.user.services.screen-time.serviceConfig.ExecStart
          );
          module-menu-eval = pkgs.writeText "rofi-apps-menu-eval" (
            builtins.concatStringsSep "\n" (map (package: package.outPath) menuOnly.environment.systemPackages)
          );
          compat-module-eval = pkgs.writeText "rofi-apps-compat-eval" (
            builtins.concatStringsSep "\n" (
              map (package: package.outPath) compat.config.environment.systemPackages
            )
          );
        }
      );
    };
}
