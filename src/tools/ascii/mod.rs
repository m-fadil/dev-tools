use crate::errors::{CliError, Result};
use crate::tool::Tool;
use std::ffi::OsString;

pub static TOOL: Tool = Tool {
    name: "ascii",
    aliases: &[],
    description: "Tabel ASCII dan info karakter (desimal, hex, oktal, biner, Unicode)",
    category: "Utilitas",
    run,
};

fn run(_args: Vec<OsString>) -> Result<()> {
    Err(CliError::Failed("belum diporting".into()))
}
