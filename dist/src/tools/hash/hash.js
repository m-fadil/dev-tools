import { createHash, createHmac } from 'node:crypto';
export const HASH_ALGORITHMS = [
    'md5',
    'sha1',
    'sha224',
    'sha256',
    'sha384',
    'sha512',
    'sha3-256',
    'sha3-512',
    'blake2b512',
];
export const HASH_ENCODINGS = ['hex', 'base64', 'base64url'];
/** Digest of data; with hmacKey, an HMAC instead of a plain hash. */
export function hash(data, algorithm, encoding, hmacKey) {
    const hasher = hmacKey === undefined ? createHash(algorithm) : createHmac(algorithm, hmacKey);
    return hasher.update(data).digest(encoding);
}
//# sourceMappingURL=hash.js.map