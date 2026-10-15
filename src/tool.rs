use std::ffi::OsString;

pub struct Tool {
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub description: &'static str,
    pub category: &'static str,
    pub run: fn(Vec<OsString>) -> crate::errors::Result<()>,
}
