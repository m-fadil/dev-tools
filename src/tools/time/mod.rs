use crate::errors::{CliError, Result};
use crate::tool::Tool;
use std::ffi::OsString;

pub static TOOL: Tool = Tool {
    name: "time",
    aliases: &["ts"],
    description: "Konversi epoch/ISO-8601 ke UTC, zona waktu, dan waktu relatif",
    category: "Utilitas",
    run,
};

fn run(_args: Vec<OsString>) -> Result<()> {
    Err(CliError::Failed("belum diporting".into()))
}
