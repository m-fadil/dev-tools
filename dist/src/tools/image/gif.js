import { ImageError, ShortData } from './bytes.js';
export function parseGif(s) {
    const version = s.latin1(3, 3);
    const width = s.u16(6, true, 'layar logis');
    const height = s.u16(8, true, 'layar logis');
    if (width === 0 || height === 0)
        throw new ImageError(`ukuran GIF tidak valid: ${width}x${height}`);
    const flags = s.u8(10);
    const globalColors = flags & 0x80 ? 1 << ((flags & 7) + 1) : 0;
    let off = 13 + globalColors * 3;
    let frames = 0;
    let loopCount = null;
    let truncated = false;
    /** Skips a chain of data sub-blocks, returning the offset after the 0-length terminator. */
    const skipSubBlocks = (from) => {
        let at = from;
        for (;;) {
            const size = s.u8(at, 'sub-blok GIF');
            at++;
            if (size === 0)
                return at;
            s.need(at, size, 'sub-blok GIF');
            at += size;
        }
    };
    try {
        s.need(13, globalColors * 3, 'tabel warna global');
        for (;;) {
            const block = s.u8(off, 'trailer GIF');
            off++;
            if (block === 0x3b)
                break;
            if (block === 0x21) {
                const label = s.u8(off, 'ekstensi GIF');
                off++;
                if (label === 0xff && s.u8(off) === 11) {
                    const id = s.latin1(off + 1, 11);
                    if (id === 'NETSCAPE2.0' || id === 'ANIMEXTS1.0') {
                        const at = off + 12;
                        if (s.u8(at) >= 3 && s.u8(at + 1) === 1)
                            loopCount = s.u16(at + 2, true);
                    }
                }
                off = skipSubBlocks(off);
            }
            else if (block === 0x2c) {
                const packed = s.u8(off + 8, 'deskriptor gambar');
                off += 9;
                if (packed & 0x80) {
                    off += 3 * (1 << ((packed & 7) + 1));
                }
                s.u8(off, 'data gambar'); // LZW minimum code size
                off = skipSubBlocks(off + 1);
                frames++;
            }
            else {
                throw new ImageError(`blok GIF tidak valid (0x${block.toString(16)}) pada offset ${off - 1}`);
            }
        }
    }
    catch (error) {
        // Running out of bytes is only acceptable when the read limit (not the file) cut the scan short.
        if (!(error instanceof ShortData) || s.complete)
            throw error;
        truncated = true;
    }
    if (frames === 0 && !truncated)
        throw new ImageError('GIF tidak berisi frame');
    return {
        width,
        height,
        truncated,
        warnings: truncated ? [s.limitWarning('jumlah frame minimal sebanyak yang terhitung')] : [],
        details: {
            version,
            colorTableSize: globalColors,
            frames,
            framesAtLeast: truncated,
            animated: truncated && frames < 2 ? null : frames > 1,
            loopCount,
        },
    };
}
//# sourceMappingURL=gif.js.map