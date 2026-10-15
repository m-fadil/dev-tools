import { randomBytes } from 'node:crypto';
export const UUID_VERSIONS = ['1', '4', '7'];
/** Milliseconds between the Gregorian epoch (1582-10-15) and the Unix epoch. */
const GREGORIAN_OFFSET_MS = 12219292800000n;
function format(bytes, version) {
    bytes.writeUInt8((bytes.readUInt8(6) & 0x0f) | (version << 4), 6);
    bytes.writeUInt8((bytes.readUInt8(8) & 0x3f) | 0x80, 8);
    const hex = bytes.toString('hex');
    return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20)}`;
}
/**
 * UUID v7 (RFC 9562 5.7): 48-bit unix ms, then a 12-bit rand_a used as a counter (method 1, RFC 9562 6.2) and
 * 62 random bits. Output is strictly increasing; on counter overflow or a backwards clock the timestamp is bumped.
 */
export function createV7Generator(now = Date.now) {
    let lastMs = -1;
    let counter = 0;
    // Seed with the top bit clear so a same-millisecond burst has headroom before overflowing.
    const seed = () => randomBytes(2).readUInt16BE(0) & 0x7ff;
    return () => {
        const ms = now();
        if (ms > lastMs) {
            lastMs = ms;
            counter = seed();
        }
        else if (++counter > 0xfff) {
            lastMs++;
            counter = seed();
        }
        const bytes = randomBytes(16);
        bytes.writeUIntBE(lastMs, 0, 6);
        bytes.writeUInt16BE(counter, 6);
        return format(bytes, 7);
    };
}
/**
 * UUID v1 (RFC 9562 5.1): 60-bit count of 100ns ticks since 1582-10-15, random 14-bit clock sequence, and a random
 * 48-bit node id with the multicast bit set (RFC 9562 6.10); no real MAC address is ever read. Timestamps are
 * strictly increasing within a generator, so values never repeat.
 */
export function createV1Generator(now = Date.now) {
    const node = randomBytes(6);
    node.writeUInt8(node.readUInt8(0) | 0x01, 0);
    const clockSeq = randomBytes(2).readUInt16BE(0) & 0x3fff;
    let last = -1n;
    return () => {
        let ticks = (BigInt(now()) + GREGORIAN_OFFSET_MS) * 10000n;
        if (ticks <= last)
            ticks = last + 1n;
        last = ticks;
        const bytes = Buffer.alloc(16);
        bytes.writeUInt32BE(Number(ticks & 0xffffffffn), 0);
        bytes.writeUInt16BE(Number((ticks >> 32n) & 0xffffn), 4);
        bytes.writeUInt16BE(Number((ticks >> 48n) & 0x0fffn), 6);
        bytes.writeUInt16BE(clockSeq, 8);
        node.copy(bytes, 10);
        return format(bytes, 1);
    };
}
//# sourceMappingURL=version.js.map