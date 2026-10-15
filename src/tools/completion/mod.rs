mod scripts;

use std::ffi::OsString;
use std::io::Write;

use clap::Parser;

use crate::errors::{Result, usage};
use crate::io::parse;
use crate::tool::Tool;
use scripts::{CompletionEntry, SHELLS, completion_script};

pub static TOOL: Tool = Tool {
    name: "completion",
    aliases: &[],
    description: "Cetak skrip auto-complete shell (bash, zsh, fish)",
    category: "Utilitas",
    run,
};

const HELP: &str = r##"dt completion <shell>

Cetak skrip auto-complete untuk nama tool dan alias (argumen pertama).
  shell   bash, zsh, fish
  -h, --help  Tampilkan bantuan

Pasang:
  bash   echo 'source <(dt completion bash)' >> ~/.bashrc
  zsh    dt completion zsh > "${fpath[1]}/_dt"   # lalu: autoload -U compinit && compinit
  fish   dt completion fish > ~/.config/fish/completions/dt.fish
"##;

#[derive(Parser)]
#[command(name = "dt completion", override_help = HELP, disable_version_flag = true)]
struct Args {
    positionals: Vec<String>,
}

fn run(args: Vec<OsString>) -> Result<()> {
    let a = parse::<Args>("completion", args)?;
    let script = match a.positionals.as_slice() {
        [shell] => {
            let entries: Vec<CompletionEntry> = crate::tools::TOOLS
                .iter()
                .flat_map(|tool| {
                    std::iter::once(tool.name)
                        .chain(tool.aliases.iter().copied())
                        .map(|name| CompletionEntry {
                            name: name.to_string(),
                            description: tool.description.to_string(),
                        })
                })
                .collect();
            completion_script(shell, &entries)
        }
        _ => None,
    };
    let Some(script) = script else {
        return Err(usage(format!("Tentukan shell: {}", SHELLS.join(", "))));
    };
    let mut out = std::io::stdout().lock();
    let _ = out.write_all(script.as_bytes());
    let _ = out.flush();
    Ok(())
}
