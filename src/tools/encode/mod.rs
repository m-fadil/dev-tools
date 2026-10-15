use crate::errors::{CliError, Result};
use crate::tool::Tool;
use std::ffi::OsString;

pub static TOOL: Tool = Tool {
    name: "encode",
    aliases: &[],
    description: "Encode teks/file ke base64, base64url, hex, atau URL",
    category: "Encoding",
    run,
};

fn run(_args: Vec<OsString>) -> Result<()> {
    Err(CliError::Failed("belum diporting".into()))
}
