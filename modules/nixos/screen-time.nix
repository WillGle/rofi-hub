{ self }:
{
  config,
  lib,
  pkgs,
  ...
}:
let
  system = pkgs.stdenv.hostPlatform.system;
  cfg = config.services.rofi-apps.screen-time;
  defaultPackage = self.packages.${system}.screen-time;
in
{
  options.services.rofi-apps.screen-time = {
    enable = lib.mkEnableOption "the Screen Time tracker and its Rofi timeline";

    package = lib.mkOption {
      type = lib.types.package;
      default = defaultPackage;
      defaultText = lib.literalExpression "self.packages.\${system}.screen-time";
      description = "The Screen Time package. It owns the user service unit.";
    };
  };

  config = lib.mkIf cfg.enable {
    environment.systemPackages = [ cfg.package ];

    systemd.user.services.screen-time = {
      enable = true;
      description = "Screen Time tracker";
      after = [ "graphical-session.target" ];
      partOf = [ "graphical-session.target" ];
      wantedBy = [ "graphical-session.target" ];
      serviceConfig = {
        ExecStart = "${cfg.package}/bin/screen-time daemon";
        Restart = "always";
        RestartSec = "2s";
        StateDirectory = "screen-time";
        RuntimeDirectory = "screen-time";
        NoNewPrivileges = true;
        PrivateTmp = true;
      };
    };
  };
}
