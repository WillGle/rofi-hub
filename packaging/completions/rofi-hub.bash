# bash completion for rofi-hub
_rofi_hub() {
    local cur prev
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"

    case "$prev" in
        --theme)
            COMPREPLY=($(compgen -W "auto nord solarized gruvbox" -- "$cur"))
            return 0
            ;;
        theme)
            COMPREPLY=($(compgen -W "show set list dump" -- "$cur"))
            return 0
            ;;
    esac

    if [ "$COMP_CWORD" -eq 1 ]; then
        COMPREPLY=($(compgen -W "open list theme doctor --theme --no-theme --help --version" -- "$cur"))
        return 0
    fi

    case "${COMP_WORDS[1]}" in
        theme)
            if [ "$COMP_CWORD" -eq 2 ]; then
                COMPREPLY=($(compgen -W "show set list dump" -- "$cur"))
            elif [ "${COMP_WORDS[2]}" = "set" ]; then
                COMPREPLY=($(compgen -W "auto nord solarized gruvbox" -- "$cur"))
            fi
            ;;
        list|doctor)
            COMPREPLY=($(compgen -W "--json" -- "$cur"))
            ;;
        open)
            COMPREPLY=($(compgen -W "--theme --no-theme" -- "$cur"))
            ;;
    esac
}

complete -F _rofi_hub rofi-hub
