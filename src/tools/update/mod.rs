mod http;
mod package;
mod stage;
mod target;

use crate::errors::{CliError, Result, usage};
use crate::io::{parse, print_json};
use crate::tool::Tool;
use crate::tools::semver::version::{Version, compare, try_parse};
use clap::Parser;
use http::Client;
use serde_json::{Map, Value};
use std::cmp::Ordering;
use std::ffi::OsString;
use std::io::Write;
use target::Target;

pub static TOOL: Tool = Tool {
    name: "update",
    aliases: &[],
    description: "Periksa dan pasang rilis dt terbaru dari GitHub Releases",
    category: "Utilitas",
    run,
};

const HELP: &str = "dt update [options]

Periksa dan pasang rilis dt dari GitHub Releases, dengan cara yang sama seperti install.sh:
unduh arsip + .sha256, verifikasi SHA-256, lalu ganti binary dt yang sedang berjalan.
Tanpa opsi: pasang rilis stabil terbaru bila lebih baru dari versi saat ini.
      --check            Hanya laporkan versi saat ini, terbaru, dan ketersediaan pembaruan (tidak memasang)
      --version <tag>    Pasang versi tertentu (v1.2.3 atau 1.2.3; boleh prerelease atau lebih lama).
                         Dengan --check tidak ada akses jaringan: hanya membandingkan dengan versi ini
      --force            Pasang ulang meski sudah versi yang sama atau lebih baru
      --json             Keluaran JSON: {current, latest, available, updated, target?}
  -h, --help             Tampilkan bantuan

Butuh curl atau wget. DT_RELEASES_URL mengganti dasar URL rilis
(default: https://github.com/m-fadil/dev-tools/releases).
Rilis prerelease hanya bisa dipasang lewat --version. Binary hasil cargo install
akan diganti dengan build rilis (musl di Linux).
Exit: 0 sukses (termasuk --check dan sudah terbaru), 1 gagal, 2 penggunaan salah.
Contoh: dt update --check   |   dt update --version vX.Y.Z
";

const DEFAULT_RELEASES: &str = "https://github.com/m-fadil/dev-tools/releases";

#[derive(Parser)]
#[command(name = "dt update", disable_help_flag = true, disable_version_flag = true)]
struct Args {
    #[arg(long, overrides_with = "check")]
    check: bool,
    #[arg(long, overrides_with = "force")]
    force: bool,
    #[arg(long, overrides_with = "json")]
    json: bool,
    #[arg(short = 'h', long, overrides_with = "help")]
    help: bool,
    #[arg(long, overrides_with = "version")]
    version: Option<String>,
    positionals: Vec<String>,
}

fn say(text: &str) {
    let _ = writeln!(std::io::stdout(), "{text}");
}

fn releases_url() -> String {
    let url = std::env::var("DT_RELEASES_URL").ok().filter(|v| !v.is_empty());
    let url = url.as_deref().unwrap_or(DEFAULT_RELEASES);
    url.trim_end_matches('/').to_string()
}

/// `--version` value as a release tag: one `v` prefix, strict SemVer behind it.
fn normalize_tag(text: &str) -> Result<String> {
    let tag = if text.starts_with('v') {
        text.to_string()
    } else {
        format!("v{text}")
    };
    try_parse(&tag).map_err(|e| usage(format!("Versi tidak valid, {e}")))?;
    Ok(tag)
}

/// Tag from the `.../releases/tag/vX` URL that `releases/latest` redirects to.
fn tag_from_location(location: &str) -> Option<&str> {
    let (_, tail) = location.rsplit_once("/tag/")?;
    let tag = tail.split(['?', '#']).next()?.trim();
    (tag.starts_with('v') && !tag.contains('/')).then_some(tag)
}

fn resolve_latest(client: &Client, releases: &str) -> Result<String> {
    let none = || CliError::Failed("Tidak ada rilis stabil. Tentukan versi: dt update --version vX.Y.Z".into());
    // GitHub answers 404 for /releases/latest while a repository has no stable release;
    // every other failure (DNS, refused, TLS, 5xx) is reported with its own cause.
    let location = match client.redirect_target(&format!("{releases}/latest")) {
        Err(CliError::Failed(m)) if m.contains("404") => return Err(none()),
        other => other?,
    };
    let tag = tag_from_location(&location).ok_or_else(none)?;
    try_parse(tag).map_err(|e| CliError::Failed(format!("Tag rilis terbaru tidak valid, {e}")))?;
    Ok(tag.to_string())
}

/// Whether to install: pinned versions install whenever they differ, latest only when newer.
fn needs_update(current: &Version, target: &Version, pinned: bool, force: bool) -> bool {
    force
        || match compare(current, target) {
            Ordering::Less => true,
            Ordering::Equal => false,
            Ordering::Greater => pinned,
        }
}

fn report(current: &str, latest: &str, available: bool, updated: bool, triple: Option<&str>) -> Value {
    let mut doc = Map::new();
    doc.insert("current".into(), current.into());
    doc.insert("latest".into(), latest.into());
    doc.insert("available".into(), available.into());
    doc.insert("updated".into(), updated.into());
    if let Some(t) = triple {
        doc.insert("target".into(), t.into());
    }
    Value::Object(doc)
}

/// Downloads, verifies and swaps in the release binary. Returns the replaced path.
fn install(client: &Client, releases: &str, tag: &str, target: &Target) -> Result<String> {
    let exe = stage::current_exe()?;
    let dir = exe
        .parent()
        .ok_or_else(|| CliError::Failed(format!("Gagal menentukan folder dt: {}", stage::display(&exe))))?;
    // Staged first: an unwritable directory fails before anything is downloaded.
    let (staged, mut file) = stage::Staged::create(dir)?;
    let tmp = stage::TempDir::create()?;

    let stem = target.stem(tag);
    let archive = format!("{stem}.{}", target.ext);
    let base = format!("{releases}/download/{tag}");
    let archive_path = tmp.path().join(&archive);
    let checksum_path = tmp.path().join(format!("{archive}.sha256"));
    client
        .fetch(&format!("{base}/{archive}"), &archive_path)
        .map_err(|e| CliError::Failed(format!("{e}\nPeriksa apakah versi dan platform ini tersedia di rilis.")))?;
    client.fetch(&format!("{base}/{archive}.sha256"), &checksum_path)?;
    package::verify_checksum(&archive_path, &checksum_path, &archive)?;

    package::extract(&archive_path, &stem, target.bin, &mut file)?;
    // The creation mode is subject to umask; pin it to 0755 like install.sh's chmod.
    #[cfg(unix)]
    file.set_permissions(std::os::unix::fs::PermissionsExt::from_mode(0o755))
        .map_err(|e| CliError::Failed(format!("Gagal mengatur izin dt: {e}")))?;
    drop(file);
    staged.commit(&exe)?;
    Ok(stage::display(&exe))
}

fn run(args: Vec<OsString>) -> Result<()> {
    let a: Args = parse("update", args)?;
    if a.help {
        let _ = write!(std::io::stdout(), "{HELP}");
        return Ok(());
    }
    if let Some(p) = a.positionals.first() {
        return Err(usage(format!("Argumen tidak dikenal: {p}")));
    }
    let current_text = env!("CARGO_PKG_VERSION");
    let current = try_parse(current_text).map_err(|e| CliError::Failed(format!("Versi dt tidak valid, {e}")))?;
    let releases = releases_url();

    let pinned = a.version.is_some();
    let mut client = None;
    let tag = match a.version.as_deref() {
        Some(v) => normalize_tag(v)?,
        None => {
            let c = Client::detect()?;
            let tag = resolve_latest(&c, &releases)?;
            client = Some(c);
            tag
        }
    };
    let latest_text = &tag[1..];
    let latest = try_parse(&tag).map_err(|e| CliError::Failed(format!("Versi tidak valid, {e}")))?;
    let available = compare(&current, &latest) == Ordering::Less;

    if a.check {
        let triple = target::detect().ok().map(|t| t.triple);
        if a.json {
            print_json(&report(current_text, latest_text, available, false, triple.as_deref()));
        } else {
            say(&format!("Versi saat ini : {current_text}"));
            say(&format!("Versi terbaru  : {latest_text}"));
            say(if available {
                "Pembaruan tersedia. Jalankan: dt update"
            } else {
                "dt sudah versi terbaru."
            });
        }
        return Ok(());
    }

    if !needs_update(&current, &latest, pinned, a.force) {
        if a.json {
            print_json(&report(current_text, latest_text, available, false, None));
        } else {
            say(&format!("dt sudah versi terbaru ({current_text})."));
        }
        return Ok(());
    }

    let target = target::detect()?;
    let client = match client {
        Some(c) => c,
        None => Client::detect()?,
    };
    if !a.json {
        say(&format!(
            "Memperbarui dt {current_text} -> {latest_text} ({})",
            target.triple
        ));
    }
    let path = install(&client, &releases, &tag, &target)?;
    if a.json {
        print_json(&report(
            current_text,
            latest_text,
            available,
            true,
            Some(&target.triple),
        ));
    } else {
        say(&format!("Berhasil diperbarui ke {tag}"));
        say(&format!("Terpasang: {path}"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(text: &str) -> Version {
        try_parse(text).unwrap()
    }

    #[test]
    fn location_yields_stable_tag() {
        let url = "https://github.com/m-fadil/dev-tools/releases/tag/v1.2.3";
        assert_eq!(tag_from_location(url), Some("v1.2.3"));
        assert_eq!(
            tag_from_location("/m-fadil/dev-tools/releases/tag/v1.0.0?x=1#y"),
            Some("v1.0.0")
        );
    }

    #[test]
    fn location_without_tag_is_rejected() {
        assert_eq!(tag_from_location("https://github.com/m-fadil/dev-tools/releases"), None);
        assert_eq!(tag_from_location("https://github.com/x/y/releases/tag/1.2.3"), None);
        assert_eq!(tag_from_location("https://github.com/x/y/releases/tag/v1/extra"), None);
        assert_eq!(tag_from_location("https://github.com/x/y/releases/tag/"), None);
    }

    #[test]
    fn tag_normalization_adds_single_v_and_validates() {
        assert_eq!(normalize_tag("1.2.3").unwrap(), "v1.2.3");
        assert_eq!(normalize_tag("v1.2.3-rc.1").unwrap(), "v1.2.3-rc.1");
        for bad in ["", "v", "vv1.0.0", "1.2", "1.0.0/../x", "latest"] {
            assert!(matches!(normalize_tag(bad), Err(CliError::Usage(_))), "{bad}");
        }
    }

    #[test]
    fn latest_installs_only_newer_versions() {
        assert!(needs_update(&v("1.0.0"), &v("1.1.0"), false, false));
        assert!(!needs_update(&v("1.1.0"), &v("1.1.0"), false, false));
        assert!(!needs_update(&v("1.2.0"), &v("1.1.0"), false, false));
        assert!(needs_update(&v("1.0.0-rc.1"), &v("1.0.0"), false, false));
    }

    #[test]
    fn pinned_installs_any_different_version() {
        assert!(needs_update(&v("1.2.0"), &v("1.1.0"), true, false));
        assert!(needs_update(&v("1.0.0"), &v("1.1.0-rc.1"), true, false));
        assert!(!needs_update(&v("1.1.0"), &v("1.1.0"), true, false));
    }

    #[test]
    fn force_always_installs() {
        assert!(needs_update(&v("1.1.0"), &v("1.1.0"), false, true));
        assert!(needs_update(&v("2.0.0"), &v("1.1.0"), false, true));
    }

    #[test]
    fn report_omits_target_when_unknown() {
        let doc = report("1.0.0", "1.1.0", true, false, None);
        assert_eq!(doc.get("target"), None);
        let doc = report("1.0.0", "1.1.0", true, true, Some("x"));
        assert_eq!(doc["target"], "x");
        assert_eq!(doc["updated"], true);
    }
}
