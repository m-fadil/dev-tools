use crate::errors::{CliError, Result};
use crate::tool::Tool;
use std::ffi::OsString;

pub static TOOL: Tool = Tool {
    name: "mac",
    aliases: &[],
    description: "Parse dan konversi alamat MAC (colon, hyphen, dot, plain), flag, dan alamat acak",
    category: "Jaringan",
    run,
};

fn run(_args: Vec<OsString>) -> Result<()> {
    Err(CliError::Failed("belum diporting".into()))
}
