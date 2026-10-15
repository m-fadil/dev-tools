mod codec;
mod errors;
mod io;
mod numfmt;
mod random;
mod tool;
mod tools;

use std::ffi::OsString;
use std::io::IsTerminal;

use errors::{CliError, Result, usage};
use tool::Tool;
use tools::{TOOLS, find_tool, tools_by_category};

fn tool_line(tool: &Tool) -> String {
    let alias = if tool.aliases.is_empty() {
        String::new()
    } else {
        format!(" (alias: {})", tool.aliases.join(", "))
    };
    format!("  {:<12} {}{}", tool.name, tool.description, alias)
}

fn help_text() -> String {
    let groups: Vec<String> = tools_by_category()
        .into_iter()
        .map(|(category, list)| {
            let lines: Vec<String> = list.iter().map(|t| tool_line(t)).collect();
            format!("{category}:\n{}", lines.join("\n"))
        })
        .collect();
    format!(
        "dt <tool> [options]\n\nTanpa argumen: menu interaktif dengan pencarian. Ctrl+C: batalkan.\n\n{}\n\nGunakan dt <tool> --help untuk opsi tool.",
        groups.join("\n\n")
    )
}

fn menu() -> Result<Option<&'static str>> {
    const EXIT: &str = "Keluar";
    let mut items: Vec<String> = TOOLS.iter().map(|t| format!("{} › {}", t.category, t.name)).collect();
    items.push(EXIT.to_string());
    let scorer = |input: &str, _: &String, label: &str, idx: usize| -> Option<i64> {
        let q = input.trim().to_lowercase();
        let hit = match TOOLS.get(idx) {
            Some(t) => {
                q.is_empty()
                    || std::iter::once(t.name)
                        .chain(t.aliases.iter().copied())
                        .chain([t.description, t.category])
                        .any(|s| s.to_lowercase().contains(&q))
            }
            // The exit entry is only shown for an empty query.
            None => q.is_empty() && label == EXIT,
        };
        hit.then_some(0)
    };
    let picked = inquire::Select::new("Pilih developer tool (ketik untuk mencari)", items)
        .with_scorer(&scorer)
        .raw_prompt()?;
    Ok(TOOLS.get(picked.index).map(|t| t.name))
}

fn run(args: Vec<OsString>) -> Result<()> {
    let first = args.first().and_then(|a| a.to_str().map(str::to_owned));
    let args = if first.as_deref() == Some("") { Vec::new() } else { args };
    let first = first.filter(|f| !f.is_empty());
    if matches!(first.as_deref(), Some("-h" | "--help")) {
        println!("{}", help_text());
        return Ok(());
    }
    let name = match first {
        Some(n) => n,
        None if !args.is_empty() => {
            return Err(usage(format!("Tool tidak dikenal: {}", args[0].to_string_lossy())));
        }
        None => {
            if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
                println!("{}", help_text());
                return Err(CliError::Failed(
                    "Menu memerlukan terminal. Gunakan dt syncgit --repo <path> [options].".into(),
                ));
            }
            match menu()? {
                Some(n) => n.to_string(),
                None => return Ok(()),
            }
        }
    };
    let tool = find_tool(&name).ok_or_else(|| usage(format!("Tool tidak dikenal: {name}")))?;
    (tool.run)(args.into_iter().skip(1).collect())
}

fn main() {
    #[cfg(unix)]
    // SAFETY: restoring the default SIGPIPE disposition before any other work.
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
    // Windows main threads only get 1 MiB; deeply nested input (math, json) needs more headroom.
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    let result = std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .spawn(move || run(args))
        .map_err(CliError::from)
        .and_then(|handle| {
            handle
                .join()
                .unwrap_or_else(|_| Err(CliError::Failed("Terjadi kesalahan internal.".into())))
        });
    let code = match result {
        Ok(()) => 0,
        Err(CliError::Usage(m)) => {
            eprintln!("{m}\n\nGunakan dt --help atau dt <tool> --help.");
            2
        }
        Err(CliError::Failed(m)) => {
            eprintln!("{m}");
            1
        }
        Err(CliError::Cancelled) => {
            eprintln!("Dibatalkan.");
            130
        }
        Err(CliError::Exit(c)) => i32::from(c),
    };
    std::process::exit(code);
}
