mod common;

use common::{Out, dt_env};
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

const D1: &str = "2026-01-01T10:00:00+00:00";
const D2: &str = "2026-01-02T10:00:00+00:00";
const D3: &str = "2026-01-03T10:00:00+00:00";

/// git `--date=raw` form of the UTC fixture dates above.
fn raw(date: &str) -> String {
    let epoch = match date {
        D1 => 1_767_261_600,
        D2 => 1_767_348_000,
        D3 => 1_767_434_400,
        _ => panic!("unknown date"),
    };
    format!("{epoch} +0000")
}

type Env = Vec<(String, String)>;

fn identity(date: &str, committer_date: &str) -> Vec<(&'static str, String)> {
    vec![
        ("GIT_AUTHOR_NAME", "Dev".into()),
        ("GIT_AUTHOR_EMAIL", "developer@source.example".into()),
        ("GIT_AUTHOR_DATE", date.into()),
        ("GIT_COMMITTER_NAME", "Dev".into()),
        ("GIT_COMMITTER_EMAIL", "developer@source.example".into()),
        ("GIT_COMMITTER_DATE", committer_date.into()),
    ]
}

struct Fixture {
    _dir: TempDir,
    env: Env,
    source: PathBuf,
    work: PathBuf,
    destination: PathBuf,
    mailmap: PathBuf,
    initial: String,
}

#[derive(Clone, Default)]
struct Sync {
    push: bool,
    dry_run: bool,
    delete_tags: bool,
}

impl Fixture {
    fn git_try(&self, repo: &Path, args: &[&str], extra: &[(&str, String)]) -> Result<String, String> {
        let mut cmd = Command::new("git");
        cmd.arg("-C").arg(repo).args(args).env_clear();
        for key in ["PATH", "SystemRoot"] {
            if let Some(value) = std::env::var_os(key) {
                cmd.env(key, value);
            }
        }
        for (k, v) in &self.env {
            cmd.env(k, v);
        }
        for (k, v) in extra {
            cmd.env(k, v);
        }
        let out = cmd.output().map_err(|e| e.to_string())?;
        if out.status.success() {
            Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
        } else {
            Err(String::from_utf8_lossy(&out.stderr).to_string())
        }
    }

    fn git(&self, repo: &Path, args: &[&str]) -> String {
        self.git_try(repo, args, &[])
            .unwrap_or_else(|e| panic!("git {args:?} failed: {e}"))
    }

    fn git_env(&self, repo: &Path, args: &[&str], env: &[(&str, String)]) -> String {
        self.git_try(repo, args, env)
            .unwrap_or_else(|e| panic!("git {args:?} failed: {e}"))
    }

    fn commit(&self, message: &str, content: &str, date: &str, committer_date: &str) -> String {
        std::fs::write(self.source.join("f.txt"), content).unwrap();
        self.git(&self.source, &["add", "--", "f.txt"]);
        self.git_env(
            &self.source,
            &["commit", "-q", "-m", message],
            &identity(date, committer_date),
        );
        self.git(&self.source, &["rev-parse", "HEAD"])
    }

    fn annotated_tag(&self, name: &str, date: &str, message: &str, reference: &str) {
        self.git_env(
            &self.source,
            &["tag", "-f", "-a", "-m", message, name, reference],
            &identity(date, date),
        );
    }

    fn refs(&self, repo: &Path) -> String {
        self.git(
            repo,
            &[
                "for-each-ref",
                "--format=%(refname) %(objectname)",
                "refs/heads",
                "refs/tags",
            ],
        )
    }

    fn run(&self, opts: &Sync) -> Out {
        let work = self.work.to_string_lossy().into_owned();
        let mailmap = self.mailmap.to_string_lossy().into_owned();
        let mut args = vec![
            "syncgit",
            "--repo",
            &work,
            "--source",
            "github",
            "--destination",
            "origin",
            "--mailmap",
            &mailmap,
        ];
        if opts.dry_run {
            args.push("-n");
        }
        if opts.push {
            args.push("-p");
        }
        if opts.delete_tags {
            args.push("-d");
        }
        args.push("develop");
        let env: Vec<(&str, &str)> = self.env.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        dt_env(&args, b"", &env)
    }

    fn sync(&self, opts: &Sync) {
        let r = self.run(opts);
        assert_eq!(r.code, 0, "stdout: {}\nstderr: {}", r.out, r.err);
    }
}

fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    let config = root.join("gitconfig");
    std::fs::write(
        &config,
        "[user]\n\tname = Test\n\temail = test@example.com\n[commit]\n\tgpgSign = false\n[tag]\n\tgpgSign = false\n[core]\n\tautocrlf = false\n",
    )
    .unwrap();
    let env: Env = vec![
        ("GIT_CONFIG_GLOBAL".into(), config.to_string_lossy().into_owned()),
        ("GIT_CONFIG_NOSYSTEM".into(), "1".into()),
        ("GIT_TERMINAL_PROMPT".into(), "0".into()),
        ("HOME".into(), root.to_string_lossy().into_owned()),
        (
            "XDG_CONFIG_HOME".into(),
            root.join(".config").to_string_lossy().into_owned(),
        ),
    ];
    let source = root.join("source");
    let work = root.join("work");
    let destination = root.join("destination.git");
    let mailmap = root.join("mailmap.txt");
    let mut f = Fixture {
        _dir: dir,
        env,
        source,
        work,
        destination,
        mailmap,
        initial: String::new(),
    };
    let (s, w, d) = (f.source.clone(), f.work.clone(), f.destination.clone());
    f.git(&root, &["init", "-q", "-b", "develop", s.to_str().unwrap()]);
    f.initial = f.commit("initial", "0\n", D1, D1);
    f.git(
        &root,
        &["clone", "-q", "--no-local", s.to_str().unwrap(), w.to_str().unwrap()],
    );
    f.git(&w, &["remote", "rename", "origin", "github"]);
    f.git(&root, &["init", "-q", "--bare", d.to_str().unwrap()]);
    f.git(&w, &["remote", "add", "origin", d.to_str().unwrap()]);
    std::fs::write(
        &f.mailmap,
        "Dev <developer@destination.example> <developer@source.example>\n",
    )
    .unwrap();
    f
}

#[test]
fn maps_author_and_committer_while_preserving_dates_full_message_and_tree() {
    let f = fixture();
    let upstream = f.commit("feat: x\n\nbody with ünicode", "1\n", D2, D3);
    f.sync(&Sync::default());
    let format = "%an <%ae>|%ad|%cn <%ce>|%cd|%B";
    assert_eq!(
        f.git(
            &f.work,
            &["log", "-1", "--date=raw", &format!("--format={format}"), "develop"]
        ),
        format!(
            "Dev <developer@destination.example>|{}|Dev <developer@destination.example>|{}|feat: x\n\nbody with ünicode",
            raw(D2),
            raw(D3)
        )
    );
    assert_eq!(
        f.git(&f.work, &["rev-parse", "develop^{tree}"]),
        f.git(&f.source, &["rev-parse", &format!("{upstream}^{{tree}}")])
    );
    assert_ne!(f.git(&f.work, &["rev-parse", "develop"]), upstream);
    assert_eq!(f.git(&f.work, &["rev-parse", "develop^"]), f.initial);
}

#[test]
fn matches_a_rewritten_upstream_tip_by_content_without_duplicating_its_commit() {
    let f = fixture();
    f.git_env(&f.source, &["commit", "-q", "--amend", "--no-edit"], &identity(D1, D2));
    assert_ne!(f.git(&f.source, &["rev-parse", "HEAD"]), f.initial);
    let upstream = f.commit("new", "1\n", D2, D2);
    f.sync(&Sync::default());
    assert_eq!(f.git(&f.work, &["rev-list", "--count", "develop"]), "2");
    assert_eq!(f.git(&f.work, &["rev-parse", "develop^"]), f.initial);
    assert_eq!(
        f.git(&f.work, &["rev-parse", "develop^{tree}"]),
        f.git(&f.source, &["rev-parse", &format!("{upstream}^{{tree}}")])
    );
}

#[test]
fn a_tip_whose_subject_was_rewritten_upstream_falls_back_to_a_unique_tree_match() {
    let f = fixture();
    f.git_env(
        &f.source,
        &["commit", "-q", "--amend", "-m", "initial renamed"],
        &identity(D1, D1),
    );
    f.commit("new", "1\n", D2, D2);
    f.sync(&Sync::default());
    assert_eq!(f.git(&f.work, &["log", "--format=%s", "develop"]), "new\ninitial");
}

#[test]
fn a_tree_only_fallback_that_is_ambiguous_stops_instead_of_guessing() {
    let f = fixture();
    f.git_env(
        &f.source,
        &["commit", "-q", "--amend", "-m", "initial renamed"],
        &identity(D1, D1),
    );
    f.git_env(
        &f.source,
        &["commit", "-q", "--allow-empty", "-m", "empty"],
        &identity(D2, D2),
    );
    let before = f.refs(&f.work);
    let r = f.run(&Sync::default());
    assert_eq!(r.code, 1);
    assert!(r.err.contains("tree-nya ambigu (2 commit)"), "{}", r.err);
    assert_eq!(f.refs(&f.work), before);
}

#[test]
fn an_empty_upstream_commit_after_the_last_synced_one_is_still_imported() {
    let f = fixture();
    f.git_env(
        &f.source,
        &["commit", "-q", "--allow-empty", "-m", "empty"],
        &identity(D2, D2),
    );
    f.sync(&Sync::default());
    f.git_env(
        &f.source,
        &["commit", "-q", "--allow-empty", "-m", "empty2"],
        &identity(D3, D3),
    );
    f.git(&f.source, &["tag", "v2"]);
    f.sync(&Sync::default());
    assert_eq!(
        f.git(&f.work, &["log", "--format=%s", "develop"]),
        "empty2\nempty\ninitial"
    );
    assert_eq!(
        f.git(&f.work, &["rev-parse", "v2"]),
        f.git(&f.work, &["rev-parse", "develop"])
    );
}

#[test]
fn restores_the_original_branch_or_detached_head_after_syncing_another_branch() {
    let f = fixture();
    f.git(&f.work, &["switch", "-q", "-c", "other"]);
    f.commit("one", "1\n", D2, D2);
    f.sync(&Sync::default());
    assert_eq!(f.git(&f.work, &["symbolic-ref", "HEAD"]), "refs/heads/other");
    f.git(&f.work, &["switch", "-q", "--detach", "other"]);
    f.commit("two", "2\n", D3, D3);
    f.sync(&Sync::default());
    assert_eq!(f.git(&f.work, &["rev-parse", "--abbrev-ref", "HEAD"]), "HEAD");
    assert_eq!(f.git(&f.work, &["rev-parse", "HEAD"]), f.initial);
    assert_eq!(f.git(&f.work, &["log", "--format=%s", "develop"]), "two\none\ninitial");
}

#[test]
fn dry_run_does_not_change_heads_tags_checkout_or_destination_even_with_push_and_deletion() {
    let f = fixture();
    f.git(&f.work, &["branch", "untouched"]);
    f.git(&f.work, &["tag", "manual"]);
    f.git(&f.work, &["push", "-q", "origin", "develop", "refs/tags/manual"]);
    f.commit("new", "1\n", D2, D2);
    f.annotated_tag("v1", D2, "release", "HEAD");
    let local_before = f.refs(&f.work);
    let remote_before = f.refs(&f.destination);
    let checkout_before = f.git(&f.work, &["symbolic-ref", "HEAD"]);
    f.sync(&Sync {
        dry_run: true,
        push: true,
        delete_tags: true,
    });
    assert_eq!(f.refs(&f.work), local_before);
    assert_eq!(f.refs(&f.destination), remote_before);
    assert_eq!(f.git(&f.work, &["symbolic-ref", "HEAD"]), checkout_before);
    assert_eq!(f.git(&f.work, &["show", "HEAD:f.txt"]), "0");
    assert_eq!(f.git(&f.work, &["status", "--porcelain"]), "");
}

#[test]
fn maps_annotated_and_lightweight_tags_and_preserves_mapped_tagger_metadata() {
    let f = fixture();
    let upstream = f.commit("new", "1\n", D2, D2);
    f.annotated_tag("v1", D3, "release one\n\nnotes", "HEAD");
    f.git(&f.source, &["tag", "light"]);
    f.sync(&Sync::default());
    let local = f.git(&f.work, &["rev-parse", "develop"]);
    assert_ne!(local, upstream);
    assert_eq!(f.git(&f.work, &["rev-parse", "v1^{commit}"]), local);
    assert_eq!(f.git(&f.work, &["rev-parse", "light"]), local);
    assert_eq!(f.git(&f.work, &["cat-file", "-t", "light"]), "commit");
    assert_eq!(f.git(&f.work, &["cat-file", "-t", "v1"]), "tag");
    assert_eq!(
        f.git(
            &f.work,
            &[
                "for-each-ref",
                "refs/tags/v1",
                "--format=%(taggername)|%(taggeremail)|%(taggerdate:raw)|%(contents)"
            ]
        ),
        format!("Dev|<developer@destination.example>|{}|release one\n\nnotes", raw(D3))
    );
    let before = f.refs(&f.work);
    f.sync(&Sync::default());
    assert_eq!(f.refs(&f.work), before);
}

#[test]
fn pushes_destination_branch_and_tags_and_force_updates_a_moved_source_tag() {
    let f = fixture();
    f.commit("one", "1\n", D1, D1);
    f.annotated_tag("v1", D1, "release", "HEAD");
    f.sync(&Sync {
        push: true,
        ..Sync::default()
    });
    let first_tag = f.git(&f.destination, &["rev-parse", "v1"]);
    assert_eq!(
        f.git(&f.destination, &["rev-parse", "develop"]),
        f.git(&f.work, &["rev-parse", "develop"])
    );
    assert_eq!(
        f.git(&f.destination, &["rev-parse", "v1"]),
        f.git(&f.work, &["rev-parse", "v1"])
    );
    let second = f.commit("two", "2\n", D2, D2);
    f.annotated_tag("v1", D2, "moved release", &second);
    f.sync(&Sync {
        push: true,
        ..Sync::default()
    });
    assert_ne!(f.git(&f.destination, &["rev-parse", "v1"]), first_tag);
    assert_eq!(
        f.git(&f.destination, &["rev-parse", "v1"]),
        f.git(&f.work, &["rev-parse", "v1"])
    );
    assert_eq!(
        f.git(&f.destination, &["rev-parse", "v1^{commit}"]),
        f.git(&f.work, &["rev-parse", "develop"])
    );
    assert_eq!(
        f.git(&f.destination, &["rev-parse", "develop"]),
        f.git(&f.work, &["rev-parse", "develop"])
    );
}

#[test]
fn preserves_absent_source_tags_by_default_and_deletes_local_and_remote_tags_only_on_opt_in() {
    let f = fixture();
    f.git(&f.work, &["tag", "manual"]);
    f.git(&f.work, &["push", "-q", "origin", "develop", "refs/tags/manual"]);
    f.sync(&Sync {
        push: true,
        ..Sync::default()
    });
    assert_eq!(f.git(&f.work, &["tag"]), "manual");
    assert_eq!(f.git(&f.destination, &["tag"]), "manual");
    f.sync(&Sync {
        push: true,
        delete_tags: true,
        ..Sync::default()
    });
    assert_eq!(f.git(&f.work, &["tag"]), "");
    assert_eq!(f.git(&f.destination, &["tag"]), "");
}

#[test]
fn a_rejected_destination_tag_is_not_overwritten_and_does_not_block_the_other_pushes() {
    let f = fixture();
    f.commit("one", "1\n", D1, D1);
    f.annotated_tag("v1", D1, "release", "HEAD");
    f.sync(&Sync {
        push: true,
        ..Sync::default()
    });
    let second = f.commit("two", "2\n", D2, D2);
    f.annotated_tag("v1", D2, "moved release", &second);
    f.annotated_tag("v2", D2, "release", "HEAD");
    f.git(&f.destination, &["tag", "v2", "develop"]);
    let conflicting = f.git(&f.destination, &["rev-parse", "v2"]);
    let r = f.run(&Sync {
        push: true,
        ..Sync::default()
    });
    assert_eq!(r.code, 1);
    let tail = r.err.split("Push gagal:\n").nth(1).unwrap_or_default();
    assert!(
        tail.lines().next().is_some_and(|l| l.contains("refs/tags/v2")) || tail.contains("refs/tags/v2"),
        "{}",
        r.err
    );
    assert_eq!(
        f.git(&f.destination, &["rev-parse", "develop"]),
        f.git(&f.work, &["rev-parse", "develop"])
    );
    assert_eq!(
        f.git(&f.destination, &["rev-parse", "v1"]),
        f.git(&f.work, &["rev-parse", "v1"])
    );
    assert_eq!(f.git(&f.destination, &["rev-parse", "v2"]), conflicting);
    assert_eq!(f.git(&f.destination, &["cat-file", "-t", "v2"]), "commit");
}

#[test]
fn skips_tags_whose_matching_local_commit_is_ambiguous_while_synchronizing_the_branch() {
    let f = fixture();
    f.commit("x", "1\n", D2, D2);
    f.commit("y", "2\n", D2, D2);
    f.commit("x", "1\n", D2, D2);
    f.annotated_tag("v1", D2, "release", "HEAD");
    f.sync(&Sync {
        push: true,
        ..Sync::default()
    });
    assert_eq!(f.git(&f.work, &["tag"]), "");
    assert_eq!(f.git(&f.destination, &["tag"]), "");
    assert_eq!(f.git(&f.work, &["rev-list", "--count", "develop"]), "4");
    assert_eq!(
        f.git(&f.destination, &["rev-parse", "develop"]),
        f.git(&f.work, &["rev-parse", "develop"])
    );
}

#[test]
fn rejects_a_dirty_working_tree_without_changing_local_or_destination_refs() {
    let f = fixture();
    f.commit("new", "1\n", D2, D2);
    let before = f.refs(&f.work);
    let remote_before = f.refs(&f.destination);
    std::fs::write(f.work.join("f.txt"), "dirty\n").unwrap();
    let r = f.run(&Sync {
        push: true,
        ..Sync::default()
    });
    assert_eq!(r.code, 1);
    assert!(r.err.contains("working tree ") && r.err.contains(" kotor"), "{}", r.err);
    assert_eq!(f.refs(&f.work), before);
    assert_eq!(f.refs(&f.destination), remote_before);
    assert!(f.git(&f.work, &["diff", "--", "f.txt"]).contains("+dirty"));
}

#[test]
fn inherited_identity_variables_do_not_override_mapped_commit_metadata() {
    let mut f = fixture();
    f.commit("new", "1\n", D2, D3);
    f.env.extend([
        ("GIT_AUTHOR_NAME".into(), "Wrong author".into()),
        ("GIT_AUTHOR_EMAIL".into(), "wrong@author.example".into()),
        ("GIT_AUTHOR_DATE".into(), D1.into()),
        ("GIT_COMMITTER_NAME".into(), "Wrong committer".into()),
        ("GIT_COMMITTER_EMAIL".into(), "wrong@committer.example".into()),
        ("GIT_COMMITTER_DATE".into(), D1.into()),
    ]);
    f.sync(&Sync::default());
    assert_eq!(
        f.git(
            &f.work,
            &["log", "-1", "--date=raw", "--format=%an|%ae|%ad|%cn|%ce|%cd"]
        ),
        format!(
            "Dev|developer@destination.example|{}|Dev|developer@destination.example|{}",
            raw(D2),
            raw(D3)
        )
    );
}

#[test]
fn dry_run_fetch_ignores_remote_refmaps_that_would_overwrite_local_branches_and_tags() {
    let f = fixture();
    f.git(
        &f.work,
        &["config", "--add", "remote.github.fetch", "+refs/heads/*:refs/heads/*"],
    );
    f.git(
        &f.work,
        &["config", "--add", "remote.github.fetch", "+refs/tags/*:refs/tags/*"],
    );
    f.git(&f.work, &["tag", "release"]);
    f.commit("new", "1\n", D2, D2);
    f.git(&f.source, &["tag", "release"]);
    let before = f.refs(&f.work);
    let result = f.run(&Sync {
        dry_run: true,
        ..Sync::default()
    });
    assert_eq!(result.code, 0, "{}", result.err);
    assert_eq!(f.refs(&f.work), before);
    assert_eq!(
        f.git(&f.work, &["rev-parse", "refs/syncgit-tags/github/release"]),
        f.git(&f.source, &["rev-parse", "release"])
    );
}

#[test]
fn unfinished_git_operations_are_rejected_before_fetching_or_changing_refs() {
    let f = fixture();
    f.commit("new", "1\n", D2, D2);
    let before = f.refs(&f.work);
    let source_before = f.git(&f.work, &["rev-parse", "refs/remotes/github/develop"]);
    for state in [
        "CHERRY_PICK_HEAD",
        "MERGE_HEAD",
        "REVERT_HEAD",
        "rebase-merge",
        "rebase-apply",
        "sequencer",
    ] {
        let path = PathBuf::from(f.git(&f.work, &["rev-parse", "--git-path", state]));
        let path = if path.is_absolute() { path } else { f.work.join(path) };
        if state.contains('-') || state == "sequencer" {
            std::fs::create_dir(&path).unwrap();
        } else {
            std::fs::write(&path, format!("{}\n", f.initial)).unwrap();
        }
        let result = f.run(&Sync::default());
        assert_eq!(result.code, 1, "{state}: {}", result.err);
        assert_eq!(
            result.err,
            format!("Operasi Git {state} masih berlangsung; selesaikan terlebih dahulu\n")
        );
        assert_eq!(f.refs(&f.work), before);
        assert_eq!(
            f.git(&f.work, &["rev-parse", "refs/remotes/github/develop"]),
            source_before
        );
        if path.is_dir() {
            std::fs::remove_dir(&path).unwrap();
        } else {
            std::fs::remove_file(&path).unwrap();
        }
    }
}
