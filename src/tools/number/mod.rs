use crate::errors::{CliError, Result};
use crate::tool::Tool;
use std::ffi::OsString;

pub static TOOL: Tool = Tool {
    name: "number",
    aliases: &["num"],
    description: "Ubah bilangan antar basis 2/8/10/16 dengan ukuran bebas",
    category: "Matematika",
    run,
};

fn run(_args: Vec<OsString>) -> Result<()> {
    Err(CliError::Failed("belum diporting".into()))
}
