use crate::errors::{CliError, Result};
use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyncGitOptions {
    pub repo: String,
    pub source: String,
    pub destination: String,
    pub mailmap: String,
    /// Empty: the source remote's default branch (see `source_default_branch`).
    pub branches: Vec<String>,
    pub dry_run: bool,
    pub push: bool,
    pub delete_tags: bool,
}

#[derive(Clone)]
struct Commit {
    hash: String,
    tree: String,
    author_name: String,
    author_email: String,
    author_date: String,
    committer_name: String,
    committer_email: String,
    committer_date: String,
    subject: String,
}

struct Tag {
    name: String,
    object: String,
    annotated: bool,
    commit: Option<String>,
}

struct GitResult {
    code: i32,
    stdout: String,
    stderr: String,
}

type Env = Vec<(&'static str, String)>;
type Key = (String, String, String);

const COMMIT_FORMAT: &str = "%H%x00%T%x00%aN%x00%aE%x00%aI%x00%cN%x00%cE%x00%cI%x00%s";

macro_rules! args {
    ($($e:expr),* $(,)?) => { [$(String::from($e)),*] };
}

fn fail<T>(message: impl Into<String>) -> Result<T> {
    Err(CliError::Failed(message.into()))
}

fn warning(message: &str) {
    eprintln!("{message}");
}

fn key(commit: &Commit) -> Key {
    (commit.tree.clone(), commit.author_date.clone(), commit.subject.clone())
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn short(hash: &str) -> &str {
    hash.get(..7).unwrap_or(hash)
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

pub(super) fn spawn_error(error: std::io::Error) -> CliError {
    let code = match error.kind() {
        std::io::ErrorKind::NotFound => "ENOENT",
        std::io::ErrorKind::PermissionDenied => "EACCES",
        std::io::ErrorKind::NotADirectory => "ENOTDIR",
        _ => return error.into(),
    };
    CliError::Failed(format!("spawn git {code}"))
}

fn signal_name(status: &ExitStatus) -> &'static str {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(signal) = status.signal() {
            for (number, name) in [
                (libc::SIGHUP, "SIGHUP"),
                (libc::SIGINT, "SIGINT"),
                (libc::SIGQUIT, "SIGQUIT"),
                (libc::SIGILL, "SIGILL"),
                (libc::SIGTRAP, "SIGTRAP"),
                (libc::SIGABRT, "SIGABRT"),
                (libc::SIGBUS, "SIGBUS"),
                (libc::SIGFPE, "SIGFPE"),
                (libc::SIGKILL, "SIGKILL"),
                (libc::SIGUSR1, "SIGUSR1"),
                (libc::SIGSEGV, "SIGSEGV"),
                (libc::SIGUSR2, "SIGUSR2"),
                (libc::SIGPIPE, "SIGPIPE"),
                (libc::SIGALRM, "SIGALRM"),
                (libc::SIGTERM, "SIGTERM"),
                (libc::SIGCHLD, "SIGCHLD"),
                (libc::SIGCONT, "SIGCONT"),
                (libc::SIGSTOP, "SIGSTOP"),
                (libc::SIGTSTP, "SIGTSTP"),
                (libc::SIGTTIN, "SIGTTIN"),
                (libc::SIGTTOU, "SIGTTOU"),
                (libc::SIGURG, "SIGURG"),
                (libc::SIGXCPU, "SIGXCPU"),
                (libc::SIGXFSZ, "SIGXFSZ"),
                (libc::SIGVTALRM, "SIGVTALRM"),
                (libc::SIGPROF, "SIGPROF"),
                (libc::SIGWINCH, "SIGWINCH"),
                (libc::SIGIO, "SIGIO"),
                (libc::SIGSYS, "SIGSYS"),
            ] {
                if signal == number {
                    return name;
                }
            }
            #[cfg(target_os = "linux")]
            match signal {
                libc::SIGSTKFLT => return "SIGSTKFLT",
                libc::SIGPWR => return "SIGPWR",
                _ => {}
            }
        }
    }
    #[cfg(not(unix))]
    let _ = status;
    "null"
}

/// Lexical equivalent of Node's `path.resolve(base, path)` (no filesystem access).
pub fn resolve(base: &Path, path: &str) -> PathBuf {
    let joined = base.join(path);
    let mut out = PathBuf::new();
    for component in joined.components() {
        match component {
            Component::ParentDir => {
                if out.file_name().is_some_and(|name| name != "..") {
                    out.pop();
                } else if !out.has_root() {
                    out.push("..");
                }
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

fn lossy(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

struct Git {
    repo: String,
    mailmap: Option<String>,
}

impl Git {
    fn new(repo: String, mailmap: Option<String>) -> Git {
        Git { repo, mailmap }
    }

    fn command<S: AsRef<OsStr>>(&self, args: &[S], env: &Env) -> Command {
        let mut command = Command::new("git");
        command.arg("-C").arg(&self.repo);
        if let Some(mailmap) = &self.mailmap {
            command.arg("-c").arg(format!("mailmap.file={mailmap}"));
        }
        command.args(args);
        for (name, value) in env {
            command.env(name, value);
        }
        command
    }

    fn result(&self, args: &[String], input: Option<&str>, env: &Env) -> Result<GitResult> {
        let mut child = self
            .command(args, env)
            .stdin(if input.is_some() { Stdio::piped() } else { Stdio::null() })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(spawn_error)?;
        let output = std::thread::scope(|scope| {
            if let Some(data) = input {
                let stdin = child.stdin.take();
                // A command may exit before consuming stdin; its exit status is the useful error.
                scope.spawn(move || {
                    if let Some(mut pipe) = stdin {
                        let _ = pipe.write_all(data.as_bytes());
                    }
                });
            }
            child.wait_with_output()
        })?;
        let mut stderr = text(&output.stderr);
        if stderr.is_empty() && output.status.code().is_none() {
            stderr = format!("terminated by {}", signal_name(&output.status));
        }
        Ok(GitResult {
            code: output.status.code().unwrap_or(128),
            stdout: text(&output.stdout),
            stderr,
        })
    }

    fn run_full(&self, args: &[String], input: Option<&str>, env: &Env) -> Result<String> {
        let result = self.result(args, input, env)?;
        if result.code != 0 {
            let detail = if result.stderr.trim().is_empty() {
                result.stdout.trim()
            } else {
                result.stderr.trim()
            };
            return fail(format!("git {} gagal ({}): {}", args.join(" "), result.code, detail));
        }
        Ok(result.stdout)
    }

    fn run(&self, args: &[String]) -> Result<String> {
        self.run_full(args, None, &Vec::new())
    }

    fn exists(&self, reference: &str) -> Result<bool> {
        let result = self.result(&args!["show-ref", "--verify", "--quiet", reference], None, &Vec::new())?;
        match result.code {
            0 => Ok(true),
            1 => Ok(false),
            _ => fail(format!("Tidak dapat membaca {reference}: {}", result.stderr.trim())),
        }
    }

    fn rev(&self, reference: &str) -> Result<String> {
        Ok(self.run(&args!["rev-parse", "--verify", reference])?.trim().to_string())
    }

    /// Streams `git log` commits in output order without buffering the whole history.
    /// `visit` returning `Ok(true)` stops the walk early (the git process is killed).
    fn walk(&self, revisions: &[String], extra: &[&str], mut visit: impl FnMut(Commit) -> Result<bool>) -> Result<()> {
        if revisions.is_empty() {
            return Ok(());
        }
        let mut args = args!["log", "-z", "--use-mailmap", format!("--format={COMMIT_FORMAT}")].to_vec();
        args.extend(extra.iter().map(|e| e.to_string()));
        args.extend(revisions.iter().cloned());
        args.push("--".to_string());
        let mut child = self
            .command(&args, &Vec::new())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(spawn_error)?;
        let mut stderr_pipe = child.stderr.take();
        let stderr_reader = std::thread::spawn(move || {
            let mut buffer = Vec::new();
            if let Some(pipe) = stderr_pipe.as_mut() {
                let _ = pipe.read_to_end(&mut buffer);
            }
            buffer
        });
        let Some(stdout) = child.stdout.take() else {
            return fail("stdout git tidak tersedia");
        };
        let mut reader = BufReader::new(stdout);
        let mut fields: Vec<String> = Vec::new();
        let mut partial = false;
        let mut buffer = Vec::new();
        loop {
            buffer.clear();
            let read = match reader.read_until(0, &mut buffer) {
                Ok(n) => n,
                Err(e) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(e.into());
                }
            };
            if read == 0 {
                break;
            }
            if buffer.last() != Some(&0) {
                partial = true;
                break;
            }
            buffer.pop();
            fields.push(text(&buffer));
            if fields.len() == 9 {
                let mut it = std::mem::take(&mut fields).into_iter();
                let mut next = || it.next().unwrap_or_default();
                let commit = Commit {
                    hash: next(),
                    tree: next(),
                    author_name: next(),
                    author_email: next(),
                    author_date: next(),
                    committer_name: next(),
                    committer_email: next(),
                    committer_date: next(),
                    subject: next(),
                };
                match visit(commit) {
                    Ok(false) => {}
                    Ok(true) => {
                        let _ = child.kill();
                        let _ = child.wait();
                        return Ok(());
                    }
                    Err(e) => {
                        let _ = child.kill();
                        let _ = child.wait();
                        return Err(e);
                    }
                }
            }
        }
        drop(reader);
        let status = child.wait()?;
        let stderr = text(&stderr_reader.join().unwrap_or_default());
        if !status.success() {
            let message = if stderr.trim().is_empty() {
                format!("terminated by {}", signal_name(&status))
            } else {
                stderr.trim().to_string()
            };
            return fail(format!(
                "git {} gagal ({}): {}",
                args.join(" "),
                status.code().unwrap_or(128),
                message
            ));
        }
        if partial || !fields.is_empty() {
            return fail("Metadata commit Git tidak valid");
        }
        Ok(())
    }

    fn log(&self, revisions: &[String], extra: &[&str]) -> Result<Vec<Commit>> {
        let mut commits = Vec::new();
        self.walk(revisions, extra, |commit| {
            commits.push(commit);
            Ok(false)
        })?;
        Ok(commits)
    }

    fn tags(&self, namespace: &str) -> Result<Vec<Tag>> {
        let output = self.run(&args![
            "for-each-ref",
            "--format=%(refname)%00%(objecttype)%00%(objectname)%00",
            format!("{namespace}/")
        ])?;
        let mut tags = Vec::new();
        for record in output.split("\0\n") {
            if record.is_empty() {
                continue;
            }
            let mut parts = record.split('\0');
            let (reference, kind, object) = (parts.next(), parts.next(), parts.next());
            let (Some(reference), Some(kind), Some(object)) = (reference, kind, object) else {
                return fail("Metadata tag Git tidak valid");
            };
            if reference.is_empty()
                || kind.is_empty()
                || object.is_empty()
                || !reference.starts_with(&format!("{namespace}/"))
            {
                return fail("Metadata tag Git tidak valid");
            }
            let peeled = self.rev(&format!("{reference}^{{}}"))?;
            let kind_of_peeled = self.run(&args!["cat-file", "-t", &peeled])?.trim().to_string();
            tags.push(Tag {
                name: reference[namespace.len() + 1..].to_string(),
                object: object.to_string(),
                annotated: kind == "tag",
                commit: (kind_of_peeled == "commit").then_some(peeled),
            });
        }
        Ok(tags)
    }
}

fn identity_environment(commit: &Commit) -> Env {
    vec![
        ("GIT_AUTHOR_NAME", commit.author_name.clone()),
        ("GIT_AUTHOR_EMAIL", commit.author_email.clone()),
        ("GIT_AUTHOR_DATE", commit.author_date.clone()),
        ("GIT_COMMITTER_NAME", commit.committer_name.clone()),
        ("GIT_COMMITTER_EMAIL", commit.committer_email.clone()),
        ("GIT_COMMITTER_DATE", commit.committer_date.clone()),
    ]
}

/// Splits `Name <email>` (email without angle brackets); the name has no newline.
fn split_identity(value: &str) -> Option<(&str, &str)> {
    let rest = value.strip_suffix('>')?;
    let lt = rest.rfind('<')?;
    let email = &rest[lt + 1..];
    if email.contains(['<', '>']) {
        return None;
    }
    let name = rest[..lt].strip_suffix(' ')?;
    if name.contains('\n') {
        return None;
    }
    Some((name, email))
}

/// Matches `^tagger (.*) <([^<>]*)> (\d+ [+-]\d{4})$` per header line.
fn parse_tagger(header: &str) -> Option<(String, String, String)> {
    for line in header.split('\n') {
        let Some(rest) = line.strip_prefix("tagger ") else {
            continue;
        };
        let Some((rest, zone)) = rest.rsplit_once(' ') else {
            continue;
        };
        let zone_ok = zone.len() == 5
            && matches!(zone.as_bytes()[0], b'+' | b'-')
            && zone.as_bytes()[1..].iter().all(u8::is_ascii_digit);
        if !zone_ok {
            continue;
        }
        let Some((rest, epoch)) = rest.rsplit_once(' ') else {
            continue;
        };
        if epoch.is_empty() || !epoch.bytes().all(|b| b.is_ascii_digit()) {
            continue;
        }
        if let Some((name, email)) = split_identity(rest) {
            return Some((name.to_string(), email.to_string(), format!("{epoch} {zone}")));
        }
    }
    None
}

/// Byte index of the first line starting with a PGP/SSH signature marker.
fn signature_index(contents: &str) -> Option<usize> {
    let mut offset = 0;
    for line in contents.split_inclusive('\n') {
        if line.starts_with("-----BEGIN PGP SIGNATURE-----") || line.starts_with("-----BEGIN SSH SIGNATURE-----") {
            return Some(offset);
        }
        offset += line.len();
    }
    None
}

fn tag_identity(git: &Git, tag: &Tag) -> Result<Option<(String, Env)>> {
    let object = git.run(&args!["cat-file", "tag", &tag.object])?;
    let boundary = object.find("\n\n");
    let header = boundary.map_or(object.as_str(), |b| &object[..b]);
    let Some((tagger_name, tagger_email, date)) = parse_tagger(header) else {
        warning(&format!("   {}: tagger kosong/tidak valid, lewati", tag.name));
        return Ok(None);
    };
    let mapped = git
        .run_full(
            &args!["check-mailmap", "--stdin"],
            Some(&format!("{tagger_name} <{tagger_email}>\n")),
            &Vec::new(),
        )?
        .trim_end()
        .to_string();
    let Some((name, email)) = split_identity(&mapped) else {
        return fail(format!("Identitas mailmap tag {} tidak valid", tag.name));
    };
    let contents = boundary.map_or("", |b| &object[b + 2..]);
    // Transformed objects cannot retain a signature covering their original target/identity.
    let message = match signature_index(contents) {
        Some(i) => &contents[..i],
        None => contents,
    };
    Ok(Some((
        message.to_string(),
        vec![
            ("GIT_COMMITTER_NAME", name.to_string()),
            ("GIT_COMMITTER_EMAIL", email.to_string()),
            ("GIT_COMMITTER_DATE", date),
        ],
    )))
}

fn destination_tags(git: &Git, remote: &str) -> Result<HashMap<String, String>> {
    let mut refs = HashMap::new();
    let output = git.run(&args!["ls-remote", "--refs", "--tags", "--", remote])?;
    for line in output.split('\n') {
        if line.is_empty() {
            continue;
        }
        let Some((hash, name)) = line.split_once('\t') else {
            return fail("Metadata remote tag Git tidak valid");
        };
        refs.insert(name.to_string(), hash.to_string());
    }
    Ok(refs)
}

/// Branch the source remote's HEAD points to (e.g. main or master); asks the remote, so it needs network access.
pub fn source_default_branch(repo: &str, source: &str) -> Result<String> {
    if source.starts_with('-') {
        return fail(format!("Remote sumber tidak valid: {source}"));
    }
    let cwd = std::env::current_dir()?;
    let git = Git::new(lossy(&resolve(&cwd, repo)), None);
    let output = git.run(&args!["ls-remote", "--symref", "--", source, "HEAD"])?;
    // /^ref: refs\/heads\/(\S+)\tHEAD$/m
    let branch = output.split('\n').find_map(|line| {
        let name = line.strip_prefix("ref: refs/heads/")?.strip_suffix("\tHEAD")?;
        (!name.is_empty() && !name.chars().any(char::is_whitespace)).then(|| name.to_string())
    });
    match branch {
        Some(b) => Ok(b),
        None => fail(format!(
            "Branch default {source} tidak terdeteksi; sebutkan branch secara eksplisit."
        )),
    }
}

#[derive(Default)]
struct Progress {
    pending: HashSet<String>,
    synced_branches: Vec<String>,
    plain_tags: Vec<String>,
    moved_tags: Vec<String>,
    deleted_tags: Vec<String>,
    preserve_recovery: bool,
    switched: bool,
}

struct Original {
    head: String,
    branch: Option<String>,
}

fn restore(git: &Git, original: &Original, progress: &Progress) -> Result<()> {
    if !progress.switched || progress.preserve_recovery {
        return Ok(());
    }
    match &original.branch {
        Some(branch) => git.run(&args!["switch", "--no-guess", "--", branch])?,
        None => git.run(&args!["switch", "--detach", &original.head])?,
    };
    Ok(())
}

fn apply_commit(git: &Git, commit: &Commit) -> Result<()> {
    git.run(&args![
        "-c",
        "commit.gpgSign=false",
        "cherry-pick",
        "--no-gpg-sign",
        "--no-commit",
        &commit.hash
    ])?;
    git.run_full(
        &args![
            "-c",
            "commit.gpgSign=false",
            "commit",
            "-q",
            "--no-gpg-sign",
            "--allow-empty",
            "--cleanup=verbatim",
            "-C",
            &commit.hash,
            "--reset-author"
        ],
        None,
        &identity_environment(commit),
    )?;
    Ok(())
}

fn recover_failure(git: &Git, commit: &Commit, branch: &str, progress: &mut Progress, error: CliError) -> CliError {
    let inspected = (|| -> Result<(String, String)> {
        let unmerged = git.run(&args!["diff", "--name-only", "--diff-filter=U", "-z"])?;
        let dirty = git.run(&args!["status", "--porcelain=v1", "--untracked-files=no"])?;
        Ok((unmerged, dirty))
    })();
    let (unmerged, dirty) = match inspected {
        Ok(pair) => pair,
        Err(inspect_error) => {
            // Unknown state: never switch away from possible changes, and keep the real error.
            progress.preserve_recovery = true;
            warning(&format!(
                "   Status working tree {branch} tidak terbaca ({inspect_error}); HEAD tidak dipulihkan. Periksa git status secara manual."
            ));
            return error;
        }
    };
    progress.preserve_recovery = !unmerged.is_empty() || !dirty.trim().is_empty();
    if !unmerged.is_empty() {
        let environment = identity_environment(commit)
            .iter()
            .map(|(name, value)| format!("{name}={}", shell_quote(value)))
            .collect::<Vec<_>>()
            .join(" ");
        return CliError::Failed(format!(
            "KONFLIK di {hash} pada {branch}. Working tree dibiarkan untuk pemulihan manual. Selesaikan konflik, jalankan git add, lalu:\n{environment} git -c commit.gpgSign=false commit --no-gpg-sign --allow-empty --cleanup=verbatim -C {hash} --reset-author\nJalankan ulang syncgit setelah commit selesai.",
            hash = commit.hash
        ));
    }
    if progress.preserve_recovery {
        warning(&format!(
            "   Perubahan belum di-commit pada {branch}; HEAD tidak dipulihkan. Periksa git status dan selesaikan operasi manual sebelum menjalankan ulang."
        ));
    }
    error
}

#[allow(clippy::too_many_lines)]
fn sync_body(
    git: &Git,
    options: &SyncGitOptions,
    branches: &[String],
    namespace: &str,
    progress: &mut Progress,
) -> Result<()> {
    println!(">> fetch remote {}", options.source);
    // Explicit destinations prevent remote fetch configuration from writing local heads/tags.
    git.run(&args![
        "fetch",
        "-q",
        "--no-tags",
        "--refmap=",
        "--",
        &options.source,
        format!("+refs/heads/*:refs/remotes/{}/*", options.source)
    ])?;
    git.run(&args![
        "fetch",
        "-q",
        "--no-tags",
        "--prune",
        "--force",
        "--refmap=",
        "--",
        &options.source,
        format!("+refs/tags/*:{namespace}/*")
    ])?;

    for branch in branches {
        println!("== {branch}");
        let local_ref = format!("refs/heads/{branch}");
        let source_ref = format!("refs/remotes/{}/{branch}", options.source);
        if !git.exists(&local_ref)? || !git.exists(&source_ref)? {
            warning(&format!(
                "   branch lokal atau {}/{branch} tidak ada, lewati",
                options.source
            ));
            continue;
        }
        let Some(tip) = git.log(std::slice::from_ref(&local_ref), &["-1"])?.into_iter().next() else {
            return fail(format!("Metadata commit tip {branch} tidak ditemukan"));
        };
        let tip_key = key(&tip);
        // Match by the tag identity (tree, author date, subject), not the tree alone: an empty commit shares
        // its parent's tree, so a tree-only match could select a newer upstream commit and skip it.
        let mut by_key: Option<Commit> = None;
        let mut by_tree: Option<Commit> = None;
        let mut trees = 0usize;
        git.walk(std::slice::from_ref(&source_ref), &[], |commit| {
            if key(&commit) == tip_key {
                by_key = Some(commit);
                return Ok(true);
            }
            if commit.tree != tip.tree {
                return Ok(false);
            }
            if by_tree.is_none() {
                by_tree = Some(commit);
            }
            trees += 1;
            Ok(false)
        })?;
        if by_key.is_none() && trees == 0 {
            return fail(format!(
                "tree tip {branch} tidak ditemukan di {}/{branch}; periksa manual",
                options.source
            ));
        }
        // Tree-only fallback (e.g. upstream rewrote the tip's subject) is safe only when the tree is unique.
        if by_key.is_none() && trees > 1 {
            return fail(format!(
                "tip {branch} tidak cocok author date/subject di {}/{branch} dan tree-nya ambigu ({trees} commit); periksa manual",
                options.source
            ));
        }
        if by_key.is_none() {
            warning(&format!(
                "   author date/subject tip {branch} tidak ditemukan di {}/{branch}; titik sinkron dipilih lewat tree saja",
                options.source
            ));
        }
        let Some(base) = by_key.or(by_tree) else {
            return fail("Titik sinkron tidak ditemukan");
        };
        let commits = git.log(&[format!("{}..{source_ref}", base.hash)], &["--reverse"])?;
        if commits.is_empty() {
            println!("   sudah terbaru");
        } else {
            println!("   titik sinkron: {}; commit baru:", short(&base.hash));
            for commit in &commits {
                println!("     {} {}", short(&commit.hash), commit.subject);
                progress.pending.insert(commit.hash.clone());
            }
            if !options.dry_run {
                git.run(&args!["switch", "--no-guess", "--", branch])?;
                progress.switched = true;
                for commit in &commits {
                    if let Err(error) = apply_commit(git, commit) {
                        return Err(recover_failure(git, commit, branch, progress, error));
                    }
                }
                if git.rev(&format!("{local_ref}^{{tree}}"))? != git.rev(&format!("{source_ref}^{{tree}}"))? {
                    return fail(format!(
                        "PERINGATAN: tree {branch} != {}/{branch}; periksa manual",
                        options.source
                    ));
                }
                println!("   OK -> {}", short(&git.rev(&local_ref)?));
            }
        }
        progress.synced_branches.push(branch.clone());
    }

    println!("== tag");
    let upstream_tags = git.tags(namespace)?;
    let local_tag_list = git.tags("refs/tags")?;
    let local_tags: HashMap<&str, &Tag> = local_tag_list.iter().map(|t| (t.name.as_str(), t)).collect();
    let mut targets: Vec<String> = Vec::new();
    for tag in &upstream_tags {
        if let Some(commit) = &tag.commit
            && !targets.contains(commit)
        {
            targets.push(commit.clone());
        }
    }
    let upstream_commits: HashMap<String, Commit> = git
        .log(&targets, &["--no-walk=unsorted"])?
        .into_iter()
        .map(|c| (c.hash.clone(), c))
        .collect();
    // Index only local commits that can match a source tag; the history is streamed, not held in memory.
    let wanted: HashSet<Key> = upstream_commits.values().map(key).collect();
    let wanted_trees: HashSet<&str> = upstream_commits.values().map(|c| c.tree.as_str()).collect();
    let mut heads: Vec<String> = Vec::new();
    for branch in branches {
        let reference = format!("refs/heads/{branch}");
        if git.exists(&reference)? {
            heads.push(reference);
        }
    }
    let mut index: HashMap<Key, Vec<String>> = HashMap::new();
    let mut trees: HashSet<String> = HashSet::new();
    if !wanted.is_empty() {
        git.walk(&heads, &[], |commit| {
            if wanted_trees.contains(commit.tree.as_str()) {
                trees.insert(commit.tree.clone());
            }
            let identity = key(&commit);
            if !wanted.contains(&identity) {
                return Ok(false);
            }
            index.entry(identity).or_default().push(commit.hash);
            Ok(false)
        })?;
    }
    for tag in &upstream_tags {
        let Some(tag_commit) = &tag.commit else {
            warning(&format!("   {}: target tag bukan commit, lewati", tag.name));
            continue;
        };
        let Some(commit) = upstream_commits.get(tag_commit) else {
            return fail(format!("Metadata commit tag {} tidak ditemukan", tag.name));
        };
        let candidates = index.get(&key(commit)).map(Vec::as_slice).unwrap_or(&[]);
        if candidates.is_empty() {
            if options.dry_run && progress.pending.contains(&commit.hash) {
                println!(
                    "   {}: akan dibuat setelah commit {} dibawa",
                    tag.name,
                    short(&commit.hash)
                );
            } else if trees.contains(&commit.tree) {
                warning(&format!(
                    "   {}: tree cocok tapi author date/subject beda; periksa manual",
                    tag.name
                ));
            } else {
                warning(&format!("   {}: commit padanan belum ada, lewati", tag.name));
            }
            continue;
        }
        if candidates.len() > 1 {
            warning(&format!(
                "   {}: ambigu ({} commit lokal cocok), lewati",
                tag.name,
                candidates.len()
            ));
            continue;
        }
        let target = &candidates[0];
        let current = local_tags.get(tag.name.as_str()).copied();
        if let Some(current) = current
            && current.commit.as_ref() == Some(target)
            && current.annotated == tag.annotated
        {
            progress.plain_tags.push(tag.name.clone());
            continue;
        }
        let annotation = if tag.annotated { tag_identity(git, tag)? } else { None };
        if tag.annotated && annotation.is_none() {
            continue;
        }
        println!(
            "   {} -> {}{}",
            tag.name,
            short(target),
            current
                .map(|c| format!(" (pindah dari {})", short(&c.object)))
                .unwrap_or_default()
        );
        if current.is_some() {
            &mut progress.moved_tags
        } else {
            &mut progress.plain_tags
        }
        .push(tag.name.clone());
        if !options.dry_run {
            match annotation {
                Some((message, environment)) => {
                    git.run_full(
                        &args![
                            "-c",
                            "tag.gpgSign=false",
                            "tag",
                            "--no-sign",
                            "-f",
                            "-a",
                            "--cleanup=verbatim",
                            "-F",
                            "-",
                            "--",
                            &tag.name,
                            target
                        ],
                        Some(&message),
                        &environment,
                    )?;
                }
                None => {
                    git.run(&args![
                        "-c",
                        "tag.gpgSign=false",
                        "tag",
                        "--no-sign",
                        "-f",
                        "--",
                        &tag.name,
                        target
                    ])?;
                }
            }
        }
    }
    let upstream_names: HashSet<&str> = upstream_tags.iter().map(|t| t.name.as_str()).collect();
    let mut local_names: Vec<&str> = local_tags.keys().copied().collect();
    local_names.sort_unstable();
    for name in local_names {
        if upstream_names.contains(name) {
            continue;
        }
        if !options.delete_tags {
            println!("   {name}: hanya ada lokal (pakai --delete-tags untuk menghapus)");
        } else {
            warning(&format!(
                "   {name}: tidak ada di {}; {} (termasuk tag buatan lokal)",
                options.source,
                if options.dry_run { "akan dihapus" } else { "dihapus" }
            ));
            if !options.dry_run {
                git.run(&args!["tag", "-d", "--", name])?;
                progress.deleted_tags.push(name.to_string());
            }
        }
    }
    Ok(())
}

fn push(git: &Git, failures: &mut Vec<String>, args: impl AsRef<[String]>) -> Result<bool> {
    let args = args.as_ref();
    let mut full = Vec::with_capacity(args.len() + 1);
    full.push("push".to_string());
    full.extend_from_slice(args);
    let result = git.result(&full, None, &Vec::new())?;
    // git push reports ref updates and rejections on stderr.
    let output = format!("{}{}", result.stdout, result.stderr);
    let output = output.trim_end();
    if result.code == 0 {
        if !output.is_empty() {
            println!("{output}");
        }
        return Ok(true);
    }
    if !output.is_empty() {
        eprintln!("{output}");
    }
    failures.push(format!("git push {} ({})", args.join(" "), result.code));
    Ok(false)
}

pub fn sync_git(options: &SyncGitOptions) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let repo_path = resolve(&cwd, &options.repo);
    let repo = lossy(&repo_path);
    let mailmap = if Path::new(&options.mailmap).is_absolute() {
        options.mailmap.clone()
    } else {
        lossy(&resolve(&repo_path, &options.mailmap))
    };
    if !std::fs::metadata(&mailmap).map(|m| m.is_file()).unwrap_or(false) {
        return fail(format!("mailmap tidak ada: {mailmap}"));
    }
    let git = Git::new(repo.clone(), Some(mailmap));
    let remotes_out = git.run(&args!["remote"])?;
    let remotes: Vec<&str> = remotes_out.trim().split('\n').collect();
    if !remotes.contains(&options.source.as_str()) || options.source.starts_with('-') {
        return fail(format!("Remote sumber tidak ada/tidak valid: {}", options.source));
    }
    if options.push && (!remotes.contains(&options.destination.as_str()) || options.destination.starts_with('-')) {
        return fail(format!("Remote tujuan tidak ada/tidak valid: {}", options.destination));
    }
    let namespace = format!("refs/syncgit-tags/{}", options.source);
    git.run(&args!["check-ref-format", format!("{namespace}/check")])?;
    let branches: Vec<String> = if options.branches.is_empty() {
        vec![source_default_branch(&repo, &options.source)?]
    } else {
        let mut unique: Vec<String> = Vec::new();
        for branch in &options.branches {
            if !unique.contains(branch) {
                unique.push(branch.clone());
            }
        }
        unique
    };
    if options.branches.is_empty() {
        println!(">> branch default {}: {}", options.source, branches[0]);
    }
    for branch in &branches {
        if branch.starts_with('-') {
            return fail(format!("Nama branch tidak valid: {branch}"));
        }
        git.run(&args!["check-ref-format", format!("refs/heads/{branch}")])?;
    }
    if !git
        .run(&args!["status", "--porcelain=v1", "--untracked-files=no"])?
        .trim()
        .is_empty()
    {
        return fail(format!("working tree {repo} kotor; simpan perubahan terlebih dahulu"));
    }
    for state in [
        "CHERRY_PICK_HEAD",
        "MERGE_HEAD",
        "REVERT_HEAD",
        "rebase-merge",
        "rebase-apply",
        "sequencer",
    ] {
        let path = git.run(&args!["rev-parse", "--git-path", state])?;
        if std::fs::metadata(resolve(&repo_path, path.trim())).is_ok() {
            return fail(format!(
                "Operasi Git {state} masih berlangsung; selesaikan terlebih dahulu"
            ));
        }
    }
    let original_head = git.rev("HEAD")?;
    let symbolic = git.result(&args!["symbolic-ref", "--quiet", "HEAD"], None, &Vec::new())?;
    if symbolic.code != 0 && symbolic.code != 1 {
        return fail(format!("Tidak dapat membaca HEAD: {}", symbolic.stderr.trim()));
    }
    let original = Original {
        head: original_head,
        branch: (symbolic.code == 0).then(|| {
            symbolic
                .stdout
                .trim()
                .strip_prefix("refs/heads/")
                .unwrap_or(symbolic.stdout.trim())
                .to_string()
        }),
    };
    let observed = if options.push && !options.dry_run {
        destination_tags(&git, &options.destination)?
    } else {
        HashMap::new()
    };
    let mut progress = Progress::default();

    if let Err(error) = sync_body(&git, options, &branches, &namespace, &mut progress) {
        // The operation's error is what the user must act on; a failed restore is only reported.
        if let Err(restore_error) = restore(&git, &original, &progress) {
            warning(&format!("   HEAD awal tidak dapat dipulihkan: {restore_error}"));
        }
        return Err(error);
    }
    restore(&git, &original, &progress)?;

    if options.push && !options.dry_run {
        println!(">> push ke {}", options.destination);
        // Pushes are not atomic. Tag pushes are independent of each other, so one rejected tag does not block the
        // rest; every failure is reported together at the end.
        let mut failures: Vec<String> = Vec::new();
        let branches_pushed = progress.synced_branches.is_empty() || {
            let mut a = args!["--", &options.destination].to_vec();
            a.extend(
                progress
                    .synced_branches
                    .iter()
                    .map(|b| format!("refs/heads/{b}:refs/heads/{b}")),
            );
            push(&git, &mut failures, a)?
        };
        // Tags point at the synced branch commits; without the branches on the destination they are held back.
        if !branches_pushed {
            if !progress.plain_tags.is_empty() || !progress.moved_tags.is_empty() || !progress.deleted_tags.is_empty() {
                failures.push("push tag dilewati karena push branch gagal".to_string());
            }
        } else {
            if !progress.plain_tags.is_empty() {
                let mut a = args!["--", &options.destination].to_vec();
                a.extend(
                    progress
                        .plain_tags
                        .iter()
                        .map(|n| format!("refs/tags/{n}:refs/tags/{n}")),
                );
                push(&git, &mut failures, a)?;
            }
            for name in &progress.moved_tags {
                let reference = format!("refs/tags/{name}");
                let previous = observed.get(&reference).map(String::as_str).unwrap_or("");
                push(
                    &git,
                    &mut failures,
                    args![
                        format!("--force-with-lease={reference}:{previous}"),
                        "--",
                        &options.destination,
                        format!("{reference}:{reference}")
                    ],
                )?;
            }
            for name in &progress.deleted_tags {
                let reference = format!("refs/tags/{name}");
                if let Some(previous) = observed.get(&reference) {
                    push(
                        &git,
                        &mut failures,
                        args![
                            format!("--force-with-lease={reference}:{previous}"),
                            "--",
                            &options.destination,
                            format!(":{reference}")
                        ],
                    )?;
                }
            }
        }
        if !failures.is_empty() {
            let list: Vec<String> = failures.iter().map(|f| format!("  {f}")).collect();
            return fail(format!("Push gagal:\n{}", list.join("\n")));
        }
    }
    println!(">> selesai");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lexical_repo_resolution_never_walks_above_the_filesystem_root() {
        let base = if cfg!(windows) {
            Path::new("C:\\work")
        } else {
            Path::new("/work")
        };
        let root = base.ancestors().last().expect("absolute fixture path");
        assert_eq!(resolve(base, "../../.."), root);
        assert_eq!(resolve(base, "../../../repo/./child/.."), root.join("repo"));
    }

    #[test]
    fn annotated_tag_identity_preserves_name_email_and_raw_timezone() {
        assert_eq!(
            parse_tagger("object hash\ntagger Dev 'Ü' <dev@example.com> 1767261600 -0230\n"),
            Some(("Dev 'Ü'".into(), "dev@example.com".into(), "1767261600 -0230".into()))
        );
        assert_eq!(parse_tagger("tagger Dev <dev@example.com> -1 +0000"), None);
        assert_eq!(parse_tagger("tagger Dev <dev@example.com> 1 +000"), None);
        assert_eq!(parse_tagger("object hash\ntag release"), None);
        assert_eq!(split_identity("Dev <dev>extra>"), None);
    }

    #[test]
    fn transformed_tag_signatures_are_removed_only_at_the_start_of_a_line() {
        for signature in ["PGP", "SSH"] {
            let contents = format!("release ü\n\n-----BEGIN {signature} SIGNATURE-----\nsignature\n");
            assert_eq!(signature_index(&contents), Some("release ü\n\n".len()));
        }
        assert_eq!(signature_index("inline -----BEGIN PGP SIGNATURE-----"), None);
        assert_eq!(signature_index("-----BEGIN OTHER SIGNATURE-----"), None);
        assert_eq!(shell_quote("Dev 'name'"), "'Dev '\\''name'\\'''");
    }

    #[test]
    fn git_spawn_errors_keep_the_legacy_error_text() {
        assert_eq!(
            spawn_error(std::io::Error::from(std::io::ErrorKind::NotFound)).to_string(),
            "spawn git ENOENT"
        );
        assert_eq!(
            spawn_error(std::io::Error::from(std::io::ErrorKind::PermissionDenied)).to_string(),
            "spawn git EACCES"
        );
    }

    #[cfg(unix)]
    #[test]
    fn git_signal_errors_report_the_signal_name() {
        use std::os::unix::process::ExitStatusExt;
        assert_eq!(signal_name(&ExitStatus::from_raw(libc::SIGTERM)), "SIGTERM");
        assert_eq!(signal_name(&ExitStatus::from_raw(libc::SIGKILL)), "SIGKILL");
    }
}
