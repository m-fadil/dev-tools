use crate::errors::{CliError, Result};
use crate::tool::Tool;
use std::ffi::OsString;

pub static TOOL: Tool = Tool {
    name: "fake",
    aliases: &[],
    description: "Buat data palsu (nama, email, alamat, IP, tanggal, dll.) untuk pengujian",
    category: "Utilitas",
    run,
};

fn run(_args: Vec<OsString>) -> Result<()> {
    Err(CliError::Failed("belum diporting".into()))
}
