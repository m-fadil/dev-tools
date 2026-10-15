import { createCipheriv, createDecipheriv, randomBytes, scryptSync } from 'node:crypto';

// Token layout (base64url): version(1) | salt(16) | iv(12) | auth tag(16) | ciphertext.
// AES-256-GCM, key = scrypt(password, salt) with the parameters below. The version byte is bound as GCM AAD and
// identifies the whole format (including scrypt parameters), so any change to them needs a new version.
const VERSION = 1;
const SALT = 16;
const IV = 12;
const TAG = 16;
const SCRYPT = { N: 16384, r: 8, p: 1 };

const deriveKey = (password: string, salt: Buffer): Buffer => scryptSync(password, salt, 32, SCRYPT);
const aad = Buffer.from([VERSION]);

export function encrypt(plain: Buffer, password: string): string {
    const salt = randomBytes(SALT);
    const iv = randomBytes(IV);
    const cipher = createCipheriv('aes-256-gcm', deriveKey(password, salt), iv);
    cipher.setAAD(aad);
    const body = Buffer.concat([cipher.update(plain), cipher.final()]);
    return Buffer.concat([Buffer.from([VERSION]), salt, iv, cipher.getAuthTag(), body]).toString('base64url');
}

export function decrypt(token: string, password: string): Buffer {
    const raw = Buffer.from(token.trim(), 'base64url');
    if (raw.length < 1 + SALT + IV + TAG || raw[0] !== VERSION)
        throw new Error('Format data terenkripsi tidak dikenal.');
    const salt = raw.subarray(1, 1 + SALT);
    const iv = raw.subarray(1 + SALT, 1 + SALT + IV);
    const tag = raw.subarray(1 + SALT + IV, 1 + SALT + IV + TAG);
    const body = raw.subarray(1 + SALT + IV + TAG);
    const decipher = createDecipheriv('aes-256-gcm', deriveKey(password, salt), iv);
    decipher.setAAD(aad);
    decipher.setAuthTag(tag);
    try {
        return Buffer.concat([decipher.update(body), decipher.final()]);
    } catch (error) {
        throw new Error('Dekripsi gagal: password salah atau data rusak.', { cause: error });
    }
}
