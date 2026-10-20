mod common;

use common::{Out, dt_in_dir};
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

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

struct Cli {
    dir: TempDir,
    config: String,
}

fn setup() -> Cli {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("gitconfig");
    std::fs::write(&config, "[user]\n\tname = Test\n\temail = test@example.com\n").unwrap();
    Cli {
        config: config.to_string_lossy().into_owned(),
        dir,
    }
}

impl Cli {
    fn path(&self) -> PathBuf {
        common::canonical(self.dir.path())
    }

    /// run the CLI with no terminal at all, as cron/CI would
    fn cli(&self, args: &[&str]) -> Out {
        dt_in_dir(
            args,
            b"",
            &[("GIT_CONFIG_GLOBAL", &self.config), ("GIT_CONFIG_NOSYSTEM", "1")],
            Some(&self.path()),
        )
    }

    fn git(&self, cwd: &Path, args: &[&str]) -> String {
        let mut cmd = Command::new("git");
        cmd.arg("-C").arg(cwd).args(args).env_clear();
        for key in ["PATH", "SystemRoot"] {
            if let Some(value) = std::env::var_os(key) {
                cmd.env(key, value);
            }
        }
        let out = cmd
            .env("GIT_CONFIG_GLOBAL", &self.config)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .output()
            .unwrap();
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }
}

fn str_of(p: &Path) -> &str {
    p.to_str().unwrap()
}

#[test]
fn top_level_and_syncgit_help_succeed_and_list_the_registered_command() {
    let c = setup();
    let top = c.cli(&["--help"]);
    assert_eq!(top.code, 0);
    assert!(
        top.out
            .lines()
            .any(|l| l.starts_with("  syncgit      ") && !l["  syncgit      ".len()..].starts_with(' '))
    );
    let tool = c.cli(&["syncgit", "--help"]);
    assert_eq!(tool.code, 0);
    assert_eq!(tool.out, HELP);
    assert_eq!(tool.err, "");
}

#[test]
fn unknown_command_and_bad_option_exit_2() {
    let c = setup();
    assert_eq!(c.cli(&["nope"]).code, 2);
    let bad = c.cli(&["syncgit", "--nope"]);
    assert_eq!(bad.code, 2);
    assert!(bad.err.contains("--nope"), "{}", bad.err);
}

#[test]
fn no_terminal_menu_and_wizard_fail_fast_instead_of_hanging() {
    let c = setup();
    let menu = c.cli(&[]);
    assert_eq!(menu.code, 1);
    assert!(menu.err.contains("Menu memerlukan terminal"), "{}", menu.err);
    for args in [&["syncgit", "--interactive"][..], &["syncgit"]] {
        let wizard = c.cli(args);
        assert_eq!(wizard.code, 1);
        assert_eq!(
            wizard.err,
            "Wizard memerlukan terminal. Gunakan --repo <path> dan flag tanpa --interactive.\n"
        );
    }
}

#[test]
fn identical_source_and_destination_with_push_is_a_usage_error_rejected_before_touching_git() {
    let c = setup();
    let absent = c.path().join("absent");
    let r = c.cli(&[
        "syncgit",
        "--repo",
        str_of(&absent),
        "--source",
        "origin",
        "--destination",
        "origin",
        "-p",
    ]);
    assert_eq!(r.code, 2);
    assert!(r.err.contains("Remote sumber dan tujuan harus berbeda"), "{}", r.err);
}

#[test]
fn missing_mailmap_is_reported_with_its_resolved_path() {
    let c = setup();
    let dir = c.path();
    let repo = dir.join("no-mailmap");
    c.git(&dir, &["init", "-q", str_of(&repo)]);
    let r = c.cli(&["syncgit", "--repo", str_of(&repo), "-n"]);
    assert_eq!(r.code, 1);
    let expected = format!("mailmap tidak ada: {}", repo.join("mailmap.txt").display());
    assert!(r.err.contains(&expected), "{}", r.err);
}

fn make_pair(c: &Cli, branch: &str, source_name: &str, work_name: &str) -> (PathBuf, PathBuf) {
    let dir = c.path();
    let source = dir.join(source_name);
    let work = dir.join(work_name);
    c.git(&dir, &["init", "-q", "-b", branch, str_of(&source)]);
    std::fs::write(source.join("f.txt"), "0\n").unwrap();
    c.git(&source, &["add", "f.txt"]);
    c.git(&source, &["commit", "-qm", "initial"]);
    c.git(&dir, &["clone", "-q", "-o", "github", str_of(&source), str_of(&work)]);
    (source, work)
}

fn assert_plan(out: &str, branch: &str) {
    let marker = format!("== {branch}\n   titik sinkron: ");
    let pos = out.find(&marker).unwrap_or_else(|| panic!("no plan in: {out}"));
    let rest = &out[pos + marker.len()..];
    let (hash, rest) = rest.split_once(';').unwrap();
    assert!(!hash.is_empty() && hash.chars().all(|ch| ch.is_ascii_alphanumeric()));
    let rest = rest
        .strip_prefix(" commit baru:\n     ")
        .unwrap_or_else(|| panic!("bad: {rest}"));
    let (hash2, rest) = rest.split_once(' ').unwrap();
    assert!(!hash2.is_empty() && hash2.chars().all(|ch| ch.is_ascii_alphanumeric()));
    assert!(rest.starts_with("feature\n"), "{rest}");
}

#[test]
fn dry_run_from_a_different_cwd_uses_the_repo_mailmap_and_leaves_refs_untouched() {
    let c = setup();
    let (source, work) = make_pair(&c, "main", "source", "work");
    std::fs::write(
        work.join("mailmap.txt"),
        "Test <test@destination.example> <test@example.com>\n",
    )
    .unwrap();
    std::fs::write(source.join("f.txt"), "1\n").unwrap();
    c.git(&source, &["commit", "-qam", "feature"]);
    let before = c.git(&work, &["rev-parse", "main"]);

    let r = c.cli(&[
        "syncgit",
        "--repo",
        str_of(&work),
        "--destination",
        "gitlab",
        "-n",
        "main",
    ]);
    assert_eq!(r.code, 0, "{}", r.err);
    assert_plan(&r.out, "main");
    assert_eq!(c.git(&work, &["rev-parse", "main"]), before);

    // Without --push the destination is unused, so it may equal the source.
    let same = c.cli(&[
        "syncgit",
        "--repo",
        str_of(&work),
        "--source",
        "github",
        "--destination",
        "github",
        "-n",
        "main",
    ]);
    assert_eq!(same.code, 0, "{}", same.err);
}

#[test]
fn without_branch_arguments_the_source_default_branch_is_used_main_or_master() {
    for branch in ["main", "master"] {
        let c = setup();
        let (source, work) = make_pair(&c, branch, &format!("source-{branch}"), &format!("work-{branch}"));
        std::fs::write(work.join("mailmap.txt"), "").unwrap();
        std::fs::write(source.join("f.txt"), "1\n").unwrap();
        c.git(&source, &["commit", "-qam", "feature"]);

        let r = c.cli(&["syncgit", "--repo", str_of(&work), "--destination", "gitlab", "-n"]);
        assert_eq!(r.code, 0, "{}", r.err);
        assert!(
            r.out.contains(&format!(">> branch default github: {branch}\n")),
            "{branch}: {}",
            r.out
        );
        assert_plan(&r.out, branch);
    }
}
