{ self, app, package }:
{
  config,
  lib,
  pkgs,
  ...
}:
let
  system = pkgs.stdenv.hostPlatform.system;
  cfg = config.services.rofi-apps.${app};
  defaultPackage = self.packages.${system}.${package};
in
{
  options.services.rofi-apps.${app} = {
    enable = lib.mkEnableOption "the ${app} Rofi app";

    package = lib.mkOption {
      type = lib.types.package;
      default = defaultPackage;
      defaultText = lib.literalExpression "self.packages.\${system}.${package}";
      description = "The ${app} app package. It carries its own Rofi runtime dependency.";
    };
  };

  config = lib.mkIf cfg.enable {
    environment.systemPackages = [ cfg.package ];
  };
}
