{ self }:
{
  config,
  lib,
  ...
}:
let
  cfg = config.services.rofi-hub;
in
{
  options.services.rofi-hub = {
    enable = lib.mkEnableOption "the legacy Rofi Hub module (deprecated)";

    package = lib.mkOption {
      type = lib.types.nullOr lib.types.package;
      default = null;
      description = ''
        Deprecated: the old bundle package. The compatibility module maps the
        selected apps to per-app packages and ignores this value.
      '';
    };

    apps = lib.mkOption {
      type = lib.types.listOf (
        lib.types.enum [
          "screen-time"
          "calendar"
          "keybinds"
          "clipboard"
        ]
      );
      default = [ "screen-time" ];
      description = "Deprecated: sub-apps to enable through services.rofi-apps.";
    };

    providerDirs = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      default = [ ];
      description = ''
        Deprecated and ignored. Provider manifests are owned by app packages
        and discovered through XDG_DATA_DIRS.
      '';
    };
  };

  config = lib.mkIf cfg.enable (
    lib.warn "services.rofi-hub is deprecated; use services.rofi-apps.<app>.enable" {
      services.rofi-apps.menu.enable = lib.mkDefault true;
      services.rofi-apps.calendar.enable = lib.mkDefault (builtins.elem "calendar" cfg.apps);
      services.rofi-apps.clipboard.enable = lib.mkDefault (builtins.elem "clipboard" cfg.apps);
      services.rofi-apps.keybinds.enable = lib.mkDefault (builtins.elem "keybinds" cfg.apps);
      services.rofi-apps.screen-time.enable = lib.mkDefault (
        builtins.elem "screen-time" cfg.apps
      );
    }
  );
}
