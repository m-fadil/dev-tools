use crate::errors::{CliError, Result};

/// Release platform, chosen at compile time exactly like install.sh's `detect_target`.
#[derive(Debug, PartialEq, Eq)]
pub struct Target {
    pub triple: String,
    /// Archive extension of the release asset.
    pub ext: &'static str,
    /// Name of the executable inside the archive.
    pub bin: &'static str,
}

const fn arch() -> Option<&'static str> {
    if cfg!(target_arch = "x86_64") {
        Some("x86_64")
    } else if cfg!(target_arch = "aarch64") {
        Some("aarch64")
    } else {
        None
    }
}

/// Linux always maps to the musl asset (static binary), as install.sh does.
const fn os() -> Option<(&'static str, &'static str, &'static str)> {
    if cfg!(target_os = "linux") {
        Some(("unknown-linux-musl", "tar.gz", "dt"))
    } else {
        None
    }
}

pub fn detect() -> Result<Target> {
    let Some(arch) = arch() else {
        return Err(CliError::Failed(format!(
            "Arsitektur tidak didukung: {}",
            std::env::consts::ARCH
        )));
    };
    let Some((suffix, ext, bin)) = os() else {
        return Err(CliError::Failed(format!(
            "Sistem operasi tidak didukung: {}",
            std::env::consts::OS
        )));
    };
    Ok(Target {
        triple: format!("{arch}-{suffix}"),
        ext,
        bin,
    })
}

impl Target {
    /// `dt-<tag>-<target>`: archive stem and top-level folder inside the archive.
    pub fn stem(&self, tag: &str) -> String {
        format!("dt-{tag}-{}", self.triple)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stem_and_triple_follow_release_naming() {
        let t = Target {
            triple: "x86_64-unknown-linux-musl".into(),
            ext: "tar.gz",
            bin: "dt",
        };
        assert_eq!(t.stem("v1.2.3"), "dt-v1.2.3-x86_64-unknown-linux-musl");
    }

    #[cfg(all(target_os = "linux", any(target_arch = "x86_64", target_arch = "aarch64")))]
    #[test]
    fn linux_uses_musl_asset() {
        let t = detect().unwrap();
        assert!(t.triple.ends_with("-unknown-linux-musl"), "{}", t.triple);
        assert_eq!((t.ext, t.bin), ("tar.gz", "dt"));
    }
}
