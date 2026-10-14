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
] as const;
export const HASH_ENCODINGS = ['hex', 'base64', 'base64url'] as const;
export type HashAlgorithm = (typeof HASH_ALGORITHMS)[number];
export type HashEncoding = (typeof HASH_ENCODINGS)[number];

/** Digest of data; with hmacKey, an HMAC instead of a plain hash. */
export function hash(data: Buffer, algorithm: HashAlgorithm, encoding: HashEncoding, hmacKey?: string): string {
    const hasher = hmacKey === undefined ? createHash(algorithm) : createHmac(algorithm, hmacKey);
    return hasher.update(data).digest(encoding);
}
