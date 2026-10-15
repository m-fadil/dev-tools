mod describe;
mod ip;

use crate::errors::{Result, usage};
use crate::io::{integer, parse, print_json};
use crate::tool::Tool;
use clap::Parser;
use serde_json::json;
use std::ffi::OsString;

use describe::{describe, pow2_string, render, to_json};
use ip::{format_address, make_network, parse_address, parse_network};

pub static TOOL: Tool = Tool {
    name: "cidr",
    aliases: &[],
    description: "Hitung subnet IPv4/IPv6: netmask, broadcast, rentang host, contains, split",
    category: "Jaringan",
    run,
};

const MAX_SPLIT: u32 = 4096;

const HELP: &str = r##"dt cidr [options] <ip/prefix | ip netmask>

Hitung detail subnet IPv4 dan IPv6 (aritmetika BigInt, parsing ketat).
  --contains <ip>     Cek apakah ip ada di subnet: cetak yes/no, exit 0 (ya) / 1 (tidak)
  --split <prefix>    Daftar subnet dengan panjang prefix tersebut (maks 4096 subnet)
      --json          Keluaran JSON
  -h, --help          Tampilkan bantuan

Bentuk "ip netmask" (mis. 10.0.0.0 255.255.255.0) hanya untuk IPv4. Prefix wajib.
Oktet berawalan nol (010) ditolak karena ambigu. Versi IP berbeda dianggap bukan anggota.
Exit: 0 sukses, 2 argumen/alamat tidak valid; dengan --contains, 1 berarti "no".
Contoh: dt cidr 192.168.1.10/24   |   dt cidr --split 26 10.0.0.0/24
        dt cidr --contains 2001:db8::1 2001:db8::/32"##;

#[derive(Parser)]
#[command(name = "dt cidr", override_help = HELP, disable_version_flag = true, args_override_self = true)]
struct Args {
    #[arg(long)]
    contains: Option<String>,
    #[arg(long)]
    split: Option<String>,
    #[arg(long)]
    json: bool,
    positionals: Vec<String>,
}

fn run(args: Vec<OsString>) -> Result<()> {
    let a: Args = parse("cidr", args)?;
    if a.contains.is_some() && a.split.is_some() {
        return Err(usage("--contains dan --split tidak bisa digabung."));
    }
    let net = parse_network(&a.positionals).map_err(usage)?;
    let net_text = format!("{}/{}", format_address(net.version, net.network), net.prefix);

    if let Some(text) = &a.contains {
        let ip = parse_address(text).map_err(usage)?;
        let yes = ip.version == net.version && ip.value >= net.network && ip.value <= net.last;
        if a.json {
            print_json(&json!({
                "network": net_text,
                "address": format_address(ip.version, ip.value),
                "contains": yes,
            }));
        } else {
            println!("{}", if yes { "yes" } else { "no" });
        }
        return if yes {
            Ok(())
        } else {
            Err(crate::errors::CliError::Exit(1))
        };
    }

    if let Some(split) = &a.split {
        let bits = net.version.bits();
        let target = integer(Some(split), 0, 0, i64::from(bits), "split")? as u32;
        if target < net.prefix {
            return Err(usage(format!(
                "Prefix --split ({target}) tidak boleh lebih pendek dari /{}.",
                net.prefix
            )));
        }
        let diff = target - net.prefix;
        if diff > 12 {
            return Err(usage(format!(
                "Hasil {} subnet melebihi batas {MAX_SPLIT}; pilih prefix lebih pendek.",
                pow2_string(diff)
            )));
        }
        let count = 1u32 << diff;
        if count > MAX_SPLIT {
            return Err(usage(format!(
                "Hasil {count} subnet melebihi batas {MAX_SPLIT}; pilih prefix lebih pendek."
            )));
        }
        let size = if bits - target >= 128 {
            0
        } else {
            1u128 << (bits - target)
        };
        let subnets: Vec<String> = (0..count)
            .map(|i| {
                let addr = net.network.wrapping_add(u128::from(i).wrapping_mul(size));
                format!("{}/{target}", format_address(net.version, addr))
            })
            .collect();
        if a.json {
            print_json(&json!({
                "network": net_text,
                "prefix": target,
                "count": subnets.len(),
                "subnets": subnets,
            }));
        } else {
            println!("{}", subnets.join("\n"));
        }
        return Ok(());
    }

    let description = describe(&make_network(net.version, net.address, net.prefix));
    if a.json {
        print_json(&to_json(&description));
    } else {
        println!("{}", render(&description));
    }
    Ok(())
}
