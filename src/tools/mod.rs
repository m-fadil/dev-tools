//! Tool registry: shared by the menu, help, and command dispatch.

use crate::tool::Tool;

pub mod ascii;
pub mod bytes;
pub mod case;
pub mod checksum;
pub mod cidr;
pub mod color;
pub mod completion;
pub mod cron;
pub mod csv;
pub mod decode;
pub mod diff;
pub mod encode;
pub mod encrypt;
pub mod env;
pub mod fake;
pub mod hash;
pub mod image;
pub mod json;
pub mod jwt;
pub mod lorem;
pub mod mac;
pub mod math;
pub mod number;
pub mod password;
pub mod port;
pub mod random;
pub mod regex;
pub mod semver;
pub mod syncgit;
pub mod text;
pub mod time;
pub mod url;

/// Registration order matches the legacy registry.
pub static TOOLS: &[&Tool] = &[
    &syncgit::TOOL,
    &hash::TOOL,
    &checksum::TOOL,
    &encrypt::TOOL,
    &encode::TOOL,
    &decode::TOOL,
    &random::TOOL,
    &password::TOOL,
    &jwt::TOOL,
    &json::TOOL,
    &time::TOOL,
    &cron::TOOL,
    &url::TOOL,
    &diff::TOOL,
    &env::TOOL,
    &port::TOOL,
    &ascii::TOOL,
    &bytes::TOOL,
    &case::TOOL,
    &cidr::TOOL,
    &color::TOOL,
    &completion::TOOL,
    &csv::TOOL,
    &fake::TOOL,
    &image::TOOL,
    &lorem::TOOL,
    &mac::TOOL,
    &math::TOOL,
    &number::TOOL,
    &regex::TOOL,
    &semver::TOOL,
    &text::TOOL,
];

/// Resolves a command name or alias.
pub fn find_tool(name: &str) -> Option<&'static Tool> {
    TOOLS
        .iter()
        .copied()
        .find(|t| t.name == name || t.aliases.contains(&name))
}

/// Tools grouped by category, categories in first-appearance order, tools sorted by name.
pub fn tools_by_category() -> Vec<(&'static str, Vec<&'static Tool>)> {
    let mut groups: Vec<(&'static str, Vec<&'static Tool>)> = Vec::new();
    for tool in TOOLS {
        match groups.iter_mut().find(|(c, _)| *c == tool.category) {
            Some((_, list)) => list.push(tool),
            None => groups.push((tool.category, vec![tool])),
        }
    }
    for (_, list) in &mut groups {
        list.sort_by(|a, b| a.name.cmp(b.name));
    }
    groups
}
