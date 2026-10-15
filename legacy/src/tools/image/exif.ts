import { ImageError, Src } from './bytes.js';

export interface Exif {
    orientation: number | null;
    make: string | null;
    model: string | null;
    software: string | null;
    dateTimeOriginal: string | null;
}

const TYPE_ASCII = 2;
const TYPE_SHORT = 3;
const TYPE_LONG = 4;

/**
 * Reads the few wanted tags from a TIFF structure (APP1 payload after "Exif\0\0").
 * Both byte orders are supported. Damage never fails the image: it becomes a warning and keeps what was read.
 */
export function parseExif(tiff: Buffer): { exif: Exif; warnings: string[] } {
    const exif: Exif = { orientation: null, make: null, model: null, software: null, dateTimeOriginal: null };
    const warnings: string[] = [];
    // Complete = true: the TIFF block is the whole universe, any overrun is damage.
    const s = new Src(tiff, tiff.length);
    try {
        const order = s.latin1(0, 2, 'header TIFF');
        if (order !== 'II' && order !== 'MM') throw new ImageError('urutan byte TIFF tidak dikenal');
        const little = order === 'II';
        if (s.u16(2, little) !== 42) throw new ImageError('magic TIFF bukan 42');

        const readIfd = (offset: number, onTag: (tag: number, type: number, count: number, entry: number) => void) => {
            let count: number;
            try {
                count = s.u16(offset, little, 'IFD');
            } catch (error) {
                if (!(error instanceof ImageError)) throw error;
                warnings.push(`EXIF rusak: ${error.message}`);
                return;
            }
            for (let i = 0; i < count; i++) {
                const entry = offset + 2 + i * 12;
                if (!s.has(entry, 12)) {
                    warnings.push('EXIF rusak: file terpotong (entri IFD)');
                    break;
                }
                try {
                    onTag(s.u16(entry, little), s.u16(entry + 2, little), s.u32(entry + 4, little), entry);
                } catch (error) {
                    if (!(error instanceof ImageError)) throw error;
                    warnings.push(`EXIF rusak: ${error.message}`);
                }
            }
        };
        const ascii = (type: number, count: number, entry: number): string | null => {
            if (type !== TYPE_ASCII || count === 0) return null;
            const start = count <= 4 ? entry + 8 : s.u32(entry + 8, little);
            s.need(start, count, 'string EXIF');
            const raw = tiff.subarray(start, start + count);
            const nul = raw.indexOf(0);
            return (
                raw
                    .toString('utf8', 0, nul < 0 ? raw.length : nul)
                    .replace(/[\p{Cc}]/gu, '\uFFFD')
                    .trim() || null
            );
        };

        let exifIfd = 0;
        readIfd(s.u32(4, little), (tag, type, count, entry) => {
            if (tag === 0x010f) exif.make = ascii(type, count, entry);
            else if (tag === 0x0110) exif.model = ascii(type, count, entry);
            else if (tag === 0x0131) exif.software = ascii(type, count, entry);
            else if (tag === 0x0112 && type === TYPE_SHORT && count === 1) {
                const value = s.u16(entry + 8, little);
                if (value >= 1 && value <= 8) exif.orientation = value;
                else warnings.push(`EXIF: nilai orientasi tidak valid: ${value}`);
            } else if (tag === 0x8769 && type === TYPE_LONG && count === 1) exifIfd = s.u32(entry + 8, little);
        });
        if (exifIfd) {
            readIfd(exifIfd, (tag, type, count, entry) => {
                if (tag === 0x9003) exif.dateTimeOriginal = ascii(type, count, entry);
            });
        }
    } catch (error) {
        if (!(error instanceof ImageError)) throw error;
        warnings.push(`EXIF rusak: ${error.message}`);
    }
    return { exif, warnings };
}
