import { open, stat } from 'node:fs/promises';
import { ImageError, Src, type Parsed } from './bytes.js';
import { parseGif } from './gif.js';
import { parseJpeg } from './jpeg.js';
import { parsePng } from './png.js';
import { parseWebp } from './webp.js';

const MIB = 1024 * 1024;

interface Format {
    name: string;
    mime: string;
    /** Max bytes read from the file. */
    limit: number;
    parse: (src: Src) => Parsed;
}

const FORMATS: Record<string, Format> = {
    png: { name: 'PNG', mime: 'image/png', limit: 4 * MIB, parse: parsePng },
    jpeg: { name: 'JPEG', mime: 'image/jpeg', limit: 4 * MIB, parse: parseJpeg },
    gif: { name: 'GIF', mime: 'image/gif', limit: 64 * MIB, parse: parseGif },
    webp: { name: 'WebP', mime: 'image/webp', limit: 4 * MIB, parse: parseWebp },
};

/** Detects the format from magic bytes only. */
function detect(head: Buffer): Format | undefined {
    if (head.length >= 8 && head.subarray(0, 8).equals(Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]))) {
        return FORMATS.png;
    }
    if (head.length >= 3 && head[0] === 0xff && head[1] === 0xd8 && head[2] === 0xff) return FORMATS.jpeg;
    const ascii = head.toString('latin1', 0, 12);
    if (/^GIF8[79]a/.test(ascii)) return FORMATS.gif;
    if (ascii.length === 12 && ascii.startsWith('RIFF') && ascii.endsWith('WEBP')) return FORMATS.webp;
    return undefined;
}

function gcd(a: number, b: number): number {
    return b === 0 ? a : gcd(b, a % b);
}

export interface FileResult {
    path: string;
    ok: boolean;
    error?: string;
    [key: string]: unknown;
}

const FS_ERRORS: Record<string, string> = {
    ENOENT: 'file tidak ditemukan',
    EACCES: 'izin ditolak',
    EISDIR: 'bukan file biasa',
};

/** Reads at most the format's read limit and extracts metadata; never throws, failures become `{ ok: false, error }`. */
export async function inspectFile(path: string): Promise<FileResult> {
    try {
        if (!(await stat(path)).isFile()) throw new ImageError('bukan file biasa');
        const handle = await open(path, 'r');
        try {
            const stat = await handle.stat();
            if (!stat.isFile()) throw new ImageError('bukan file biasa');
            const { size } = stat;

            const read = async (length: number): Promise<Buffer> => {
                const buf = Buffer.alloc(Math.min(size, length));
                let filled = 0;
                while (filled < buf.length) {
                    const { bytesRead } = await handle.read(buf, filled, buf.length - filled, filled);
                    if (bytesRead === 0) break;
                    filled += bytesRead;
                }
                return buf.subarray(0, filled);
            };

            const format = detect(await read(12));
            if (!format) return { path, ok: false, error: `format gambar tidak dikenal: ${path}` };
            const data = await read(format.limit);
            const parsed = format.parse(new Src(data, size));
            const { width, height, truncated, warnings, details } = parsed;
            const divisor = gcd(width, height);
            return {
                path,
                ok: true,
                format: format.name,
                mime: format.mime,
                size,
                width,
                height,
                aspectRatio: `${width / divisor}:${height / divisor}`,
                ...details,
                truncated,
                warnings,
            };
        } finally {
            await handle.close();
        }
    } catch (error) {
        const reason =
            error instanceof ImageError
                ? error.message
                : (FS_ERRORS[(error as NodeJS.ErrnoException).code ?? ''] ?? (error as Error).message);
        return { path, ok: false, error: `${path}: ${reason}` };
    }
}
