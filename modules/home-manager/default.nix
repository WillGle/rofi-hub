{ self }:
{
  imports = [
    (import ./app.nix {
      inherit self;
      app = "calendar";
      package = "rofi-calendar";
    })
    (import ./app.nix {
      inherit self;
      app = "clipboard";
      package = "rofi-clipboard";
    })
    (import ./app.nix {
      inherit self;
      app = "keybinds";
      package = "rofi-keybinds";
    })
    (import ./app.nix {
      inherit self;
      app = "network";
      package = "rofi-network";
    })
    (import ./screen-time.nix { inherit self; })
    (import ./menu.nix { inherit self; })
  ];
}
