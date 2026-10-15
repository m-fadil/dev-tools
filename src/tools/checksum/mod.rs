use crate::errors::{CliError, Result};
use crate::tool::Tool;
use std::ffi::OsString;

pub static TOOL: Tool = Tool {
    name: "checksum",
    aliases: &["sum"],
    description: "Hitung dan verifikasi checksum file (format sha256sum)",
    category: "Keamanan & Kripto",
    run,
};

fn run(_args: Vec<OsString>) -> Result<()> {
    Err(CliError::Failed("belum diporting".into()))
}
