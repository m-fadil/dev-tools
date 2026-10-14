import { ImageError } from './bytes.js';
export function parseWebp(s) {
    const riffSize = s.u32(4, true, 'header RIFF');
    const total = riffSize + 8;
    if (riffSize < 4 + 8)
        throw new ImageError(`ukuran RIFF tidak valid: ${riffSize}`);
    if (total > s.fileSize)
        throw new ImageError(`file terpotong (RIFF ${total} B, file ${s.fileSize} B)`);
    const type = s.latin1(12, 4, 'chunk pertama');
    const size = s.u32(16, true, 'chunk pertama');
    if (20 + size > total)
        throw new ImageError(`chunk ${type.trim()} melewati akhir RIFF`);
    const body = 20;
    let width;
    let height;
    let variant;
    let alpha = false;
    let animated = false;
    let icc = false;
    let exif = false;
    let xmp = false;
    let frames = 1;
    let loopCount = null;
    let truncated = false;
    if (type === 'VP8 ') {
        variant = 'VP8';
        if (size < 10)
            throw new ImageError('chunk VP8 terlalu pendek');
        if (s.u8(body, 'frame VP8') & 1)
            throw new ImageError('frame VP8 bukan keyframe');
        if (s.latin1(body + 3, 3) !== '\x9d\x01\x2a')
            throw new ImageError('start code VP8 tidak valid');
        width = s.u16(body + 6, true) & 0x3fff;
        height = s.u16(body + 8, true) & 0x3fff;
    }
    else if (type === 'VP8L') {
        variant = 'VP8L';
        if (size < 5)
            throw new ImageError('chunk VP8L terlalu pendek');
        if (s.u8(body, 'header VP8L') !== 0x2f)
            throw new ImageError('signature VP8L tidak valid');
        const bits = s.u32(body + 1, true);
        width = (bits & 0x3fff) + 1;
        height = ((bits >>> 14) & 0x3fff) + 1;
        alpha = ((bits >>> 28) & 1) === 1;
        if (bits >>> 29 !== 0)
            throw new ImageError(`versi VP8L tidak didukung: ${bits >>> 29}`);
    }
    else if (type === 'VP8X') {
        variant = 'VP8X';
        if (size < 10)
            throw new ImageError('chunk VP8X terlalu pendek');
        const flags = s.u8(body, 'header VP8X');
        icc = (flags & 0x20) !== 0;
        alpha = (flags & 0x10) !== 0;
        exif = (flags & 0x08) !== 0;
        xmp = (flags & 0x04) !== 0;
        animated = (flags & 0x02) !== 0;
        s.need(body, 10, 'header VP8X');
        width = s.b.readUIntLE(body + 4, 3) + 1;
        height = s.b.readUIntLE(body + 7, 3) + 1;
        if (animated) {
            frames = 0;
            let off = body + size + (size & 1);
            while (off < total) {
                if (!s.has(off, 8)) {
                    if (s.complete)
                        throw new ImageError('file terpotong (header chunk WebP)');
                    truncated = true;
                    break;
                }
                const chunk = s.latin1(off, 4);
                const length = s.u32(off + 4, true);
                const next = off + 8 + length + (length & 1);
                if (off + 8 + length > total)
                    throw new ImageError(`chunk ${chunk} melewati akhir RIFF`);
                if (chunk === 'ANMF')
                    frames++;
                else if (chunk === 'ANIM' && length >= 6 && s.has(off + 8, 6))
                    loopCount = s.u16(off + 12, true);
                if (off + 8 + length > s.length) {
                    truncated = true;
                    break;
                }
                off = next;
            }
            if (frames === 0 && !truncated)
                throw new ImageError('WebP animasi tidak berisi frame (ANMF)');
        }
    }
    else {
        throw new ImageError(`chunk pertama WebP tidak dikenal: ${type.trim()}`);
    }
    if (width === 0 || height === 0)
        throw new ImageError(`ukuran WebP tidak valid: ${width}x${height}`);
    return {
        width,
        height,
        truncated,
        warnings: truncated ? [s.limitWarning('jumlah frame minimal sebanyak yang terhitung')] : [],
        details: {
            variant,
            animated,
            alpha,
            icc,
            exif,
            xmp,
            frames,
            framesAtLeast: truncated,
            loopCount: animated ? loopCount : null,
        },
    };
}
//# sourceMappingURL=webp.js.map