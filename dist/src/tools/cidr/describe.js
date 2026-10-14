import { formatAddress, BITS, maskOf } from './ip.js';
/** True when the whole range [network, last] of a /prefix network lies inside the block base/blockPrefix. */
const within = (network, prefix, base, blockPrefix) => prefix >= blockPrefix && network >> BigInt(32 - blockPrefix) === base >> BigInt(32 - blockPrefix);
export function describe(net) {
    const { version, prefix } = net;
    const fmt = (v) => formatAddress(version, v);
    const total = 1n << BigInt(BITS[version] - prefix);
    const base = {
        version,
        address: fmt(net.address),
        network: fmt(net.network),
        prefix,
        cidr: `${fmt(net.network)}/${prefix}`,
    };
    if (version === 6) {
        return { ...base, first: fmt(net.network), last: fmt(net.last), totalAddresses: total.toString() };
    }
    // RFC 3021: /31 has two usable hosts, /32 a single one; otherwise network and broadcast are excluded.
    const usable = prefix === 32 ? 1n : prefix === 31 ? 2n : total - 2n;
    const [firstHost, lastHost] = prefix >= 31 ? [net.network, net.last] : [net.network + 1n, net.last - 1n];
    const mask = maskOf(4, prefix);
    const { network } = net;
    return {
        ...base,
        netmask: fmt(mask),
        wildcard: fmt(mask ^ 0xffffffffn),
        broadcast: fmt(net.last),
        firstHost: fmt(firstHost),
        lastHost: fmt(lastHost),
        totalAddresses: total.toString(),
        usableHosts: usable.toString(),
        private: within(network, prefix, 0x0a000000n, 8) ||
            within(network, prefix, 0xac100000n, 12) ||
            within(network, prefix, 0xc0a80000n, 16),
        loopback: within(network, prefix, 0x7f000000n, 8),
        linkLocal: within(network, prefix, 0xa9fe0000n, 16),
    };
}
export function render(d) {
    const rows = [
        ['Alamat', d.address],
        ['Jaringan', d.cidr],
        ['Prefix', `/${d.prefix}`],
        ['Netmask', d.netmask],
        ['Wildcard', d.wildcard],
        ['Broadcast', d.broadcast],
        ['Host pertama', d.firstHost],
        ['Host terakhir', d.lastHost],
        ['Alamat pertama', d.first],
        ['Alamat terakhir', d.last],
        ['Total alamat', d.totalAddresses],
        ['Host usable', d.usableHosts],
    ];
    const flags = [d.private && 'private (RFC 1918)', d.loopback && 'loopback', d.linkLocal && 'link-local'].filter(Boolean);
    if (flags.length)
        rows.push(['Seluruh subnet', flags.join(', ')]);
    return rows
        .filter((r) => r[1] !== undefined)
        .map(([k, v]) => `${k.padEnd(15)} ${v}`)
        .join('\n');
}
//# sourceMappingURL=describe.js.map