# bash completion for screen-time
_screen_time() {
    local cur prev
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"

    case "$prev" in
        --format)
            COMPREPLY=($(compgen -W "rofi rofi-theme" -- "$cur"))
            return 0
            ;;
        --date)
            COMPREPLY=($(compgen -W "$(date +%Y-%m-%d)" -- "$cur"))
            return 0
            ;;
        --to|--against)
            COMPREPLY=($(compgen -d -- "$cur"))
            return 0
            ;;
        config)
            COMPREPLY=($(compgen -W "example path init" -- "$cur"))
            return 0
            ;;
    esac

    if [ "$COMP_CWORD" -eq 1 ]; then
        COMPREPLY=($(compgen -W "daemon render doctor snapshot check-data schema-version checkpoint config --help --version" -- "$cur"))
        return 0
    fi

    case "${COMP_WORDS[1]}" in
        render)
            COMPREPLY=($(compgen -W "--format --date --app" -- "$cur"))
            ;;
        doctor)
            COMPREPLY=($(compgen -W "--json --no-runtime" -- "$cur"))
            ;;
        snapshot)
            COMPREPLY=($(compgen -W "--to --json --no-config-hash" -- "$cur"))
            ;;
        check-data)
            COMPREPLY=($(compgen -W "--against --json --allow-migration" -- "$cur"))
            ;;
        config)
            if [ "$COMP_CWORD" -eq 2 ]; then
                COMPREPLY=($(compgen -W "example path init" -- "$cur"))
            elif [ "${COMP_WORDS[2]}" = "init" ]; then
                COMPREPLY=($(compgen -W "--force" -- "$cur"))
            fi
            ;;
    esac
}

complete -F _screen_time screen-time
