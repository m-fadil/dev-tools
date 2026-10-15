use crate::errors::{CliError, Result};
use crate::tool::Tool;
use std::ffi::OsString;

pub static TOOL: Tool = Tool {
    name: "hash",
    aliases: &[],
    description: "Hash atau HMAC teks/file (md5, sha1, sha2, sha3, blake2b, crc32)",
    category: "Keamanan & Kripto",
    run,
};

fn run(_args: Vec<OsString>) -> Result<()> {
    Err(CliError::Failed("belum diporting".into()))
}
