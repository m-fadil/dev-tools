use crate::errors::{CliError, Result};
use crate::tool::Tool;
use std::ffi::OsString;

pub static TOOL: Tool = Tool {
    name: "cron",
    aliases: &[],
    description: "Jelaskan ekspresi cron dan tampilkan waktu eksekusi berikutnya",
    category: "Utilitas",
    run,
};

fn run(_args: Vec<OsString>) -> Result<()> {
    Err(CliError::Failed("belum diporting".into()))
}
