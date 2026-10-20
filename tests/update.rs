mod common;

use common::{Out, dt_env};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use tempfile::TempDir;

const CURRENT: &str = env!("CARGO_PKG_VERSION");

fn have_curl() -> bool {
    Command::new("curl")
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
}

/// Release asset suffix for this platform, built like the tool does; `None` where no release exists.
fn target() -> Option<(String, &'static str)> {
    let arch = if cfg!(target_arch = "x86_64") {
        "x86_64"
    } else if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else {
        return None;
    };
    let (suffix, ext) = if cfg!(target_os = "linux") {
        ("unknown-linux-musl", "tar.gz")
    } else {
        return None;
    };
    Some((format!("{arch}-{suffix}"), ext))
}

macro_rules! require_curl {
    () => {
        if !have_curl() {
            eprintln!("skip: curl tidak ada di PATH");
            return;
        }
    };
}

macro_rules! require_target {
    () => {
        match target() {
            Some(t) => t,
            None => {
                eprintln!("skip: platform tanpa aset rilis");
                return;
            }
        }
    };
}

fn handle(stream: TcpStream, base: &str, routes: &HashMap<String, Vec<u8>>, latest: Option<&str>) {
    let mut reader = BufReader::new(&stream);
    let mut request = String::new();
    if reader.read_line(&mut request).is_err() {
        return;
    }
    // Drain the headers; the response never depends on them.
    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) | Err(_) => break,
            Ok(_) if line.trim().is_empty() => break,
            Ok(_) => {}
        }
    }
    let mut parts = request.split_whitespace();
    let (method, path) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
    let (status, extra, body): (&str, String, &[u8]) = match (path, latest) {
        ("/latest", Some(tag)) => ("302 Found", format!("Location: {base}/tag/{tag}\r\n"), &[]),
        (p, Some(_)) if p.starts_with("/tag/") => ("200 OK", String::new(), &[]),
        _ => match routes.get(path) {
            Some(body) => ("200 OK", String::new(), body.as_slice()),
            None => ("404 Not Found", String::new(), &[]),
        },
    };
    let mut out = &stream;
    let head = format!(
        "HTTP/1.1 {status}\r\n{extra}Content-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = out.write_all(head.as_bytes());
    if method != "HEAD" {
        let _ = out.write_all(body);
    }
}

/// Local release host: serves `routes` and redirects `/latest` to `/tag/<latest>`.
fn serve(routes: Vec<(String, Vec<u8>)>, latest: Option<&str>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let routes: Arc<HashMap<String, Vec<u8>>> = Arc::new(routes.into_iter().collect());
    let latest = latest.map(str::to_string);
    let base_url = base.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let (routes, latest, base) = (routes.clone(), latest.clone(), base_url.clone());
            std::thread::spawn(move || handle(stream, &base, &routes, latest.as_deref()));
        }
    });
    base
}

fn release_routes(tag: &str, archive: Vec<u8>, checksum: String) -> Vec<(String, Vec<u8>)> {
    let (triple, ext) = target().unwrap();
    let name = format!("dt-{tag}-{triple}.{ext}");
    vec![
        (format!("/download/{tag}/{name}"), archive),
        (format!("/download/{tag}/{name}.sha256"), checksum.into_bytes()),
    ]
}

fn checksum_line(tag: &str, archive: &[u8]) -> String {
    let (triple, ext) = target().unwrap();
    format!("{}  dt-{tag}-{triple}.{ext}\n", hex::encode(Sha256::digest(archive)))
}

/// A copy of the dt binary in its own folder, plus a scratch TMPDIR to prove cleanup.
struct Sandbox {
    _root: TempDir,
    dir: PathBuf,
    exe: PathBuf,
    scratch: PathBuf,
}

impl Sandbox {
    fn new() -> Self {
        let root = TempDir::new().unwrap();
        let dir = root.path().join("bin");
        let scratch = root.path().join("tmp");
        std::fs::create_dir(&dir).unwrap();
        std::fs::create_dir(&scratch).unwrap();
        let exe = dir.join("dt");
        std::fs::copy(env!("CARGO_BIN_EXE_dt"), &exe).unwrap();
        Self {
            _root: root,
            dir,
            exe,
            scratch,
        }
    }

    fn run(&self, args: &[&str], releases: &str) -> Out {
        run_exe(&self.exe, args, releases, &self.scratch)
    }

    fn original(&self) -> Vec<u8> {
        std::fs::read(env!("CARGO_BIN_EXE_dt")).unwrap()
    }

    fn assert_untouched(&self) {
        assert_eq!(std::fs::read(&self.exe).unwrap(), self.original(), "binary berubah");
        assert_clean(&self.dir, &[self.exe.file_name().unwrap().to_str().unwrap()]);
        assert_eq!(
            names(&self.scratch),
            Vec::<String>::new(),
            "folder sementara tertinggal"
        );
    }
}

fn names(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    v.sort();
    v
}

fn assert_clean(dir: &Path, expected: &[&str]) {
    assert_eq!(names(dir), expected, "berkas sisa di {}", dir.display());
}

fn run_exe(exe: &Path, args: &[&str], releases: &str, scratch: &Path) -> Out {
    let mut attempt = 0;
    loop {
        let mut cmd = Command::new(exe);
        cmd.args(args).env_clear().stdin(Stdio::null());
        for key in ["PATH", "SystemRoot"] {
            if let Some(v) = std::env::var_os(key) {
                cmd.env(key, v);
            }
        }
        for key in ["TMPDIR", "TEMP", "TMP"] {
            cmd.env(key, scratch);
        }
        cmd.env("DT_RELEASES_URL", releases);
        match cmd.output() {
            Ok(o) => {
                return Out {
                    code: o.status.code().unwrap_or(-1),
                    out: String::from_utf8_lossy(&o.stdout).into_owned(),
                    err: String::from_utf8_lossy(&o.stderr).into_owned(),
                    out_bytes: o.stdout,
                };
            }
            // ETXTBSY: a parallel test's fork briefly holds the freshly copied binary open for writing.
            Err(e) if cfg!(unix) && e.raw_os_error() == Some(26) && attempt < 50 => {
                attempt += 1;
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            Err(e) => panic!("gagal menjalankan {}: {e}", exe.display()),
        }
    }
}

fn json(out: &Out) -> serde_json::Value {
    serde_json::from_str(&out.out).unwrap_or_else(|e| panic!("bukan JSON: {e}\n{}", out.out))
}

fn check(releases: &str, args: &[&str]) -> Out {
    let mut full = vec!["update", "--check"];
    full.extend_from_slice(args);
    dt_env(&full, b"", &[("DT_RELEASES_URL", releases)])
}

// ---- help and arguments ----

#[test]
fn help_exits_zero_and_documents_options() {
    let o = dt_env(&["update", "--help"], b"", &[]);
    assert_eq!(o.code, 0);
    for needle in ["--check", "--version", "--force", "--json", "DT_RELEASES_URL"] {
        assert!(o.out.contains(needle), "{needle}: {}", o.out);
    }
    assert_eq!(dt_env(&["update", "-h"], b"", &[]).out, o.out);
}

#[test]
fn stray_argument_is_usage_error() {
    let o = dt_env(&["update", "v1.0.0"], b"", &[]);
    assert_eq!(o.code, 2);
    assert!(o.err.contains("Argumen tidak dikenal: v1.0.0"), "{}", o.err);
}

#[test]
fn unknown_option_is_usage_error() {
    assert_eq!(dt_env(&["update", "--bogus"], b"", &[]).code, 2);
}

#[test]
fn invalid_version_is_usage_error_before_any_network() {
    for bad in ["1.x", "latest", "vv1.0.0", "1.0"] {
        let o = check("http://127.0.0.1:1", &["--version", bad]);
        assert_eq!(o.code, 2, "{bad}");
        assert!(o.err.contains("Versi tidak valid"), "{bad}: {}", o.err);
    }
}

// ---- --check ----

#[test]
fn check_with_version_compares_without_network_or_curl() {
    // Empty PATH and a dead URL: only a pure comparison can succeed.
    let o = dt_env(
        &["update", "--check", "--version", "v9.9.9"],
        b"",
        &[("PATH", ""), ("DT_RELEASES_URL", "http://127.0.0.1:1")],
    );
    assert_eq!(o.code, 0, "{}", o.err);
    assert!(o.out.contains(&format!("Versi saat ini : {CURRENT}")), "{}", o.out);
    assert!(o.out.contains("Versi terbaru  : 9.9.9"), "{}", o.out);
    assert!(o.out.contains("Pembaruan tersedia"), "{}", o.out);
}

#[test]
fn check_with_older_version_reports_no_update_and_exits_zero() {
    let o = check("http://127.0.0.1:1", &["--version", "1.0.0"]);
    assert_eq!(o.code, 0, "{}", o.err);
    assert!(o.out.contains("Versi terbaru  : 1.0.0"), "{}", o.out);
    assert!(o.out.contains("dt sudah versi terbaru."), "{}", o.out);
}

#[test]
fn check_json_with_version_has_documented_shape() {
    let o = check("http://127.0.0.1:1", &["--version", "9.9.9", "--json"]);
    assert_eq!(o.code, 0, "{}", o.err);
    let v = json(&o);
    assert_eq!(v["current"], CURRENT);
    assert_eq!(v["latest"], "9.9.9");
    assert_eq!(v["available"], true);
    assert_eq!(v["updated"], false);
    match target() {
        Some((triple, _)) => assert_eq!(v["target"], triple.as_str()),
        None => assert!(v.get("target").is_none()),
    }
}

#[test]
fn check_resolves_latest_through_the_redirect() {
    require_curl!();
    let base = serve(vec![], Some("v9.9.9"));
    let o = check(&base, &[]);
    assert_eq!(o.code, 0, "{}", o.err);
    assert!(o.out.contains("Versi terbaru  : 9.9.9"), "{}", o.out);
    assert!(o.out.contains("Pembaruan tersedia"), "{}", o.out);
    let v = json(&check(&base, &["--json"]));
    assert_eq!(
        (v["latest"].as_str(), v["available"].as_bool()),
        (Some("9.9.9"), Some(true))
    );
}

#[test]
fn check_trims_trailing_slash_of_releases_url() {
    require_curl!();
    let base = serve(vec![], Some("v9.9.9"));
    let o = check(&format!("{base}/"), &[]);
    assert_eq!(o.code, 0, "{}", o.err);
    assert!(o.out.contains("9.9.9"), "{}", o.out);
}

#[test]
fn check_when_current_is_latest_reports_up_to_date() {
    require_curl!();
    let base = serve(vec![], Some(&format!("v{CURRENT}")));
    let o = check(&base, &[]);
    assert_eq!(o.code, 0, "{}", o.err);
    assert!(o.out.contains("dt sudah versi terbaru."), "{}", o.out);
    assert_eq!(json(&check(&base, &["--json"]))["available"], false);
}

#[test]
fn check_when_current_is_newer_than_latest_is_not_an_update() {
    require_curl!();
    let base = serve(vec![], Some("v0.0.1"));
    let o = check(&base, &["--json"]);
    assert_eq!(o.code, 0, "{}", o.err);
    assert_eq!(json(&o)["available"], false);
}

#[test]
fn no_stable_release_exits_one() {
    require_curl!();
    let base = serve(vec![], None);
    let o = check(&base, &[]);
    assert_eq!(o.code, 1);
    assert!(o.err.contains("Tidak ada rilis stabil"), "{}", o.err);
}

#[test]
fn redirect_to_a_non_tag_page_exits_one() {
    require_curl!();
    // Serves 200 on /latest itself: url_effective has no /tag/ part.
    let base = serve(vec![("/latest".into(), vec![])], None);
    let o = check(&base, &[]);
    assert_eq!(o.code, 1);
    assert!(o.err.contains("Tidak ada rilis stabil"), "{}", o.err);
}

#[cfg(unix)]
#[test]
fn missing_curl_and_wget_is_a_clear_error() {
    let o = dt_env(&["update", "--check"], b"", &[("PATH", "")]);
    assert_eq!(o.code, 1);
    assert!(o.err.contains("curl atau wget"), "{}", o.err);
}

// ---- update: nothing replaced ----

#[test]
fn already_latest_without_force_changes_nothing() {
    require_curl!();
    let _ = require_target!();
    let base = serve(vec![], Some(&format!("v{CURRENT}")));
    let sb = Sandbox::new();
    let o = sb.run(&["update"], &base);
    assert_eq!(o.code, 0, "{}", o.err);
    assert!(
        o.out.contains(&format!("dt sudah versi terbaru ({CURRENT}).")),
        "{}",
        o.out
    );
    sb.assert_untouched();
    let j = sb.run(&["update", "--json"], &base);
    let v = json(&j);
    assert_eq!(
        (v["updated"].as_bool(), v["available"].as_bool()),
        (Some(false), Some(false))
    );
}

#[test]
fn pinned_current_version_without_force_changes_nothing() {
    require_curl!();
    let _ = require_target!();
    let sb = Sandbox::new();
    let o = sb.run(&["update", "--version", &format!("v{CURRENT}")], "http://127.0.0.1:1");
    assert_eq!(o.code, 0, "{}", o.err);
    assert!(o.out.contains("sudah versi terbaru"), "{}", o.out);
    sb.assert_untouched();
}

#[test]
fn missing_asset_exits_one_and_leaves_no_trace() {
    require_curl!();
    let _ = require_target!();
    let base = serve(vec![], None);
    let sb = Sandbox::new();
    let o = sb.run(&["update", "--version", "v9.9.9"], &base);
    assert_eq!(o.code, 1);
    assert!(o.err.contains("Gagal mengunduh"), "{}", o.err);
    assert!(o.err.contains("dt-v9.9.9-"), "nama aset harus disebut: {}", o.err);
    assert!(!o.out.contains("Berhasil"), "{}", o.out);
    sb.assert_untouched();
}

#[test]
fn missing_checksum_asset_exits_one_and_leaves_no_trace() {
    require_curl!();
    let _ = require_target!();
    let mut routes = release_routes("v9.9.9", b"archive".to_vec(), String::new());
    routes.pop();
    let base = serve(routes, None);
    let sb = Sandbox::new();
    let o = sb.run(&["update", "--version", "v9.9.9"], &base);
    assert_eq!(o.code, 1);
    assert!(o.err.contains(".sha256"), "{}", o.err);
    sb.assert_untouched();
}

#[test]
fn checksum_mismatch_aborts_leaving_binary_identical() {
    require_curl!();
    let _ = require_target!();
    let archive = b"not really an archive".to_vec();
    let wrong = checksum_line("v9.9.9", b"something else");
    let base = serve(release_routes("v9.9.9", archive, wrong), None);
    let sb = Sandbox::new();
    let o = sb.run(&["update", "--version", "9.9.9"], &base);
    assert_eq!(o.code, 1);
    assert!(o.err.contains("Checksum tidak cocok"), "{}", o.err);
    assert!(!o.out.contains("Berhasil"), "{}", o.out);
    sb.assert_untouched();
}

#[test]
fn malformed_checksum_file_aborts() {
    require_curl!();
    let _ = require_target!();
    let base = serve(release_routes("v9.9.9", b"x".to_vec(), "bukan hash\n".into()), None);
    let sb = Sandbox::new();
    let o = sb.run(&["update", "--version", "v9.9.9"], &base);
    assert_eq!(o.code, 1);
    assert!(o.err.contains("Format checksum"), "{}", o.err);
    sb.assert_untouched();
}

#[test]
fn json_output_is_not_printed_on_failure() {
    require_curl!();
    let _ = require_target!();
    let base = serve(vec![], None);
    let sb = Sandbox::new();
    let o = sb.run(&["update", "--version", "v9.9.9", "--json"], &base);
    assert_eq!(o.code, 1);
    assert_eq!(o.out, "", "stdout harus kosong saat gagal");
}

// ---- update: replacing the binary ----

#[cfg(unix)]
mod replace {
    use super::*;
    use flate2::Compression;
    use flate2::write::GzEncoder;
    use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};

    const PAYLOAD: &[u8] = b"#!/bin/sh\necho fake-dt 9.9.9\n";

    fn tar_gz(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut builder = tar::Builder::new(GzEncoder::new(Vec::new(), Compression::default()));
        for (path, data) in entries {
            let mut header = tar::Header::new_gnu();
            header.set_size(data.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder.append_data(&mut header, path, *data).unwrap();
        }
        builder.into_inner().unwrap().finish().unwrap()
    }

    fn release(tag: &str, entries: &[(&str, &[u8])]) -> Vec<(String, Vec<u8>)> {
        let archive = tar_gz(entries);
        let checksum = checksum_line(tag, &archive);
        release_routes(tag, archive, checksum)
    }

    fn good_release(tag: &str) -> Vec<(String, Vec<u8>)> {
        let (triple, _) = target().unwrap();
        let dt = format!("dt-{tag}-{triple}/dt");
        let readme = format!("dt-{tag}-{triple}/README.md");
        release(tag, &[(&readme, b"readme"), (&dt, PAYLOAD)])
    }

    #[test]
    fn pinned_version_replaces_binary_with_payload() {
        require_curl!();
        let _ = require_target!();
        let base = serve(good_release("v9.9.9"), None);
        let sb = Sandbox::new();
        let o = sb.run(&["update", "--version", "v9.9.9"], &base);
        assert_eq!(o.code, 0, "{}", o.err);
        assert!(
            o.out.contains(&format!("Memperbarui dt {CURRENT} -> 9.9.9")),
            "{}",
            o.out
        );
        assert!(o.out.contains("Berhasil diperbarui ke v9.9.9"), "{}", o.out);
        assert_eq!(std::fs::read(&sb.exe).unwrap(), PAYLOAD);
        assert_eq!(std::fs::metadata(&sb.exe).unwrap().permissions().mode() & 0o777, 0o755);
        assert_clean(&sb.dir, &["dt"]);
        assert_eq!(names(&sb.scratch), Vec::<String>::new());
        // The replaced binary is the one that now runs.
        let ran = Command::new(&sb.exe).output().unwrap();
        assert_eq!(String::from_utf8_lossy(&ran.stdout), "fake-dt 9.9.9\n");
    }

    #[test]
    fn version_without_v_prefix_is_accepted() {
        require_curl!();
        let _ = require_target!();
        let base = serve(good_release("v9.9.9"), None);
        let sb = Sandbox::new();
        let o = sb.run(&["update", "--version", "9.9.9"], &base);
        assert_eq!(o.code, 0, "{}", o.err);
        assert_eq!(std::fs::read(&sb.exe).unwrap(), PAYLOAD);
    }

    #[test]
    fn latest_is_resolved_and_installed_without_version_flag() {
        require_curl!();
        let _ = require_target!();
        let base = serve(good_release("v9.9.9"), Some("v9.9.9"));
        let sb = Sandbox::new();
        let o = sb.run(&["update"], &base);
        assert_eq!(o.code, 0, "{}", o.err);
        assert!(o.out.contains("Berhasil diperbarui ke v9.9.9"), "{}", o.out);
        assert_eq!(std::fs::read(&sb.exe).unwrap(), PAYLOAD);
    }

    #[test]
    fn json_reports_updated_with_target() {
        require_curl!();
        let (triple, _) = require_target!();
        let base = serve(good_release("v9.9.9"), None);
        let sb = Sandbox::new();
        let o = sb.run(&["update", "--version", "v9.9.9", "--json"], &base);
        assert_eq!(o.code, 0, "{}", o.err);
        let v = json(&o);
        assert_eq!(v["current"], CURRENT);
        assert_eq!(v["latest"], "9.9.9");
        assert_eq!(v["available"], true);
        assert_eq!(v["updated"], true);
        assert_eq!(v["target"], triple.as_str());
        assert_eq!(std::fs::read(&sb.exe).unwrap(), PAYLOAD);
    }

    #[test]
    fn force_reinstalls_the_current_version() {
        require_curl!();
        let _ = require_target!();
        let tag = format!("v{CURRENT}");
        let base = serve(good_release(&tag), Some(&tag));
        let sb = Sandbox::new();
        let o = sb.run(&["update", "--force"], &base);
        assert_eq!(o.code, 0, "{}", o.err);
        assert!(o.out.contains(&format!("Berhasil diperbarui ke {tag}")), "{}", o.out);
        assert_eq!(std::fs::read(&sb.exe).unwrap(), PAYLOAD);
    }

    #[test]
    fn pinned_older_prerelease_version_can_be_installed() {
        require_curl!();
        let _ = require_target!();
        let base = serve(good_release("v0.1.0-rc.1"), None);
        let sb = Sandbox::new();
        let o = sb.run(&["update", "--version", "v0.1.0-rc.1"], &base);
        assert_eq!(o.code, 0, "{}", o.err);
        assert!(o.out.contains("Berhasil diperbarui ke v0.1.0-rc.1"), "{}", o.out);
        assert_eq!(std::fs::read(&sb.exe).unwrap(), PAYLOAD);
    }

    #[test]
    fn archive_without_dt_entry_aborts_untouched() {
        require_curl!();
        let (triple, _) = require_target!();
        let readme = format!("dt-v9.9.9-{triple}/README.md");
        // Wrong folder too: dt must sit directly under dt-<tag>-<target>/.
        let elsewhere = "other/dt";
        let base = serve(release("v9.9.9", &[(&readme, b"readme"), (elsewhere, PAYLOAD)]), None);
        let sb = Sandbox::new();
        let o = sb.run(&["update", "--version", "v9.9.9"], &base);
        assert_eq!(o.code, 1);
        assert!(o.err.contains("Arsip tidak berisi dt"), "{}", o.err);
        sb.assert_untouched();
    }

    #[test]
    fn empty_dt_entry_aborts_untouched() {
        require_curl!();
        let (triple, _) = require_target!();
        let dt = format!("dt-v9.9.9-{triple}/dt");
        let base = serve(release("v9.9.9", &[(&dt, b"")]), None);
        let sb = Sandbox::new();
        let o = sb.run(&["update", "--version", "v9.9.9"], &base);
        assert_eq!(o.code, 1);
        assert!(o.err.contains("kosong"), "{}", o.err);
        sb.assert_untouched();
    }

    #[test]
    fn corrupt_archive_with_valid_checksum_aborts_untouched() {
        require_curl!();
        let _ = require_target!();
        let archive = b"definitely not gzip".to_vec();
        let checksum = checksum_line("v9.9.9", &archive);
        let base = serve(release_routes("v9.9.9", archive, checksum), None);
        let sb = Sandbox::new();
        let o = sb.run(&["update", "--version", "v9.9.9"], &base);
        assert_eq!(o.code, 1);
        assert!(o.err.contains("Gagal mengekstrak"), "{}", o.err);
        sb.assert_untouched();
    }

    #[test]
    fn symlinked_executable_replaces_the_real_file() {
        require_curl!();
        let _ = require_target!();
        let base = serve(good_release("v9.9.9"), None);
        let sb = Sandbox::new();
        let link_dir = sb.dir.parent().unwrap().join("links");
        std::fs::create_dir(&link_dir).unwrap();
        let link = link_dir.join("dt");
        symlink(&sb.exe, &link).unwrap();
        let o = run_exe(&link, &["update", "--version", "v9.9.9"], &base, &sb.scratch);
        assert_eq!(o.code, 0, "{}", o.err);
        assert!(std::fs::symlink_metadata(&link).unwrap().file_type().is_symlink());
        assert_eq!(std::fs::read(&sb.exe).unwrap(), PAYLOAD);
        assert_clean(&link_dir, &["dt"]);
        assert_clean(&sb.dir, &["dt"]);
    }

    #[test]
    fn unwritable_directory_fails_clearly_before_downloading() {
        require_curl!();
        let _ = require_target!();
        // root ignores directory permissions.
        if unsafe { libc::geteuid() } == 0 {
            eprintln!("skip: berjalan sebagai root");
            return;
        }
        struct Restore(PathBuf);
        impl Drop for Restore {
            fn drop(&mut self) {
                let _ = std::fs::set_permissions(&self.0, std::fs::Permissions::from_mode(0o755));
            }
        }
        let base = serve(good_release("v9.9.9"), None);
        let sb = Sandbox::new();
        let before = std::fs::metadata(&sb.exe).unwrap().ino();
        std::fs::set_permissions(&sb.dir, std::fs::Permissions::from_mode(0o555)).unwrap();
        let _restore = Restore(sb.dir.clone());
        let o = sb.run(&["update", "--version", "v9.9.9"], &base);
        assert_eq!(o.code, 1);
        assert!(o.err.contains("Tidak bisa menulis ke"), "{}", o.err);
        assert!(o.err.contains("install.sh"), "{}", o.err);
        assert!(!o.out.contains("Berhasil"), "{}", o.out);
        assert_eq!(std::fs::metadata(&sb.exe).unwrap().ino(), before);
        assert_eq!(names(&sb.scratch), Vec::<String>::new());
    }
}

#[test]
fn unreachable_server_reports_the_network_cause_not_a_missing_release() {
    require_curl!();
    // Port 1 on loopback refuses connections.
    let o = check("http://127.0.0.1:1", &[]);
    assert_eq!(o.code, 1);
    assert!(o.err.contains("Gagal mengakses"), "{}", o.err);
    assert!(!o.err.contains("Tidak ada rilis stabil"), "{}", o.err);
}
