use crate::errors::{CliError, Result};
use crate::tool::Tool;
use std::ffi::OsString;

pub static TOOL: Tool = Tool {
    name: "encrypt",
    aliases: &[],
    description: "Enkripsi/dekripsi teks atau file dengan password (AES-256-GCM)",
    category: "Keamanan & Kripto",
    run,
};

fn run(_args: Vec<OsString>) -> Result<()> {
    Err(CliError::Failed("belum diporting".into()))
}
