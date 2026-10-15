import { createHash, timingSafeEqual } from 'node:crypto';
import { createReadStream } from 'node:fs';
import { stat } from 'node:fs/promises';
import type { HashAlgorithm } from '../hash/hash.js';

/** Failure to hash a path; `reason` is a short message without the path, `code` the errno code if any. */
export class HashFileError extends Error {
    constructor(
        readonly path: string,
        readonly reason: string,
        message: string,
        readonly code?: string,
        cause?: unknown,
    ) {
        super(message, { cause });
    }
}

/** Streaming hex digest of a file; non-regular files and unreadable paths throw a HashFileError naming the path. */
export async function hashFile(path: string, algorithm: HashAlgorithm): Promise<string> {
    const info = await stat(path).catch((error: NodeJS.ErrnoException) => {
        if (error.code === 'ENOENT') {
            throw new HashFileError(path, 'tidak ditemukan', `File tidak ditemukan: ${path}`, error.code, error);
        }
        throw new HashFileError(
            path,
            error.code ?? error.message,
            `Tidak bisa membaca ${path}: ${error.message}`,
            error.code,
            error,
        );
    });
    if (info.isDirectory()) throw new HashFileError(path, 'direktori', `Bukan file (direktori): ${path}`);
    if (!info.isFile()) throw new HashFileError(path, 'bukan file biasa', `Bukan file biasa: ${path}`);
    const hasher = createHash(algorithm);
    try {
        for await (const chunk of createReadStream(path)) hasher.update(chunk as Buffer);
    } catch (error) {
        const code = (error as NodeJS.ErrnoException).code;
        throw new HashFileError(
            path,
            code ?? (error as Error).message,
            `Tidak bisa membaca ${path}: ${(error as Error).message}`,
            code,
            error,
        );
    }
    return hasher.digest('hex');
}

export interface SumEntry {
    line: number;
    digest: string;
    path: string;
}

export interface ParsedSums {
    entries: SumEntry[];
    /** 1-based numbers of malformed lines */
    invalid: number[];
}

/** Parses `<hex>  <path>` / `<hex> *<path>` lines; blank lines and `#` comments are skipped. */
export function parseSums(text: string, digestLength: number): ParsedSums {
    const entries: SumEntry[] = [];
    const invalid: number[] = [];
    text.split('\n').forEach((raw, index) => {
        const line = raw.endsWith('\r') ? raw.slice(0, -1) : raw;
        if (line.trim() === '' || line.trimStart().startsWith('#')) return;
        const match = /^([0-9a-fA-F]+) [ *](.+)$/.exec(line);
        if (!match || match[1]!.length !== digestLength * 2) {
            invalid.push(index + 1);
            return;
        }
        entries.push({ line: index + 1, digest: match[1]!, path: match[2]! });
    });
    return { entries, invalid };
}

/** Digest length in bytes for an algorithm. */
export function digestLength(algorithm: HashAlgorithm): number {
    return createHash(algorithm).digest().length;
}

/** Case-insensitive hex comparison in constant time (lengths are equal by construction). */
export function digestsEqual(a: string, b: string): boolean {
    const x = Buffer.from(a, 'hex');
    const y = Buffer.from(b, 'hex');
    return x.length === y.length && timingSafeEqual(x, y);
}
