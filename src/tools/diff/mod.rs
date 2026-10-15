use crate::errors::{CliError, Result};
use crate::tool::Tool;
use std::ffi::OsString;

pub static TOOL: Tool = Tool {
    name: "diff",
    aliases: &[],
    description: "Bandingkan dua file (sha256, diff baris, atau offset byte beda)",
    category: "Data",
    run,
};

fn run(_args: Vec<OsString>) -> Result<()> {
    Err(CliError::Failed("belum diporting".into()))
}
