import { randomBytes, randomInt } from 'node:crypto';

export const TOKEN_FORMATS = ['hex', 'base64', 'base64url', 'alnum'] as const;
export type TokenFormat = (typeof TOKEN_FORMATS)[number];

const ALNUM = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';

/** Exactly `length` characters. alnum draws each char uniformly (randomInt rejects modulo bias). */
export function tokenByLength(format: TokenFormat, length: number): string {
    if (format === 'alnum') return Array.from({ length }, () => ALNUM[randomInt(ALNUM.length)]).join('');
    // Unpadded alphabets carry 4 (hex) or 6 (base64) bits per char; slice off the surplus.
    const bitsPerChar = format === 'hex' ? 4 : 6;
    const bytes = Math.ceil((length * bitsPerChar) / 8);
    return randomBytes(bytes).toString(format).replace(/=+$/, '').slice(0, length);
}

/** Token carrying exactly `bytes` random bytes in the given encoding. */
export function tokenByBytes(format: Exclude<TokenFormat, 'alnum'>, bytes: number): string {
    return randomBytes(bytes).toString(format);
}
