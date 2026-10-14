import { createHash, timingSafeEqual } from 'node:crypto';
import { createReadStream } from 'node:fs';
import { stat } from 'node:fs/promises';
/** Failure to hash a path; `reason` is a short message without the path, `code` the errno code if any. */
export class HashFileError extends Error {
    path;
    reason;
    code;
    constructor(path, reason, message, code, cause) {
        super(message, { cause });
        this.path = path;
        this.reason = reason;
        this.code = code;
    }
}
/** Streaming hex digest of a file; non-regular files and unreadable paths throw a HashFileError naming the path. */
export async function hashFile(path, algorithm) {
    const info = await stat(path).catch((error) => {
        if (error.code === 'ENOENT') {
            throw new HashFileError(path, 'tidak ditemukan', `File tidak ditemukan: ${path}`, error.code, error);
        }
        throw new HashFileError(path, error.code ?? error.message, `Tidak bisa membaca ${path}: ${error.message}`, error.code, error);
    });
    if (info.isDirectory())
        throw new HashFileError(path, 'direktori', `Bukan file (direktori): ${path}`);
    if (!info.isFile())
        throw new HashFileError(path, 'bukan file biasa', `Bukan file biasa: ${path}`);
    const hasher = createHash(algorithm);
    try {
        for await (const chunk of createReadStream(path))
            hasher.update(chunk);
    }
    catch (error) {
        const code = error.code;
        throw new HashFileError(path, code ?? error.message, `Tidak bisa membaca ${path}: ${error.message}`, code, error);
    }
    return hasher.digest('hex');
}
/** Parses `<hex>  <path>` / `<hex> *<path>` lines; blank lines and `#` comments are skipped. */
export function parseSums(text, digestLength) {
    const entries = [];
    const invalid = [];
    text.split('\n').forEach((raw, index) => {
        const line = raw.endsWith('\r') ? raw.slice(0, -1) : raw;
        if (line.trim() === '' || line.trimStart().startsWith('#'))
            return;
        const match = /^([0-9a-fA-F]+) [ *](.+)$/.exec(line);
        if (!match || match[1].length !== digestLength * 2) {
            invalid.push(index + 1);
            return;
        }
        entries.push({ line: index + 1, digest: match[1], path: match[2] });
    });
    return { entries, invalid };
}
/** Digest length in bytes for an algorithm. */
export function digestLength(algorithm) {
    return createHash(algorithm).digest().length;
}
/** Case-insensitive hex comparison in constant time (lengths are equal by construction). */
export function digestsEqual(a, b) {
    const x = Buffer.from(a, 'hex');
    const y = Buffer.from(b, 'hex');
    return x.length === y.length && timingSafeEqual(x, y);
}
//# sourceMappingURL=checksum.js.map