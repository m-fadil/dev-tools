import { UsageError } from '../../errors.js';
export const FORMATS = ['colon', 'hyphen', 'dot', 'plain'];
const HEX = '[0-9a-fA-F]';
const PATTERNS = [
    new RegExp(`^(${HEX}{2})(?::(${HEX}{2})){5}$`),
    new RegExp(`^(${HEX}{2})(?:-(${HEX}{2})){5}$`),
    new RegExp(`^${HEX}{4}\\.${HEX}{4}\\.${HEX}{4}$`),
    new RegExp(`^${HEX}{12}$`),
];
/** Parses EUI-48 (colon, hyphen, Cisco dot or plain, surrounding whitespace ignored) into 12 lowercase hex digits. */
export function parseMac(input) {
    const text = input.trim();
    if (!PATTERNS.some((p) => p.test(text))) {
        throw new UsageError(`Alamat MAC tidak valid: ${text} (bentuk: aa:bb:cc:dd:ee:ff, aa-bb-cc-dd-ee-ff, aabb.ccdd.eeff, aabbccddeeff)`);
    }
    return text.replace(/[:.-]/g, '').toLowerCase();
}
export function formatMac(hex, format, upper) {
    const out = {
        colon: hex.match(/../g).join(':'),
        hyphen: hex.match(/../g).join('-'),
        dot: hex.match(/..../g).join('.'),
        plain: hex,
    }[format];
    return upper ? out.toUpperCase() : out;
}
/** I/G bit (LSB of first octet) and U/L bit (second LSB). */
export function macFlags(hex) {
    const first = parseInt(hex.slice(0, 2), 16);
    return { multicast: (first & 1) === 1, locallyAdministered: (first & 2) === 2 };
}
//# sourceMappingURL=mac.js.map