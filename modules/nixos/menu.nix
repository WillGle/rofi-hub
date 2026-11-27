{ self }:
{
  config,
  lib,
  pkgs,
  ...
}:
let
  system = pkgs.stdenv.hostPlatform.system;
  cfg = config.services.rofi-apps.menu;
  defaultPackage = self.packages.${system}.rofi-hub-menu;
in
{
  options.services.rofi-apps.menu = {
    enable = lib.mkEnableOption "the optional Rofi Hub menu";

    package = lib.mkOption {
      type = lib.types.package;
      default = defaultPackage;
      defaultText = lib.literalExpression "self.packages.\${system}.rofi-hub-menu";
      description = ''
        The optional provider menu. It reads app manifests and execs apps
        with the user's environment. Removing it never breaks an app.
      '';
    };
  };

  config = lib.mkIf cfg.enable {
    environment.systemPackages = [ cfg.package ];
    # The menu reads app manifests and its layout from XDG_DATA_DIRS; make
    # both package directories visible on NixOS as well.
    environment.pathsToLink = [
      "/share/rofi"
      "/share/rofi-apps"
    ];
  };
}
