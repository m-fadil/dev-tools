use crate::errors::{CliError, Result};
use crate::tool::Tool;
use std::ffi::OsString;

pub static TOOL: Tool = Tool {
    name: "random",
    aliases: &[],
    description: "Buat UUID atau token acak melalui subcommand uuid/token",
    category: "Utilitas",
    run,
};

fn run(_args: Vec<OsString>) -> Result<()> {
    Err(CliError::Failed("belum diporting".into()))
}
