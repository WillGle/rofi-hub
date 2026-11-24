complete -c screen-time -f

complete -c screen-time -n '__fish_use_subcommand' -a daemon -d 'Run the per-user tracker daemon'
complete -c screen-time -n '__fish_use_subcommand' -a render -d 'Render the UI or app detail'
complete -c screen-time -n '__fish_use_subcommand' -a doctor -d 'Check runtime and database health'
complete -c screen-time -n '__fish_use_subcommand' -a snapshot -d 'Write a database snapshot'
complete -c screen-time -n '__fish_use_subcommand' -a check-data -d 'Compare against a snapshot'
complete -c screen-time -n '__fish_use_subcommand' -a schema-version -d 'Print the schema version'
complete -c screen-time -n '__fish_use_subcommand' -a checkpoint -d 'Checkpoint the WAL'
complete -c screen-time -n '__fish_use_subcommand' -a config -d 'Print or create the configuration'

complete -c screen-time -n '__fish_seen_subcommand_from render' -l format -xa 'rofi rofi-theme'
complete -c screen-time -n '__fish_seen_subcommand_from render' -l date -d 'View a local date'
complete -c screen-time -n '__fish_seen_subcommand_from render' -l app -d 'Open one app detail'
complete -c screen-time -n '__fish_seen_subcommand_from doctor' -l json -d 'Emit JSON'
complete -c screen-time -n '__fish_seen_subcommand_from doctor' -l no-runtime -d 'Skip runtime checks'
complete -c screen-time -n '__fish_seen_subcommand_from snapshot' -l to -d 'Destination directory' -r
complete -c screen-time -n '__fish_seen_subcommand_from snapshot' -l json -d 'Emit JSON'
complete -c screen-time -n '__fish_seen_subcommand_from check-data' -l against -d 'Snapshot directory' -r
complete -c screen-time -n '__fish_seen_subcommand_from check-data' -l allow-migration -d 'Accept a documented migration'
complete -c screen-time -n '__fish_seen_subcommand_from config' -a 'example path init'
