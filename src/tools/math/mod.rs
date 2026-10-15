use crate::errors::{CliError, Result};
use crate::tool::Tool;
use std::ffi::OsString;

pub static TOOL: Tool = Tool {
    name: "math",
    aliases: &[],
    description: "Hitung ekspresi matematika (operator, fungsi, konstanta)",
    category: "Matematika",
    run,
};

fn run(_args: Vec<OsString>) -> Result<()> {
    Err(CliError::Failed("belum diporting".into()))
}
