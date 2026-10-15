use crate::errors::{CliError, Result};
use crate::tool::Tool;
use std::ffi::OsString;

pub static TOOL: Tool = Tool {
    name: "port",
    aliases: &[],
    description: "Periksa port TCP terbuka atau cari port kosong",
    category: "Jaringan",
    run,
};

fn run(_args: Vec<OsString>) -> Result<()> {
    Err(CliError::Failed("belum diporting".into()))
}
