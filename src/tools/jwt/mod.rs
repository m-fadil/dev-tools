use crate::errors::{CliError, Result};
use crate::tool::Tool;
use std::ffi::OsString;

pub static TOOL: Tool = Tool {
    name: "jwt",
    aliases: &[],
    description: "Decode, verifikasi, dan buat JWT HMAC (HS256/384/512)",
    category: "Keamanan & Kripto",
    run,
};

fn run(_args: Vec<OsString>) -> Result<()> {
    Err(CliError::Failed("belum diporting".into()))
}
