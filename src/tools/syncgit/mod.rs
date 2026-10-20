mod sync;

use crate::errors::{CliError, Result, usage};
use crate::io;
use crate::tool::Tool;
use clap::Parser;
use inquire::validator::Validation;
use inquire::{Confirm, MultiSelect, Select, Text};
use std::ffi::OsString;
use std::io::IsTerminal;
use std::path::Path;
use std::process::Command;
use sync::{SyncGitOptions, resolve, source_default_branch, sync_git};

pub static TOOL: Tool = Tool {
    name: "syncgit",
    aliases: &[],
    description: "Sinkron commit dan tag antar remote Git dengan pemetaan mailmap",
    category: "Git",
    run,
};

const HELP: &str = "dt syncgit [options] [branch ...]

Tanpa opsi: wizard Inquirer. Dengan opsi: langsung berjalan, cocok untuk CI.
  --repo <path>           Repo kerja (default: direktori saat ini)
  --source <remote>       Remote sumber (default: github)
  --destination <remote>  Remote push (default: PUSH_REMOTE atau origin)
  --mailmap <path>        Mailmap, relatif ke repo (default: mailmap.txt)
  -n, --dry-run           Preview; fetch tetap dilakukan, tidak commit/tag/push
  -p, --push              Push branch dan tag setelah sinkronisasi lokal
  -d, --delete-tags       Hapus SEMUA tag lokal yang tidak ada di sumber;
                         bila --push, hapus juga tag bernama sama di tujuan
  -a, --all-branches      Sinkron SEMUA branch sumber; branch baru dibuat lokal
      --delete-branches   Hapus branch lokal yang tidak ada di sumber;
                         bila --push, hapus juga branch bernama sama di tujuan
  -f, --force             Bila history branch di-rewrite di sumber, bangun ulang
                         dari sumber (tip lama dicadangkan; push dengan lease)
      --mirror            Sama dengan --all-branches --delete-branches --delete-tags
  -i, --interactive       Buka wizard dengan opsi sebagai nilai awal
  -h, --help              Tampilkan bantuan

Tanpa branch: memakai branch default remote sumber (HEAD, mis. main/master).
Tag lama yang dipindah diperbarui dengan lease; branch hanya di-force-push (dengan lease) bila --force.
Exit: 0 sukses, 1 gagal, 2 penggunaan salah, 130 dibatalkan.
Contoh: dt syncgit --repo /path/to/repo -n main
";

#[derive(Parser, Debug)]
#[command(name = "dt syncgit", override_help = HELP, disable_version_flag = true, args_override_self = true)]
struct Args {
    #[arg(long)]
    repo: Option<String>,
    #[arg(long)]
    source: Option<String>,
    #[arg(long)]
    destination: Option<String>,
    #[arg(long)]
    mailmap: Option<String>,
    #[arg(short = 'n', long = "dry-run")]
    dry_run: bool,
    #[arg(short = 'p', long)]
    push: bool,
    #[arg(short = 'd', long = "delete-tags")]
    delete_tags: bool,
    #[arg(short = 'a', long = "all-branches")]
    all_branches: bool,
    #[arg(long = "delete-branches")]
    delete_branches: bool,
    #[arg(short = 'f', long)]
    force: bool,
    #[arg(long)]
    mirror: bool,
    #[arg(short = 'i', long)]
    interactive: bool,
    branches: Vec<String>,
}

struct Parsed {
    interactive: bool,
    options: SyncGitOptions,
}

/// Pure parsing seam: resolves defaults without touching Git, prompts, or the terminal.
fn parse_syncgit_args(args: Vec<OsString>, cwd: &Path, push_remote: Option<&str>) -> Result<Parsed> {
    let empty = args.is_empty();
    let parsed: Args = io::parse("syncgit", args)?;
    let repo = resolve(cwd, parsed.repo.as_deref().unwrap_or("."));
    let mut branches: Vec<String> = Vec::new();
    for branch in parsed.branches {
        if !branches.contains(&branch) {
            branches.push(branch);
        }
    }
    let all_branches = parsed.all_branches || parsed.mirror;
    if all_branches && !branches.is_empty() {
        return Err(usage(
            "--all-branches/--mirror tidak dapat digabung dengan nama branch.",
        ));
    }
    Ok(Parsed {
        interactive: empty || parsed.interactive,
        options: SyncGitOptions {
            repo: repo.to_string_lossy().into_owned(),
            source: parsed.source.unwrap_or_else(|| "github".to_string()),
            destination: parsed
                .destination
                .unwrap_or_else(|| push_remote.filter(|v| !v.is_empty()).unwrap_or("origin").to_string()),
            // Kept relative: sync_git resolves it against the repo, which the wizard may still change.
            mailmap: parsed.mailmap.unwrap_or_else(|| "mailmap.txt".to_string()),
            // Empty means the source remote's default branch, resolved by sync_git.
            branches,
            dry_run: parsed.dry_run,
            push: parsed.push,
            delete_tags: parsed.delete_tags || parsed.mirror,
            all_branches,
            delete_branches: parsed.delete_branches || parsed.mirror,
            force: parsed.force,
        },
    })
}

fn git_lines(repo: &str, args: &[&str]) -> Result<Vec<String>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .map_err(sync::spawn_error)?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(CliError::Failed(format!(
            "Command failed: git -C {repo} {}\n{stderr}",
            args.join(" ")
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .trim()
        .split('\n')
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect())
}

fn invalid(message: &str) -> Validation {
    Validation::Invalid(inquire::validator::ErrorMessage::Custom(message.to_string()))
}

fn position(items: &[String], wanted: &str) -> usize {
    items.iter().position(|i| i == wanted).unwrap_or(0)
}

fn wizard(initial: SyncGitOptions) -> Result<Option<SyncGitOptions>> {
    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
        return Err(CliError::Failed(
            "Wizard memerlukan terminal. Gunakan --repo <path> dan flag tanpa --interactive.".to_string(),
        ));
    }
    let cwd = std::env::current_dir()?;
    let repo_cwd = cwd.clone();
    let repo_answer = Text::new("Path repo kerja")
        .with_default(&initial.repo)
        .with_validator(move |value: &str| {
            let path = resolve(&repo_cwd, value.trim()).to_string_lossy().into_owned();
            Ok(match git_lines(&path, &["rev-parse", "--is-inside-work-tree"]) {
                Ok(result) if result.first().map(String::as_str) == Some("true") => Validation::Valid,
                Ok(_) => invalid("Gunakan repo Git dengan working tree."),
                Err(_) => invalid("Repo Git tidak ditemukan."),
            })
        })
        .prompt()?;
    let repo = resolve(&cwd, repo_answer.trim()).to_string_lossy().into_owned();
    let remotes = git_lines(&repo, &["remote"])?;
    if remotes.is_empty() {
        return Err(CliError::Failed("Repo belum memiliki remote Git.".to_string()));
    }
    let source = Select::new("Remote sumber (hanya dibaca)", remotes.clone())
        .with_starting_cursor(position(&remotes, &initial.source))
        .prompt()?;
    let destinations: Vec<String> = remotes.iter().filter(|name| **name != source).cloned().collect();
    if destinations.is_empty() {
        return Err(CliError::Failed(
            "Tambahkan remote tujuan yang berbeda dari sumber.".to_string(),
        ));
    }
    let destination = Select::new("Remote tujuan push", destinations.clone())
        .with_starting_cursor(position(&destinations, &initial.destination))
        .prompt()?;
    // initial.mailmap is still relative to whichever repo is chosen here, unless given as an absolute path.
    let repo_path = Path::new(&repo).to_path_buf();
    let validate_base = repo_path.clone();
    let mailmap_answer = Text::new("Path mailmap (relatif ke repo atau absolut)")
        .with_default(&initial.mailmap)
        .with_validator(move |value: &str| {
            Ok(if resolve(&validate_base, value.trim()).exists() {
                Validation::Valid
            } else {
                invalid("File mailmap tidak ditemukan.")
            })
        })
        .prompt()?;
    let mailmap = resolve(&repo_path, mailmap_answer.trim())
        .to_string_lossy()
        .into_owned();
    let all_branches = Confirm::new("Sinkron SEMUA branch sumber? Branch baru dibuat di lokal.")
        .with_default(initial.all_branches)
        .prompt()?;
    let branches: Vec<String> = if all_branches {
        Vec::new()
    } else {
        let heads = git_lines(&repo, &["for-each-ref", "--format=%(refname:strip=2)", "refs/heads"])?;
        let missing: Vec<&str> = initial
            .branches
            .iter()
            .filter(|b| !heads.contains(b))
            .map(String::as_str)
            .collect();
        if !missing.is_empty() {
            eprintln!("Branch lokal tidak ada, tidak dapat dipilih: {}", missing.join(", "));
        }
        // Preselect the source's default branch when none were given; a lookup failure only drops the preselection.
        if initial.branches.is_empty() {
            println!("Membaca branch default {source} (ls-remote)...");
        }
        let preselected: Vec<String> = if initial.branches.is_empty() {
            vec![source_default_branch(&repo, &source).unwrap_or_default()]
        } else {
            initial.branches.clone()
        };
        let defaults: Vec<usize> = heads
            .iter()
            .enumerate()
            .filter(|(_, h)| preselected.contains(h))
            .map(|(i, _)| i)
            .collect();
        MultiSelect::new("Branch yang disinkronkan", heads.clone())
            .with_default(&defaults)
            .with_validator(|selected: &[inquire::list_option::ListOption<&String>]| {
                Ok(if selected.is_empty() {
                    invalid("Pilih minimal satu branch.")
                } else {
                    Validation::Valid
                })
            })
            .prompt()?
    };
    let mode_labels = vec![
        "Preview / dry-run (Recommended)".to_string(),
        "Sinkron lokal saja".to_string(),
        format!("Sinkron lokal dan push ke {destination}"),
    ];
    let default_mode = if initial.dry_run {
        0
    } else if initial.push {
        2
    } else {
        0
    };
    let mode = Select::new("Mode sinkronisasi", mode_labels)
        .with_starting_cursor(default_mode)
        .raw_prompt()?
        .index;
    let delete_tags = Confirm::new("Hapus SEMUA tag lokal yang tidak ada di sumber? Termasuk tag buatan manual.")
        .with_default(initial.delete_tags)
        .prompt()?;
    let delete_branches = Confirm::new("Hapus branch lokal yang tidak ada di sumber?")
        .with_default(initial.delete_branches)
        .prompt()?;
    let force = Confirm::new("Bangun ulang branch dari sumber bila history-nya di-rewrite (--force)?")
        .with_default(initial.force)
        .prompt()?;
    println!(
        "\n{source} -> lokal{}\nRepo: {repo}\nBranch: {}\nMailmap: {mailmap}\nHapus tag: {}, hapus branch: {}, force: {}\n",
        if mode == 2 {
            format!(" -> {destination}")
        } else {
            String::new()
        },
        if all_branches {
            "semua".to_string()
        } else {
            branches.join(", ")
        },
        if delete_tags { "ya" } else { "tidak" },
        if delete_branches { "ya" } else { "tidak" },
        if force { "ya" } else { "tidak" }
    );
    if mode != 0 && !Confirm::new("Jalankan perubahan ini?").with_default(false).prompt()? {
        println!("Dibatalkan; repo lokal dan remote tidak diubah.");
        return Ok(None);
    }
    Ok(Some(SyncGitOptions {
        repo,
        source,
        destination,
        mailmap,
        branches,
        dry_run: mode == 0,
        push: mode == 2,
        delete_tags,
        all_branches,
        delete_branches,
        force,
    }))
}

fn run(args: Vec<OsString>) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let push_remote = std::env::var("PUSH_REMOTE").ok();
    let parsed = parse_syncgit_args(args, &cwd, push_remote.as_deref())?;
    let options = if parsed.interactive {
        match wizard(parsed.options)? {
            Some(o) => o,
            None => return Ok(()),
        }
    } else {
        parsed.options
    };
    // Only pushing writes to the destination; without --push it is unused and may equal the source.
    if options.push && options.source == options.destination {
        return Err(usage(
            "Remote sumber dan tujuan harus berbeda untuk --push: syncgit tidak menulis ke sumber.",
        ));
    }
    sync_git(&options)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cwd() -> std::path::PathBuf {
        std::path::PathBuf::from(if cfg!(windows) {
            "C:\\work\\current"
        } else {
            "/work/current"
        })
    }

    fn parse(args: &[&str], push_remote: Option<&str>) -> Result<Parsed> {
        parse_syncgit_args(args.iter().map(OsString::from).collect(), &cwd(), push_remote)
    }

    fn opts(args: &[&str]) -> SyncGitOptions {
        parse(args, None).map(|p| p.options).expect("parse")
    }

    #[test]
    fn defaults_resolve_against_cwd_and_never_enable_writes() {
        assert_eq!(
            opts(&["-n"]),
            SyncGitOptions {
                repo: cwd().to_string_lossy().into_owned(),
                source: "github".into(),
                destination: "origin".into(),
                mailmap: "mailmap.txt".into(),
                branches: vec![],
                dry_run: true,
                push: false,
                delete_tags: false,
                all_branches: false,
                delete_branches: false,
                force: false,
            }
        );
    }

    #[test]
    fn relative_repo_resolves_from_cwd_and_mailmap_stays_as_given() {
        let expected = cwd().parent().expect("fixture parent").join("other");
        assert_eq!(opts(&["--repo", "../other"]).repo, expected.to_string_lossy());
        assert_eq!(opts(&["--repo=../other"]).repo, expected.to_string_lossy());
        assert_eq!(
            opts(&["--repo", "../other", "--mailmap", "ids/map.txt"]).mailmap,
            "ids/map.txt"
        );
        let absolute = cwd().join("map.txt").to_string_lossy().into_owned();
        assert_eq!(opts(&["--mailmap", &absolute]).mailmap, absolute);
    }

    #[test]
    fn destination_precedence_flag_over_push_remote_over_origin() {
        let dest = |args: &[&str], env: Option<&str>| parse(args, env).map(|p| p.options.destination).expect("parse");
        assert_eq!(dest(&["-n"], None), "origin");
        assert_eq!(dest(&["-n"], Some("mirror")), "mirror");
        assert_eq!(dest(&["-n"], Some("")), "origin");
        assert_eq!(dest(&["--destination", "gitlab"], Some("mirror")), "gitlab");
    }

    #[test]
    fn no_branch_arguments_leaves_branches_empty_for_source_default_detection() {
        assert!(opts(&["-n"]).branches.is_empty());
    }

    #[test]
    fn positional_branches_keep_order_and_drop_duplicates() {
        assert_eq!(opts(&["main", "release", "main"]).branches, ["main", "release"]);
        assert_eq!(opts(&["-n", "--", "-odd"]).branches, ["-odd"]);
    }

    #[test]
    fn short_and_long_flags_are_equivalent_and_combinable() {
        for args in [vec!["-npd"], vec!["--dry-run", "--push", "--delete-tags"]] {
            let o = opts(&args);
            assert!(o.dry_run && o.push && o.delete_tags, "{args:?}");
        }
    }

    #[test]
    fn full_sync_flags_work_alone_and_combined() {
        let all = opts(&["-a"]);
        assert!(all.all_branches && !all.delete_branches && !all.force && !all.delete_tags);
        assert!(opts(&["-f"]).force);
        assert!(opts(&["--delete-branches"]).delete_branches);
        let combined = opts(&["-af", "--delete-branches"]);
        assert!(combined.all_branches && combined.force && combined.delete_branches);
    }

    #[test]
    fn mirror_expands_to_full_sync_and_conflicts_with_branch_names() {
        let o = opts(&["--mirror"]);
        assert!(o.all_branches && o.delete_branches && o.delete_tags && !o.force);
        assert!(matches!(parse(&["--mirror", "main"], None), Err(CliError::Usage(_))));
        assert!(matches!(parse(&["-a", "main"], None), Err(CliError::Usage(_))));
    }

    #[test]
    fn wizard_opens_with_no_arguments_or_interactive() {
        let interactive = |args: &[&str]| parse(args, None).map(|p| p.interactive).expect("parse");
        assert!(interactive(&[]));
        assert!(interactive(&["-i", "--repo", "x"]));
        assert!(!interactive(&["--repo", "x"]));
        assert!(!interactive(&["main"]));
    }

    #[test]
    fn help_exits_zero_without_requiring_other_options() {
        assert!(matches!(parse(&["-h"], None), Err(CliError::Exit(0))));
        assert!(matches!(
            parse(&["--help", "--repo", "x"], None),
            Err(CliError::Exit(0))
        ));
        assert!(parse(&["-n"], None).is_ok());
    }

    #[test]
    fn usage_errors_exit_two() {
        for args in [&["--nope"][..], &["--repo"], &["--push=yes"]] {
            assert!(matches!(parse(args, None), Err(CliError::Usage(_))), "{args:?}");
        }
    }
}
