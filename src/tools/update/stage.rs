use crate::errors::{CliError, Result};
use std::fs::{self, File, OpenOptions};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

fn failed(message: impl Into<String>) -> CliError {
    CliError::Failed(message.into())
}

fn random_hex() -> Result<String> {
    let mut buf = [0u8; 8];
    getrandom::fill(&mut buf).map_err(|_| failed("Sumber acak OS tidak tersedia."))?;
    Ok(hex::encode(buf))
}

/// Private scratch folder, removed on every path (success, error, early return).
pub struct TempDir(PathBuf);

impl TempDir {
    pub fn create() -> Result<Self> {
        let base = std::env::temp_dir();
        for _ in 0..8 {
            let path = base.join(format!("dt-update-{}-{}", std::process::id(), random_hex()?));
            #[cfg_attr(not(unix), allow(unused_mut))]
            let mut builder = fs::DirBuilder::new();
            #[cfg(unix)]
            std::os::unix::fs::DirBuilderExt::mode(&mut builder, 0o700);
            match builder.create(&path) {
                Ok(()) => return Ok(Self(path)),
                Err(e) if e.kind() == ErrorKind::AlreadyExists => {}
                Err(e) => {
                    return Err(failed(format!(
                        "Gagal membuat folder sementara {}: {e}",
                        path.display()
                    )));
                }
            }
        }
        Err(failed("Gagal membuat folder sementara."))
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// New binary written next to the target; deleted unless `commit` succeeds.
pub struct Staged {
    path: PathBuf,
    armed: bool,
}

fn write_error(dir: &Path, e: &std::io::Error) -> CliError {
    match e.kind() {
        ErrorKind::PermissionDenied | ErrorKind::ReadOnlyFilesystem => failed(format!(
            "Tidak bisa menulis ke {}: {e}. Jalankan ulang dengan izin yang sesuai atau pasang ulang dengan install.sh.",
            dir.display()
        )),
        _ => failed(format!("Gagal menulis ke {}: {e}", dir.display())),
    }
}

impl Staged {
    /// Creates the staging file in `dir` (executable on unix) and returns it open for writing.
    pub fn create(dir: &Path) -> Result<(Self, File)> {
        for _ in 0..8 {
            let path = dir.join(format!(".dt-update-{}.tmp", random_hex()?));
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o755);
            match options.open(&path) {
                Ok(file) => return Ok((Self { path, armed: true }, file)),
                Err(e) if e.kind() == ErrorKind::AlreadyExists => {}
                Err(e) => return Err(write_error(dir, &e)),
            }
        }
        Err(failed("Gagal membuat berkas sementara."))
    }

    /// Moves the staged file over `target`. The file must already be closed.
    pub fn commit(mut self, target: &Path) -> Result<()> {
        replace(&self.path, target)?;
        self.armed = false;
        Ok(())
    }
}

impl Drop for Staged {
    fn drop(&mut self) {
        if self.armed {
            let _ = fs::remove_file(&self.path);
        }
    }
}

fn replace(staged: &Path, target: &Path) -> Result<()> {
    fs::rename(staged, target).map_err(|e| failed(format!("Gagal mengganti {}: {e}", target.display())))
}

/// Path of the running executable with symlinks resolved.
pub fn current_exe() -> Result<PathBuf> {
    let exe = std::env::current_exe().map_err(|e| failed(format!("Gagal menentukan lokasi dt: {e}")))?;
    exe.canonicalize()
        .map_err(|e| failed(format!("Gagal menentukan lokasi dt ({}): {e}", exe.display())))
}

pub fn display(path: &Path) -> String {
    path.display().to_string()
}
