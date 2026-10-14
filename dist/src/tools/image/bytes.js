/** A malformed or truncated image; the message is the reason only (the caller prefixes the file name). */
export class ImageError extends Error {
    name = 'ImageError';
}
/** Read past the end of the available bytes. */
export class ShortData extends ImageError {
    name = 'ShortData';
}
const MIB = 1024 * 1024;
/** Bounds-checked view over the bytes read from a file; every accessor throws ShortData instead of reading out of range. */
export class Src {
    b;
    fileSize;
    /** All bytes of the file are present (the read limit was not hit). */
    complete;
    constructor(b, fileSize) {
        this.b = b;
        this.fileSize = fileSize;
        this.complete = b.length >= fileSize;
    }
    get length() {
        return this.b.length;
    }
    has(off, len) {
        return off >= 0 && len >= 0 && off + len <= this.b.length;
    }
    need(off, len, what = 'data') {
        if (this.has(off, len))
            return;
        throw new ShortData(this.complete
            ? `file terpotong (${what})`
            : `${what} melewati batas baca ${+(this.b.length / MIB).toFixed(1)} MiB`);
    }
    u8(off, what) {
        this.need(off, 1, what);
        return this.b[off];
    }
    u16(off, little, what) {
        this.need(off, 2, what);
        return little ? this.b.readUInt16LE(off) : this.b.readUInt16BE(off);
    }
    u32(off, little, what) {
        this.need(off, 4, what);
        return little ? this.b.readUInt32LE(off) : this.b.readUInt32BE(off);
    }
    latin1(off, len, what) {
        this.need(off, len, what);
        return this.b.toString('latin1', off, off + len);
    }
    /** Warning for a scan that stopped at the read limit; `consequence` says what may be missing. */
    limitWarning(consequence) {
        return `hanya ${+(this.b.length / MIB).toFixed(1)} MiB pertama dari ${this.fileSize} B dibaca; ${consequence}`;
    }
}
//# sourceMappingURL=bytes.js.map