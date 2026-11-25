complete -c rofi-hub -f

complete -c rofi-hub -n '__fish_use_subcommand' -a open -d 'Open the hub menu or one provider'
complete -c rofi-hub -n '__fish_use_subcommand' -a list -d 'List installed providers'
complete -c rofi-hub -n '__fish_use_subcommand' -a theme -d 'Show or change the hub theme'
complete -c rofi-hub -n '__fish_use_subcommand' -a doctor -d 'Check Rofi, providers, and themes'
complete -c rofi-hub -n '__fish_use_subcommand' -a launcher -d 'Open the Rofi drun launcher'
complete -c rofi-hub -n '__fish_use_subcommand' -a window -d 'Open the Rofi window switcher'
complete -c rofi-hub -n '__fish_use_subcommand' -a calc -d 'Open rofi-calc'
complete -c rofi-hub -n '__fish_use_subcommand' -a emoji -d 'Open rofi-emoji'
complete -c rofi-hub -n '__fish_use_subcommand' -a power -d 'Open rofi-power-menu'
complete -c rofi-hub -n '__fish_use_subcommand' -a network -d 'Not ported in this phase'
complete -c rofi-hub -n '__fish_use_subcommand' -a calendar -d 'Open the Calendar provider'
complete -c rofi-hub -n '__fish_use_subcommand' -a clipboard -d 'Open the Clipboard provider'
complete -c rofi-hub -n '__fish_use_subcommand' -a keybinds -d 'Open the Keyboard Shortcuts provider'
complete -c rofi-hub -n '__fish_use_subcommand' -a screen-time -d 'Open the Screen Time provider'
complete -c rofi-hub -n '__fish_use_subcommand' -a show -d 'Open a raw Rofi mode'

complete -c rofi-hub -l theme -d 'Theme override' -xa 'auto nord solarized gruvbox'
complete -c rofi-hub -l no-theme -d 'Do not pass a generated theme'

complete -c rofi-hub -n '__fish_seen_subcommand_from theme' -a 'show set list dump'
complete -c rofi-hub -n '__fish_seen_subcommand_from list doctor' -l json -d 'Emit JSON'
