use crate::errors::{CliError, Result};
use crate::tool::Tool;
use std::ffi::OsString;

pub static TOOL: Tool = Tool {
    name: "semver",
    aliases: &["sv"],
    description: "Parse, bandingkan, urutkan, naikkan, dan cocokkan versi SemVer dengan range",
    category: "Data",
    run,
};

fn run(_args: Vec<OsString>) -> Result<()> {
    Err(CliError::Failed("belum diporting".into()))
}
