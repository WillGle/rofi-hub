complete -c rofi-hub -f

complete -c rofi-hub -n '__fish_use_subcommand' -a open -d 'Open the hub menu or one provider'
complete -c rofi-hub -n '__fish_use_subcommand' -a list -d 'List installed providers'
complete -c rofi-hub -n '__fish_use_subcommand' -a theme -d 'Show or change the hub theme'
complete -c rofi-hub -n '__fish_use_subcommand' -a doctor -d 'Check Rofi, providers, and themes'

complete -c rofi-hub -l theme -d 'Theme override' -xa 'auto nord solarized gruvbox'
complete -c rofi-hub -l no-theme -d 'Do not pass a generated theme'

complete -c rofi-hub -n '__fish_seen_subcommand_from theme' -a 'show set list dump'
complete -c rofi-hub -n '__fish_seen_subcommand_from list doctor' -l json -d 'Emit JSON'
