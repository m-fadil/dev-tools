export interface CompletionEntry {
    name: string;
    description: string;
}

export const SHELLS = ['bash', 'zsh', 'fish'] as const;
export type Shell = (typeof SHELLS)[number];

const BIN = 'dt';

// Single-quoted shell literal: close the quote, emit an escaped quote, reopen.
const sq = (text: string): string => `'${text.replaceAll("'", `'\\''`)}'`;

/** Completion script for the first argument only: tool names, aliases, and --help. */
export function completionScript(shell: Shell, entries: readonly CompletionEntry[]): string {
    switch (shell) {
        case 'bash':
            return `_dt() {
    local cur="\${COMP_WORDS[COMP_CWORD]}"
    if [ "$COMP_CWORD" -eq 1 ]; then
        COMPREPLY=( $(compgen -W ${sq([...entries.map((e) => e.name), '--help'].join(' '))} -- "$cur") )
    fi
}
complete -F _dt ${BIN}
`;
        case 'zsh':
            return `#compdef ${BIN}
_dt() {
    local -a tools
    tools=(
${entries.map((e) => `        ${sq(`${e.name}:${e.description.replaceAll(':', '\\:')}`)}`).join('\n')}
    )
    if (( CURRENT == 2 )); then
        _describe 'tool' tools
    fi
}
# Autoloaded from $fpath as _dt, zsh runs this file on the first Tab: call the completer then.
# When sourced instead, only register it.
if [[ $funcstack[1] == _dt ]]; then
    _dt "$@"
else
    compdef _dt ${BIN}
fi
`;
        case 'fish':
            return `${entries
                .map((e) => `complete -c ${BIN} -f -n '__fish_use_subcommand' -a ${sq(e.name)} -d ${sq(e.description)}`)
                .join('\n')}
`;
    }
}
