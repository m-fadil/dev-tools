mod range;
pub(crate) mod version;

use crate::errors::{CliError, Result, usage};
use crate::io::{parse, print_json};
use crate::tool::Tool;
use clap::Parser;
use serde_json::{Value, json};
use std::ffi::OsString;

use range::{parse_range, satisfies};
use version::{
    RELEASES, Release, Version, bump, check_preid, compare, format, is_numeric, ordering_sign, parse_version,
    to_number, try_parse,
};

pub static TOOL: Tool = Tool {
    name: "semver",
    aliases: &["sv"],
    description: "Parse, bandingkan, urutkan, naikkan, dan cocokkan versi SemVer dengan range",
    category: "Data",
    run,
};

const HELP: &str = r##"dt semver <perintah> [opsi] <argumen...>   (alias: sv)

Operasi Semantic Versioning 2.0.0 (SemVer ketat: tanpa nol di depan pada angka).
Satu awalan "v" diterima dan dibuang (v1.2.3 = 1.2.3).
  parse <v>               Tampilkan major, minor, patch, prerelease, build
  valid <v>               Cetak valid/invalid; exit 0 jika valid, 1 jika tidak
  compare <a> <b>         Cetak -1, 0, atau 1 (metadata build diabaikan)
  sort <v...>             Urutkan naik
      --reverse           Urutkan turun
  bump <rilis> <v>        Naikkan versi; rilis: major | minor | patch | premajor | preminor | prepatch | prerelease
      --preid <id>        Tag prerelease untuk pre* (mis. beta -> 1.0.1-beta.0)
  satisfies <v> <range>   Cetak true/false; exit 0 jika cocok, 1 jika tidak
      --json              Keluaran JSON (satu dokumen) untuk semua perintah
  -h, --help              Tampilkan bantuan

Range (gaya npm): >= <= > < =, ^1.2.3, ~1.2.3, 1.2.x, 1.*, 1.2.3 - 2.3.4,
spasi = AND, || = OR. Versi prerelease hanya cocok dengan himpunan pembanding
yang memuat prerelease pada [major,minor,patch] yang sama.
Bump mengikuti npm: pre* pada versi non-prerelease menaikkan patch dulu;
prerelease menaikkan identifier numerik terakhir atau menambah .0. Metadata build dibuang.
Exit: 0 sukses, 1 valid/satisfies bernilai salah atau bump melebihi bilangan bulat aman;
2 penggunaan salah (termasuk versi tidak valid).
Contoh: dt semver bump minor 1.2.3   |   dt semver satisfies 1.4.0 "^1.2 || >=3""##;

#[derive(Parser)]
#[command(name = "dt semver", override_help = HELP, disable_version_flag = true, args_override_self = true)]
struct Args {
    #[arg(long)]
    reverse: bool,
    #[arg(long)]
    preid: Option<String>,
    #[arg(long)]
    json: bool,
    positionals: Vec<String>,
}

fn exact(positionals: &[String], count: usize, usage_text: &str) -> Result<()> {
    if positionals.len() != count {
        return Err(usage(format!("Penggunaan: dt semver {usage_text}")));
    }
    Ok(())
}

/// Numeric identifiers become numbers when they are exactly representable.
fn ident(id: &str) -> Value {
    if is_numeric(id)
        && let Ok(n) = to_number(id, "")
    {
        return json!(n);
    }
    json!(id)
}

fn run(args: Vec<OsString>) -> Result<()> {
    let a: Args = parse("semver", args)?;
    let Some((command, rest)) = a.positionals.split_first() else {
        return Err(usage(
            "Perintah wajib diisi: parse, valid, compare, sort, bump, satisfies.",
        ));
    };
    let command = command.as_str();
    let json_out = a.json;
    if a.reverse && command != "sort" {
        return Err(usage("--reverse hanya untuk perintah sort."));
    }
    if a.preid.is_some() && command != "bump" {
        return Err(usage("--preid hanya untuk perintah bump."));
    }

    match command {
        "parse" => {
            exact(rest, 1, "parse <versi>")?;
            let v = parse_version(&rest[0])?;
            if json_out {
                print_json(&json!({
                    "input": rest[0],
                    "version": format(&v, true),
                    "major": v.major,
                    "minor": v.minor,
                    "patch": v.patch,
                    "prerelease": v.prerelease.iter().map(|id| ident(id)).collect::<Vec<_>>(),
                    "build": v.build,
                }));
            } else {
                let dash = |ids: &[String]| if ids.is_empty() { "-".to_string() } else { ids.join(".") };
                println!("major:      {}", v.major);
                println!("minor:      {}", v.minor);
                println!("patch:      {}", v.patch);
                println!("prerelease: {}", dash(&v.prerelease));
                println!("build:      {}", dash(&v.build));
            }
            Ok(())
        }
        "valid" => {
            exact(rest, 1, "valid <versi>")?;
            let r = try_parse(&rest[0]);
            let ok = r.is_ok();
            if json_out {
                print_json(&json!({
                    "input": rest[0],
                    "valid": ok,
                    "version": r.as_ref().ok().map(|v| format(v, true)),
                    "error": r.as_ref().err(),
                }));
            } else {
                println!("{}", if ok { "valid" } else { "invalid" });
            }
            if ok { Ok(()) } else { Err(CliError::Exit(1)) }
        }
        "compare" => {
            exact(rest, 2, "compare <a> <b>")?;
            let result = ordering_sign(compare(&parse_version(&rest[0])?, &parse_version(&rest[1])?));
            if json_out {
                print_json(&json!({ "a": rest[0], "b": rest[1], "result": result }));
            } else {
                println!("{result}");
            }
            Ok(())
        }
        "sort" => {
            if rest.is_empty() {
                return Err(usage("Penggunaan: dt semver sort [--reverse] <versi...>"));
            }
            let mut parsed: Vec<Version> = rest.iter().map(|s| parse_version(s)).collect::<Result<_>>()?;
            // Stable sort: versions that differ only in build keep input order.
            parsed.sort_by(|x, y| if a.reverse { compare(y, x) } else { compare(x, y) });
            let versions: Vec<String> = parsed.iter().map(|v| format(v, true)).collect();
            if json_out {
                print_json(&json!({ "reverse": a.reverse, "versions": versions }));
            } else {
                println!("{}", versions.join("\n"));
            }
            Ok(())
        }
        "bump" => {
            exact(rest, 2, "bump <rilis> <versi>")?;
            let name = rest[0].as_str();
            let Some(release) = Release::from_name(name) else {
                return Err(usage(format!(
                    "Jenis rilis tidak valid: {name}. Pilihan: {}",
                    RELEASES.join(", ")
                )));
            };
            let preid = a.preid.as_deref();
            if let Some(p) = preid {
                check_preid(p)?;
                if !name.starts_with("pre") {
                    return Err(usage("--preid hanya berlaku untuk rilis pre*."));
                }
            }
            let from = parse_version(&rest[1])?;
            let result = format(&bump(&from, release, preid)?, true);
            if json_out {
                print_json(&json!({ "release": name, "preid": preid, "from": format(&from, true), "version": result }));
            } else {
                println!("{result}");
            }
            Ok(())
        }
        "satisfies" => {
            exact(rest, 2, "satisfies <versi> <range>")?;
            let v = parse_version(&rest[0])?;
            let ok = satisfies(&v, &parse_range(&rest[1])?);
            if json_out {
                print_json(&json!({ "version": format(&v, true), "range": rest[1], "satisfies": ok }));
            } else {
                println!("{ok}");
            }
            if ok { Ok(()) } else { Err(CliError::Exit(1)) }
        }
        _ => Err(usage(format!(
            "Perintah tidak dikenal: {command}. Pilihan: parse, valid, compare, sort, bump, satisfies."
        ))),
    }
}
