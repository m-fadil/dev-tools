use crate::errors::{CliError, Result};
use crate::tool::Tool;
use std::ffi::OsString;

pub static TOOL: Tool = Tool {
    name: "decode",
    aliases: &[],
    description: "Decode base64, base64url, hex, atau URL ke teks/biner",
    category: "Encoding",
    run,
};

fn run(_args: Vec<OsString>) -> Result<()> {
    Err(CliError::Failed("belum diporting".into()))
}
