use crate::errors::{CliError, Result};
use crate::tool::Tool;
use std::ffi::OsString;

pub static TOOL: Tool = Tool {
    name: "regex",
    aliases: &["re"],
    description: "Uji regex JavaScript: daftar kecocokan, grup, dan penggantian",
    category: "Data",
    run,
};

fn run(_args: Vec<OsString>) -> Result<()> {
    Err(CliError::Failed("belum diporting".into()))
}
