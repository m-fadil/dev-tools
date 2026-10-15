use super::ip::{Network, Version, format_address, mask_of};
use serde_json::{Map, Value, json};

pub struct Description {
    pub version: Version,
    pub address: String,
    pub network: String,
    pub prefix: u32,
    pub cidr: String,
    pub netmask: Option<String>,
    pub wildcard: Option<String>,
    pub broadcast: Option<String>,
    pub first_host: Option<String>,
    pub last_host: Option<String>,
    pub first: Option<String>,
    pub last: Option<String>,
    pub total_addresses: String,
    pub usable_hosts: Option<String>,
    pub private: Option<bool>,
    pub loopback: Option<bool>,
    pub link_local: Option<bool>,
}

/// Decimal text of 2^n for n in 0..=128 (2^128 does not fit in u128).
pub fn pow2_string(n: u32) -> String {
    if n >= 128 {
        "340282366920938463463374607431768211456".to_string()
    } else {
        (1u128 << n).to_string()
    }
}

/// True when the whole range of a /prefix network lies inside the block base/blockPrefix.
fn within(network: u128, prefix: u32, base: u128, block_prefix: u32) -> bool {
    prefix >= block_prefix && network >> (32 - block_prefix) == base >> (32 - block_prefix)
}

pub fn describe(net: &Network) -> Description {
    let Network { version, prefix, .. } = *net;
    let fmt = |v: u128| format_address(version, v);
    let total = pow2_string(version.bits() - prefix);
    let mut d = Description {
        version,
        address: fmt(net.address),
        network: fmt(net.network),
        prefix,
        cidr: format!("{}/{}", fmt(net.network), prefix),
        netmask: None,
        wildcard: None,
        broadcast: None,
        first_host: None,
        last_host: None,
        first: None,
        last: None,
        total_addresses: total,
        usable_hosts: None,
        private: None,
        loopback: None,
        link_local: None,
    };
    if version == Version::V6 {
        d.first = Some(fmt(net.network));
        d.last = Some(fmt(net.last));
        return d;
    }
    // RFC 3021: /31 has two usable hosts, /32 a single one; otherwise network and broadcast are excluded.
    let total_n = 1u128 << (32 - prefix);
    let usable = match prefix {
        32 => 1,
        31 => 2,
        _ => total_n - 2,
    };
    let (first_host, last_host) = if prefix >= 31 {
        (net.network, net.last)
    } else {
        (net.network + 1, net.last - 1)
    };
    let mask = mask_of(Version::V4, prefix);
    let network = net.network;
    d.netmask = Some(fmt(mask));
    d.wildcard = Some(fmt(mask ^ 0xffff_ffff));
    d.broadcast = Some(fmt(net.last));
    d.first_host = Some(fmt(first_host));
    d.last_host = Some(fmt(last_host));
    d.usable_hosts = Some(usable.to_string());
    d.private = Some(
        within(network, prefix, 0x0a00_0000, 8)
            || within(network, prefix, 0xac10_0000, 12)
            || within(network, prefix, 0xc0a8_0000, 16),
    );
    d.loopback = Some(within(network, prefix, 0x7f00_0000, 8));
    d.link_local = Some(within(network, prefix, 0xa9fe_0000, 16));
    d
}

/// JSON document with the legacy key order; absent fields are omitted.
pub fn to_json(d: &Description) -> Value {
    let mut m = Map::new();
    m.insert("version".into(), json!(d.version.number()));
    m.insert("address".into(), json!(d.address));
    m.insert("network".into(), json!(d.network));
    m.insert("prefix".into(), json!(d.prefix));
    m.insert("cidr".into(), json!(d.cidr));
    let mut put = |k: &str, v: Option<Value>| {
        if let Some(v) = v {
            m.insert(k.into(), v);
        }
    };
    if d.version == Version::V6 {
        put("first", d.first.as_ref().map(|s| json!(s)));
        put("last", d.last.as_ref().map(|s| json!(s)));
        put("totalAddresses", Some(json!(d.total_addresses)));
    } else {
        put("netmask", d.netmask.as_ref().map(|s| json!(s)));
        put("wildcard", d.wildcard.as_ref().map(|s| json!(s)));
        put("broadcast", d.broadcast.as_ref().map(|s| json!(s)));
        put("firstHost", d.first_host.as_ref().map(|s| json!(s)));
        put("lastHost", d.last_host.as_ref().map(|s| json!(s)));
        put("totalAddresses", Some(json!(d.total_addresses)));
        put("usableHosts", d.usable_hosts.as_ref().map(|s| json!(s)));
        put("private", d.private.map(|b| json!(b)));
        put("loopback", d.loopback.map(|b| json!(b)));
        put("linkLocal", d.link_local.map(|b| json!(b)));
    }
    Value::Object(m)
}

pub fn render(d: &Description) -> String {
    let prefix = format!("/{}", d.prefix);
    let rows: [(&str, Option<&str>); 12] = [
        ("Alamat", Some(&d.address)),
        ("Jaringan", Some(&d.cidr)),
        ("Prefix", Some(&prefix)),
        ("Netmask", d.netmask.as_deref()),
        ("Wildcard", d.wildcard.as_deref()),
        ("Broadcast", d.broadcast.as_deref()),
        ("Host pertama", d.first_host.as_deref()),
        ("Host terakhir", d.last_host.as_deref()),
        ("Alamat pertama", d.first.as_deref()),
        ("Alamat terakhir", d.last.as_deref()),
        ("Total alamat", Some(&d.total_addresses)),
        ("Host usable", d.usable_hosts.as_deref()),
    ];
    let mut lines: Vec<String> = rows
        .iter()
        .filter_map(|(k, v)| v.map(|v| format!("{k:<15} {v}")))
        .collect();
    let mut flags = Vec::new();
    if d.private == Some(true) {
        flags.push("private (RFC 1918)");
    }
    if d.loopback == Some(true) {
        flags.push("loopback");
    }
    if d.link_local == Some(true) {
        flags.push("link-local");
    }
    if !flags.is_empty() {
        lines.push(format!("{:<15} {}", "Seluruh subnet", flags.join(", ")));
    }
    lines.join("\n")
}
