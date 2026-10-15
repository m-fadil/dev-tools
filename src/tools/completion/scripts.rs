pub struct CompletionEntry {
    pub name: String,
    pub description: String,
}

pub const SHELLS: &[&str] = &["bash", "zsh", "fish"];

const BIN: &str = "dt";

// Single-quoted shell literal: close the quote, emit an escaped quote, reopen.
fn sq(text: &str) -> String {
    format!("'{}'", text.replace('\'', "'\\''"))
}

/// Completion script for the first argument only: tool names, aliases, and --help.
pub fn completion_script(shell: &str, entries: &[CompletionEntry]) -> Option<String> {
    Some(match shell {
        "bash" => {
            let mut words: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
            words.push("--help");
            format!(
                "_dt() {{\n    local cur=\"${{COMP_WORDS[COMP_CWORD]}}\"\n    if [ \"$COMP_CWORD\" -eq 1 ]; then\n        COMPREPLY=( $(compgen -W {} -- \"$cur\") )\n    fi\n}}\ncomplete -F _dt {BIN}\n",
                sq(&words.join(" "))
            )
        }
        "zsh" => {
            let lines: Vec<String> = entries
                .iter()
                .map(|e| {
                    format!(
                        "        {}",
                        sq(&format!("{}:{}", e.name, e.description.replace(':', "\\:")))
                    )
                })
                .collect();
            format!(
                "#compdef {BIN}\n_dt() {{\n    local -a tools\n    tools=(\n{}\n    )\n    if (( CURRENT == 2 )); then\n        _describe 'tool' tools\n    fi\n}}\n# Autoloaded from $fpath as _dt, zsh runs this file on the first Tab: call the completer then.\n# When sourced instead, only register it.\nif [[ $funcstack[1] == _dt ]]; then\n    _dt \"$@\"\nelse\n    compdef _dt {BIN}\nfi\n",
                lines.join("\n")
            )
        }
        "fish" => {
            let lines: Vec<String> = entries
                .iter()
                .map(|e| {
                    format!(
                        "complete -c {BIN} -f -n '__fish_use_subcommand' -a {} -d {}",
                        sq(&e.name),
                        sq(&e.description)
                    )
                })
                .collect();
            format!("{}\n", lines.join("\n"))
        }
        _ => return None,
    })
}
