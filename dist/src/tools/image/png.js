import { ImageError } from './bytes.js';
const COLOR_TYPES = {
    0: { name: 'grayscale', depths: [1, 2, 4, 8, 16] },
    2: { name: 'truecolor', depths: [8, 16] },
    3: { name: 'indexed', depths: [1, 2, 4, 8] },
    4: { name: 'grayscale+alpha', depths: [8, 16] },
    6: { name: 'truecolor+alpha', depths: [8, 16] },
};
const CRC_TABLE = Uint32Array.from({ length: 256 }, (_, n) => {
    let c = n;
    for (let k = 0; k < 8; k++)
        c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    return c >>> 0;
});
export function crc32(data) {
    let c = 0xffffffff;
    for (const byte of data)
        c = CRC_TABLE[(c ^ byte) & 0xff] ^ (c >>> 8);
    return (c ^ 0xffffffff) >>> 0;
}
export function parsePng(s) {
    let off = 8;
    let ihdr;
    let trns = false;
    let dpi = null;
    const textKeys = [];
    let textCount = 0;
    let ended = false;
    let truncated = false;
    for (;;) {
        if (!s.has(off, 8)) {
            if (s.complete)
                throw new ImageError('file terpotong (tidak ada chunk IEND)');
            truncated = true;
            break;
        }
        const length = s.u32(off, false);
        const type = s.latin1(off + 4, 4);
        if (!/^[A-Za-z]{4}$/.test(type))
            throw new ImageError(`tipe chunk PNG tidak valid pada offset ${off + 4}`);
        if (!ihdr && type !== 'IHDR')
            throw new ImageError('chunk IHDR tidak ada di awal file');
        if (length > 0x7fffffff)
            throw new ImageError(`panjang chunk ${type} tidak valid: ${length}`);
        const dataStart = off + 8;
        const next = dataStart + length + 4;
        if (next > s.length) {
            if (s.complete)
                throw new ImageError(`file terpotong (chunk ${type})`);
            truncated = true;
            break;
        }
        if (s.u32(dataStart + length, false) !== crc32(s.b.subarray(off + 4, dataStart + length))) {
            throw new ImageError(`CRC chunk ${type} salah`);
        }
        if (type === 'IHDR') {
            if (ihdr)
                throw new ImageError('chunk IHDR ganda');
            if (length !== 13)
                throw new ImageError(`panjang IHDR harus 13, bukan ${length}`);
            const width = s.u32(dataStart, false);
            const height = s.u32(dataStart + 4, false);
            const depth = s.u8(dataStart + 8);
            const colorType = s.u8(dataStart + 9);
            const interlace = s.u8(dataStart + 12);
            if (width === 0 || height === 0 || width > 0x7fffffff || height > 0x7fffffff) {
                throw new ImageError(`ukuran PNG tidak valid: ${width}x${height}`);
            }
            const spec = COLOR_TYPES[colorType];
            if (!spec)
                throw new ImageError(`tipe warna PNG tidak valid: ${colorType}`);
            if (!spec.depths.includes(depth)) {
                throw new ImageError(`kedalaman bit ${depth} tidak valid untuk tipe warna ${spec.name}`);
            }
            if (s.u8(dataStart + 10) !== 0 || s.u8(dataStart + 11) !== 0) {
                throw new ImageError('metode kompresi/filter PNG tidak dikenal');
            }
            if (interlace > 1)
                throw new ImageError(`metode interlace PNG tidak valid: ${interlace}`);
            ihdr = { width, height, depth, colorType, interlace };
        }
        else if (type === 'tRNS') {
            trns = true;
        }
        else if (type === 'pHYs') {
            if (length !== 9)
                throw new ImageError(`panjang pHYs harus 9, bukan ${length}`);
            const x = s.u32(dataStart, false);
            const y = s.u32(dataStart + 4, false);
            // Unit 1 = metre; 0 means only an aspect ratio.
            if (s.u8(dataStart + 8) === 1 && x > 0 && y > 0) {
                dpi = { x: Math.round(x * 2.54) / 100, y: Math.round(y * 2.54) / 100 };
            }
        }
        else if (type === 'tEXt' || type === 'zTXt' || type === 'iTXt') {
            textCount++;
            if (textKeys.length < 100) {
                const raw = s.b.subarray(dataStart, dataStart + length);
                const nul = raw.indexOf(0);
                textKeys.push(raw.toString('latin1', 0, nul < 0 ? raw.length : nul).replace(/[\p{Cc}]/gu, '\uFFFD'));
            }
        }
        else if (type === 'IEND') {
            ended = true;
            break;
        }
        off = next;
    }
    if (!ihdr)
        throw new ImageError('chunk IHDR tidak ditemukan');
    const warnings = [];
    if (truncated && !ended) {
        warnings.push(s.limitWarning('chunk setelahnya (mis. pHYs, teks) mungkin tidak terbaca'));
    }
    return {
        width: ihdr.width,
        height: ihdr.height,
        truncated,
        warnings,
        details: {
            bitDepth: ihdr.depth,
            colorType: COLOR_TYPES[ihdr.colorType].name,
            interlaced: ihdr.interlace === 1,
            alpha: ihdr.colorType === 4 || ihdr.colorType === 6 || trns,
            dpi,
            textCount,
            textKeys,
        },
    };
}
//# sourceMappingURL=png.js.map