use crate::errors::{CliError, Result};
use crate::tool::Tool;
use std::ffi::OsString;

pub static TOOL: Tool = Tool {
    name: "cidr",
    aliases: &[],
    description: "Hitung subnet IPv4/IPv6: netmask, broadcast, rentang host, contains, split",
    category: "Jaringan",
    run,
};

fn run(_args: Vec<OsString>) -> Result<()> {
    Err(CliError::Failed("belum diporting".into()))
}
