import { crc32 } from 'node:zlib';
import type { HashEncoding } from './hash.js';

/** CRC-32 (IEEE 802.3) of data; base64 variants encode the 4 big-endian bytes. */
export function crc32Digest(data: Buffer, encoding: HashEncoding): string {
    const bytes = Buffer.alloc(4);
    bytes.writeUInt32BE(crc32(data), 0);
    return bytes.toString(encoding);
}
