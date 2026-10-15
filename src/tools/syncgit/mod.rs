use crate::errors::{CliError, Result};
use crate::tool::Tool;
use std::ffi::OsString;

pub static TOOL: Tool = Tool {
    name: "syncgit",
    aliases: &[],
    description: "Sinkron commit dan tag antar remote Git dengan pemetaan mailmap",
    category: "Git",
    run,
};

fn run(_args: Vec<OsString>) -> Result<()> {
    Err(CliError::Failed("belum diporting".into()))
}
