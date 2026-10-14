import { crc32 } from 'node:zlib';
/** CRC-32 (IEEE 802.3) of data; base64 variants encode the 4 big-endian bytes. */
export function crc32Digest(data, encoding) {
    const bytes = Buffer.alloc(4);
    bytes.writeUInt32BE(crc32(data), 0);
    return bytes.toString(encoding);
}
//# sourceMappingURL=crc32.js.map