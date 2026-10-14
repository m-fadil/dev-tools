export const BITS = { 4: 32, 6: 128 };
const OCTET = /^(0|[1-9]\d{0,2})$/;
const HEXTET = /^[0-9a-fA-F]{1,4}$/;
/** Strict dotted quad; leading zeros (010) are rejected as ambiguous (octal vs decimal). */
export function parseIPv4(text) {
    const parts = text.split('.');
    if (parts.length !== 4)
        throw new Error(`Alamat IPv4 tidak valid: ${text}`);
    let value = 0n;
    for (const part of parts) {
        if (!OCTET.test(part)) {
            const hint = /^0\d+$/.test(part) ? ' (angka nol di depan ambigu)' : '';
            throw new Error(`Oktet IPv4 tidak valid "${part}" pada ${text}${hint}`);
        }
        const n = Number(part);
        if (n > 255)
            throw new Error(`Oktet IPv4 di luar 0-255 "${part}" pada ${text}`);
        value = (value << 8n) | BigInt(n);
    }
    return value;
}
function parseGroups(text, whole) {
    if (text === '')
        return [];
    const groups = [];
    const parts = text.split(':');
    parts.forEach((part, i) => {
        if (i === parts.length - 1 && part.includes('.')) {
            const v4 = parseIPv4(part);
            groups.push(v4 >> 16n, v4 & 0xffffn);
        }
        else if (HEXTET.test(part)) {
            groups.push(BigInt(`0x${part}`));
        }
        else {
            throw new Error(`Alamat IPv6 tidak valid: ${whole}`);
        }
    });
    return groups;
}
/** Strict RFC 4291 text form: one optional "::", 1-4 hex digits per group, optional dotted-quad tail, no zone id. */
export function parseIPv6(text) {
    const fail = () => new Error(`Alamat IPv6 tidak valid: ${text}`);
    if (!text.includes(':') || /[^0-9a-fA-F:.]/.test(text))
        throw fail();
    const halves = text.split('::');
    if (halves.length > 2)
        throw fail();
    let groups;
    if (halves.length === 1) {
        groups = parseGroups(text, text);
        if (groups.length !== 8)
            throw fail();
    }
    else {
        const [head, tail] = halves;
        // The dotted tail is only legal at the very end of the address.
        if (head.includes('.'))
            throw fail();
        const left = parseGroups(head, text);
        const right = parseGroups(tail, text);
        const missing = 8 - left.length - right.length;
        if (missing < 1)
            throw fail();
        groups = [...left, ...Array(missing).fill(0n), ...right];
    }
    return groups.reduce((acc, g) => (acc << 16n) | g, 0n);
}
export function parseAddress(text) {
    if (text.includes(':'))
        return { version: 6, value: parseIPv6(text) };
    return { version: 4, value: parseIPv4(text) };
}
export function formatIPv4(value) {
    return [24n, 16n, 8n, 0n].map((shift) => String((value >> shift) & 0xffn)).join('.');
}
/** RFC 5952: lowercase, longest zero run (>= 2 groups, first on tie) compressed; IPv4-mapped keeps a dotted tail. */
export function formatIPv6(value) {
    if (value >> 32n === 0xffffn)
        return `::ffff:${formatIPv4(value & 0xffffffffn)}`;
    const groups = Array.from({ length: 8 }, (_, i) => Number((value >> BigInt((7 - i) * 16)) & 0xffffn));
    let bestStart = -1;
    let bestLength = 0;
    for (let i = 0; i < 8;) {
        if (groups[i] !== 0) {
            i++;
            continue;
        }
        let j = i;
        while (j < 8 && groups[j] === 0)
            j++;
        if (j - i > bestLength) {
            bestStart = i;
            bestLength = j - i;
        }
        i = j;
    }
    const hex = (g) => g.map((n) => n.toString(16)).join(':');
    if (bestLength < 2)
        return hex(groups);
    return `${hex(groups.slice(0, bestStart))}::${hex(groups.slice(bestStart + bestLength))}`;
}
export function formatAddress(version, value) {
    return version === 4 ? formatIPv4(value) : formatIPv6(value);
}
/** Network mask with `prefix` leading one bits. */
export function maskOf(version, prefix) {
    const bits = BITS[version];
    return ((1n << BigInt(bits)) - 1n) ^ ((1n << BigInt(bits - prefix)) - 1n);
}
/** Converts a contiguous dotted netmask to its prefix length. */
export function prefixFromNetmask(text) {
    const mask = parseIPv4(text);
    const inverted = mask ^ 0xffffffffn;
    if ((inverted & (inverted + 1n)) !== 0n)
        throw new Error(`Netmask tidak kontigu: ${text}`);
    return 32 - inverted.toString(2).replace(/^0$/, '').length;
}
export function makeNetwork(version, address, prefix) {
    const mask = maskOf(version, prefix);
    const network = address & mask;
    const last = network | (mask ^ ((1n << BigInt(BITS[version])) - 1n));
    return { version, address, prefix, network, last };
}
const PREFIX = /^(0|[1-9]\d{0,2})$/;
function parsePrefix(text, version) {
    const n = PREFIX.test(text) ? Number(text) : NaN;
    if (!(n >= 0 && n <= BITS[version])) {
        throw new Error(`Prefix tidak valid "${text}": harus bilangan bulat 0-${BITS[version]} untuk IPv${version}`);
    }
    return n;
}
/** Accepts ["ip/prefix"] or ["ip", "netmask"] (IPv4 only). */
export function parseNetwork(args) {
    if (args.length === 0 || args.length > 2)
        throw new Error('Beri satu argumen ip/prefix atau dua argumen ip netmask.');
    if (args.length === 2) {
        const [ip, mask] = args;
        if (ip.includes('/') || mask.includes('/'))
            throw new Error('Gunakan ip/prefix atau ip netmask, bukan campuran.');
        const address = parseAddress(ip);
        if (address.version !== 4)
            throw new Error('Bentuk "ip netmask" hanya untuk IPv4; gunakan ip/prefix.');
        return makeNetwork(4, address.value, prefixFromNetmask(mask));
    }
    const [text] = args;
    const slash = text.indexOf('/');
    if (slash < 0)
        throw new Error(`Prefix wajib: gunakan ${text}/<prefix> atau ${text} <netmask>`);
    const address = parseAddress(text.slice(0, slash));
    return makeNetwork(address.version, address.value, parsePrefix(text.slice(slash + 1), address.version));
}
//# sourceMappingURL=ip.js.map