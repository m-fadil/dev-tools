use crate::errors::{CliError, Result};
use std::ffi::OsStr;
use std::path::Path;
use std::process::{Command, Output, Stdio};

/// HTTP through an external `curl` (preferred) or `wget`; the binary links no TLS stack.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Client {
    program: &'static str,
}

const NULL_DEVICE: &str = "/dev/null";

fn failed(message: impl Into<String>) -> CliError {
    CliError::Failed(message.into())
}

fn spawn(program: &str, args: &[&OsStr]) -> Result<Output> {
    Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| failed(format!("Gagal menjalankan {program}: {e}")))
}

/// First non-empty line of a tool's stderr, for error context.
fn detail(output: &Output) -> Option<String> {
    String::from_utf8_lossy(&output.stderr)
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .map(str::to_string)
}

impl Client {
    pub fn detect() -> Result<Self> {
        for program in ["curl", "wget"] {
            let found = Command::new(program)
                .arg("--version")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .is_ok();
            if found {
                return Ok(Self { program });
            }
        }
        Err(failed(
            "Butuh curl atau wget untuk mengunduh pembaruan. Pasang salah satunya, lalu ulangi.",
        ))
    }

    /// Downloads `url` into `dest`, failing on HTTP errors.
    pub fn fetch(&self, url: &str, dest: &Path) -> Result<()> {
        let output = if self.program == "curl" {
            spawn(
                "curl",
                &[
                    "-fsSL".as_ref(),
                    "--retry".as_ref(),
                    "3".as_ref(),
                    "-o".as_ref(),
                    dest.as_os_str(),
                    url.as_ref(),
                ],
            )?
        } else {
            spawn("wget", &["-nv".as_ref(), "-O".as_ref(), dest.as_os_str(), url.as_ref()])?
        };
        if output.status.success() {
            return Ok(());
        }
        Err(failed(match detail(&output) {
            Some(d) => format!("Gagal mengunduh {url}: {d}"),
            None => format!("Gagal mengunduh {url}"),
        }))
    }

    /// Final URL after following redirects (curl), or the first `Location` header (wget).
    pub fn redirect_target(&self, url: &str) -> Result<String> {
        if self.program == "curl" {
            let output = spawn(
                "curl",
                &[
                    "-fsSLI".as_ref(),
                    "-o".as_ref(),
                    NULL_DEVICE.as_ref(),
                    "-w".as_ref(),
                    "%{url_effective}".as_ref(),
                    url.as_ref(),
                ],
            )?;
            if !output.status.success() {
                return Err(failed(match detail(&output) {
                    Some(d) => format!("Gagal mengakses {url}: {d}"),
                    None => format!("Gagal mengakses {url}"),
                }));
            }
            return Ok(String::from_utf8_lossy(&output.stdout).trim().to_string());
        }
        // wget exits non-zero on the unfollowed redirect; only the printed target matters.
        // wget 1.x reports it on stderr, wget2 on stdout.
        let output = spawn(
            "wget",
            &[
                "-S".as_ref(),
                "--spider".as_ref(),
                "--max-redirect=0".as_ref(),
                url.as_ref(),
            ],
        )?;
        let (stderr, stdout) = (
            String::from_utf8_lossy(&output.stderr),
            String::from_utf8_lossy(&output.stdout),
        );
        parse_wget_location(&stderr)
            .or_else(|| parse_wget_location(&stdout))
            .map(str::to_string)
            .ok_or_else(|| failed(format!("Gagal mengakses {url}")))
    }
}

/// Redirect target in `wget -S` output: the first `Location:` header (wget 1.x), or the
/// `URL '<target>' not followed` notice (wget2, which prints no headers here).
pub fn parse_wget_location(stderr: &str) -> Option<&str> {
    stderr.lines().find_map(|line| {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("URL '")
            && let Some((target, _)) = rest.split_once("' not followed")
        {
            return (!target.is_empty()).then_some(target);
        }
        let (name, value) = line.split_once(':')?;
        if !name.eq_ignore_ascii_case("location") {
            return None;
        }
        let value = value.trim();
        (!value.is_empty()).then_some(value)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wget_location_header_is_found_case_insensitively() {
        let out =
            "  HTTP/1.1 302 Found\n  Server: x\n  location: https://h/r/releases/tag/v1.0.0\r\n  Content-Length: 0\n";
        assert_eq!(parse_wget_location(out), Some("https://h/r/releases/tag/v1.0.0"));
    }

    #[test]
    fn wget_without_location_yields_none() {
        assert_eq!(parse_wget_location("  HTTP/1.1 404 Not Found\n"), None);
        assert_eq!(parse_wget_location("Location:\n"), None);
    }

    #[test]
    fn wget2_not_followed_notice_is_found() {
        let out = "HTTP response 302 Found [https://h/latest]\nURL 'https://h/r/releases/tag/v1.0.0' not followed (max redirections exceeded)\n";
        assert_eq!(parse_wget_location(out), Some("https://h/r/releases/tag/v1.0.0"));
        assert_eq!(parse_wget_location("URL '' not followed\n"), None);
    }
}
