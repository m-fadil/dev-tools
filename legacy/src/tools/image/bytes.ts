/** A malformed or truncated image; the message is the reason only (the caller prefixes the file name). */
export class ImageError extends Error {
    override name = 'ImageError';
}

/** Read past the end of the available bytes. */
export class ShortData extends ImageError {
    override name = 'ShortData';
}

export interface Parsed {
    width: number;
    height: number;
    /** The scan stopped at the read limit before the end of the file. */
    truncated: boolean;
    warnings: string[];
    /** Format specific fields, serialized flat into the result. */
    details: Record<string, unknown>;
}

const MIB = 1024 * 1024;

/** Bounds-checked view over the bytes read from a file; every accessor throws ShortData instead of reading out of range. */
export class Src {
    /** All bytes of the file are present (the read limit was not hit). */
    readonly complete: boolean;

    constructor(
        readonly b: Buffer,
        readonly fileSize: number,
    ) {
        this.complete = b.length >= fileSize;
    }

    get length(): number {
        return this.b.length;
    }

    has(off: number, len: number): boolean {
        return off >= 0 && len >= 0 && off + len <= this.b.length;
    }

    need(off: number, len: number, what = 'data'): void {
        if (this.has(off, len)) return;
        throw new ShortData(
            this.complete
                ? `file terpotong (${what})`
                : `${what} melewati batas baca ${+(this.b.length / MIB).toFixed(1)} MiB`,
        );
    }

    u8(off: number, what?: string): number {
        this.need(off, 1, what);
        return this.b[off]!;
    }

    u16(off: number, little: boolean, what?: string): number {
        this.need(off, 2, what);
        return little ? this.b.readUInt16LE(off) : this.b.readUInt16BE(off);
    }

    u32(off: number, little: boolean, what?: string): number {
        this.need(off, 4, what);
        return little ? this.b.readUInt32LE(off) : this.b.readUInt32BE(off);
    }

    latin1(off: number, len: number, what?: string): string {
        this.need(off, len, what);
        return this.b.toString('latin1', off, off + len);
    }

    /** Warning for a scan that stopped at the read limit; `consequence` says what may be missing. */
    limitWarning(consequence: string): string {
        return `hanya ${+(this.b.length / MIB).toFixed(1)} MiB pertama dari ${this.fileSize} B dibaca; ${consequence}`;
    }
}
