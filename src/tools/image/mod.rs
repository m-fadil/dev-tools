use crate::errors::{CliError, Result};
use crate::tool::Tool;
use std::ffi::OsString;

pub static TOOL: Tool = Tool {
    name: "image",
    aliases: &[],
    description: "Metadata gambar (PNG, JPEG, GIF, WebP): ukuran, rasio, EXIF, frame",
    category: "Media",
    run,
};

fn run(_args: Vec<OsString>) -> Result<()> {
    Err(CliError::Failed("belum diporting".into()))
}
