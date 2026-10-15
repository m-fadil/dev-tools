use crate::errors::{CliError, Result};
use crate::tool::Tool;
use std::ffi::OsString;

pub static TOOL: Tool = Tool {
    name: "password",
    aliases: &["pw"],
    description: "Buat password acak yang aman secara kriptografis",
    category: "Keamanan & Kripto",
    run,
};

fn run(_args: Vec<OsString>) -> Result<()> {
    Err(CliError::Failed("belum diporting".into()))
}
