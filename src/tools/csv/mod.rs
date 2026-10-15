use crate::errors::{CliError, Result};
use crate::tool::Tool;
use std::ffi::OsString;

pub static TOOL: Tool = Tool {
    name: "csv",
    aliases: &[],
    description: "Baca CSV: tabel, JSON, TSV, pilih kolom, statistik",
    category: "Data",
    run,
};

fn run(_args: Vec<OsString>) -> Result<()> {
    Err(CliError::Failed("belum diporting".into()))
}
