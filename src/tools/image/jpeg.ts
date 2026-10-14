import { ImageError, type Parsed, type Src } from './bytes.js';
import { parseExif, type Exif } from './exif.js';

const PROGRESSIVE_SOF = new Set([0xc2, 0xc6, 0xca, 0xce]);
const DENSITY_UNITS = ['none', 'dpi', 'dpcm'];

export function parseJpeg(s: Src): Parsed {
    let off = 2;
    let sof: { marker: number; precision: number; height: number; width: number; components: number } | undefined;
    let jfif: { version: string; density: { x: number; y: number; unit: string } } | null = null;
    let exif: Exif | null = null;
    const warnings: string[] = [];
    let truncated = false;

    for (;;) {
        if (!s.has(off, 2)) {
            // Out of data before SOS/EOI: only acceptable when the read limit (not the file) ended it.
            if (!sof || s.complete) {
                throw new ImageError(
                    sof ? 'file terpotong (tidak ada SOS/EOI)' : 'file terpotong (SOF tidak ditemukan)',
                );
            }
            truncated = true;
            break;
        }
        if (s.u8(off) !== 0xff) throw new ImageError(`marker JPEG tidak valid pada offset ${off}`);
        const marker = s.u8(off + 1);
        if (marker === 0xff) {
            off++; // fill byte
            continue;
        }
        if (marker === 0x00) throw new ImageError(`marker JPEG tidak valid pada offset ${off}`);
        off += 2;
        if (marker === 0xd8 || marker === 0x01 || (marker >= 0xd0 && marker <= 0xd7)) continue;
        if (marker === 0xd9) break; // EOI
        if (marker === 0xda) break; // SOS: entropy coded data follows, no more metadata

        if (!s.has(off, 2)) {
            if (!sof || s.complete) throw new ImageError('file terpotong (panjang segmen JPEG)');
            truncated = true;
            break;
        }

        const length = s.u16(off, false, 'panjang segmen');
        if (length < 2) throw new ImageError(`panjang segmen JPEG tidak valid pada offset ${off}`);
        const start = off + 2;
        const end = off + length;
        if (end > s.length) {
            if (!sof || s.complete) {
                throw new ImageError(
                    s.complete ? 'file terpotong (segmen JPEG)' : 'segmen JPEG melewati batas baca sebelum SOF',
                );
            }
            truncated = true;
            break;
        }
        const size = length - 2;

        const isSof = marker >= 0xc0 && marker <= 0xcf && marker !== 0xc4 && marker !== 0xc8 && marker !== 0xcc;
        if (isSof && !sof) {
            if (size < 6) throw new ImageError('segmen SOF terlalu pendek');
            const components = s.u8(start + 5);
            if (components === 0 || size < 6 + 3 * components) throw new ImageError('segmen SOF tidak valid');
            const height = s.u16(start + 1, false);
            const width = s.u16(start + 3, false);
            if (width === 0 || height === 0) {
                throw new ImageError(`ukuran JPEG tidak valid: ${width}x${height} (tinggi 0/DNL tidak didukung)`);
            }
            sof = { marker, precision: s.u8(start), height, width, components };
        } else if (marker === 0xe0 && !jfif && size >= 14 && s.latin1(start, 5) === 'JFIF\0') {
            const unit = s.u8(start + 7);
            if (unit <= 2) {
                jfif = {
                    version: `${s.u8(start + 5)}.${String(s.u8(start + 6)).padStart(2, '0')}`,
                    density: { x: s.u16(start + 8, false), y: s.u16(start + 10, false), unit: DENSITY_UNITS[unit]! },
                };
            } else warnings.push(`JFIF: satuan densitas tidak valid: ${unit}`);
        } else if (marker === 0xe1 && !exif && size >= 6 && s.latin1(start, 6) === 'Exif\0\0') {
            const parsed = parseExif(s.b.subarray(start + 6, end));
            exif = parsed.exif;
            warnings.push(...parsed.warnings);
        }
        off = end;
    }

    if (!sof) throw new ImageError('SOF tidak ditemukan (bukan JPEG yang utuh)');
    if (truncated) warnings.push(s.limitWarning('segmen setelahnya (mis. EXIF) mungkin tidak terbaca'));
    return {
        width: sof.width,
        height: sof.height,
        truncated,
        warnings,
        details: {
            precision: sof.precision,
            components: sof.components,
            sof: `SOF${sof.marker - 0xc0}`,
            progressive: PROGRESSIVE_SOF.has(sof.marker),
            jfif,
            exif,
        },
    };
}
