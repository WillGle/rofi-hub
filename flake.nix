{
  description = "Rofi Hub: a small Rust launcher for screen-time and future Rofi sub-apps";

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
      workspaceVersion = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).workspace.package.version;
      screenTimeVersion = (builtins.fromTOML (builtins.readFile ./crates/screen-time/Cargo.toml)).package.version;
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
              ]);
          };
          common = {
            inherit src;
            cargoLock.lockFile = ./Cargo.lock;
            nativeBuildInputs = [
              pkgs.makeWrapper
              pkgs.tzdata
            ];
            doCheck = true;
          };

          rofiHub = pkgs.rustPlatform.buildRustPackage (
            common
            // {
              pname = "rofi-hub";
              version = workspaceVersion;
              cargoBuildFlags = [ "--package" "rofi-hub" ];
              cargoTestFlags = [ "--package" "rofi-hub" ];
              postInstall = ''
                mkdir -p "$out/share/rofi-hub/themes"
                install -m 644 themes/hub.rasi "$out/share/rofi-hub/themes/hub.rasi"
              '';
              meta = {
                description = "Rofi launcher for screen-time and future sub-apps";
                license = lib.licenses.mit;
                mainProgram = "rofi-hub";
                platforms = systems;
              };
            }
          );

          screenTime = pkgs.rustPlatform.buildRustPackage (
            common
            // {
              pname = "screen-time";
              version = screenTimeVersion;
              cargoBuildFlags = [ "--package" "screen-time" ];
              cargoTestFlags = [ "--package" "screen-time" ];
              preCheck = ''
                export TZ=UTC
                export TZDIR=${pkgs.tzdata}/share/zoneinfo
              '';
              postInstall = ''
                mkdir -p "$out/share/rofi" "$out/share/screen-time" \
                  "$out/share/rofi-hub/providers" "$out/systemd"
                install -m 644 crates/screen-time/share/rofi/screen-time.rasi \
                  "$out/share/rofi/screen-time.rasi"
                install -m 644 crates/screen-time/share/screen-time/config.example.toml \
                  "$out/share/screen-time/config.example.toml"
                install -m 644 providers/screen-time.toml \
                  "$out/share/rofi-hub/providers/screen-time.toml"
                install -m 644 crates/screen-time/systemd/screen-time.service \
                  "$out/systemd/screen-time.service"
                install -m 755 crates/screen-time/share/rofi-screen-time "$out/bin/rofi-screen-time"
                wrapProgram "$out/bin/rofi-screen-time" \
                  --set SCREEN_TIME_THEME "$out/share/rofi/screen-time.rasi"
                substituteInPlace "$out/systemd/screen-time.service" \
                  --replace-fail "ExecStart=%h/.local/bin/screen-time daemon" \
                  "ExecStart=$out/bin/screen-time daemon"
              '';
              meta = {
                description = "Local, per-user screen-time tracker with a static Rofi day timeline";
                license = lib.licenses.mit;
                mainProgram = "screen-time";
                platforms = systems;
              };
            }
          );

          allInOne = pkgs.symlinkJoin {
            name = "rofi-hub-all-in-one-${workspaceVersion}";
            paths = [
              rofiHub
              screenTime
            ];
            nativeBuildInputs = [ pkgs.makeWrapper ];
            postBuild = ''
              wrapProgram "$out/bin/rofi-hub" \
                --prefix PATH : "${screenTime}/bin" \
                --set-default ROFI_HUB_PROVIDER_DIR "${screenTime}/share/rofi-hub/providers"
            '';
            meta = rofiHub.meta // {
              mainProgram = "rofi-hub";
            };
          };
        in
        {
          default = allInOne;
          rofi-hub = allInOne;
          rofi-hub-only = rofiHub;
          screen-time = screenTime;
        }
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

      homeManagerModules.default =
        {
          config,
          lib,
          pkgs,
          ...
        }:
        let
          cfg = config.services.rofi-hub;
          system = pkgs.stdenv.hostPlatform.system;
          allInOne = self.packages.${system}.default;
          hubOnly = self.packages.${system}.rofi-hub-only;
          screenTime = self.packages.${system}.screen-time;
          withScreenTime = builtins.elem "screen-time" cfg.apps;
          package = if cfg.package != null then cfg.package else if withScreenTime then allInOne else hubOnly;
        in
        {
          options.services.rofi-hub = {
            enable = lib.mkEnableOption "the Rofi Hub launcher";

            package = lib.mkOption {
              type = lib.types.nullOr lib.types.package;
              default = null;
              description = ''
                The rofi-hub package to install. The default installs every
                selected app; with an empty `apps` list it installs only the
                hub launcher.
              '';
            };

            apps = lib.mkOption {
              type = lib.types.listOf (lib.types.enum [ "screen-time" ]);
              default = [ "screen-time" ];
              description = ''
                Sub-apps to install with the hub. The screen-time tracker
                service is enabled when `screen-time` is selected.
              '';
            };

            providerDirs = lib.mkOption {
              type = lib.types.listOf lib.types.str;
              default = [ ];
              description = ''
                Extra directories to scan for provider manifests. The hub
                already scans the standard XDG data directories.
              '';
            };
          };

          config = lib.mkIf cfg.enable {
            home.packages = [ package ];

            home.sessionVariables = lib.mkIf (cfg.providerDirs != [ ]) {
              ROFI_HUB_PROVIDER_DIR = lib.concatStringsSep ":" cfg.providerDirs;
            };

            systemd.user.services = lib.mkIf withScreenTime {
              screen-time = {
                Unit = {
                  Description = "Screen Time tracker";
                  After = [ "graphical-session.target" ];
                  PartOf = [ "graphical-session.target" ];
                };

                Service = {
                  ExecStart = "${package}/bin/screen-time daemon";
                  Restart = "always";
                  RestartSec = "2s";
                  StateDirectory = "screen-time";
                  RuntimeDirectory = "screen-time";
                };

                Install.WantedBy = [ "graphical-session.target" ];
              };
            };
          };
        };

      # Kept for users migrating from the standalone screen-time flake.
      homeManagerModules.screen-time =
        {
          config,
          lib,
          pkgs,
          ...
        }:
        let
          cfg = config.services.screen-time;
          package = self.packages.${pkgs.stdenv.hostPlatform.system}.screen-time;
        in
        {
          options.services.screen-time = {
            enable = lib.mkEnableOption "the Screen Time tracker";

            package = lib.mkOption {
              type = lib.types.package;
              default = package;
              description = "The screen-time package to install.";
            };
          };

          config = lib.mkIf cfg.enable {
            home.packages = [ cfg.package ];

            systemd.user.services.screen-time = {
              Unit = {
                Description = "Screen Time tracker";
                After = [ "graphical-session.target" ];
                PartOf = [ "graphical-session.target" ];
              };

              Service = {
                ExecStart = "${cfg.package}/bin/screen-time daemon";
                Restart = "always";
                RestartSec = "2s";
                StateDirectory = "screen-time";
                RuntimeDirectory = "screen-time";
              };

              Install.WantedBy = [ "graphical-session.target" ];
            };
          };
        };

      checks = forAllSystems (
        { pkgs }:
        let
          packages = self.packages.${pkgs.stdenv.hostPlatform.system};
        in
        {
          default = packages.default;
          inherit (packages)
            rofi-hub
            rofi-hub-only
            screen-time
            ;
        }
      );
    };
}
