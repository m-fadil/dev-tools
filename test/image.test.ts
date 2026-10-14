import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { after, describe, test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { deflateSync } from 'node:zlib';
import { Src } from '../src/tools/image/bytes.js';
import { parseGif } from '../src/tools/image/gif.js';
import { parseJpeg } from '../src/tools/image/jpeg.js';

const CLI = fileURLToPath(new URL('../src/cli.js', import.meta.url));

/** run the CLI without a terminal; `input` is piped to stdin */
const cli = (args: string[], input = '', env: Record<string, string> = {}) => {
    const r = spawnSync(process.execPath, [CLI, ...args], {
        encoding: 'utf8',
        input,
        env: { PATH: process.env.PATH, ...env },
        maxBuffer: 64 * 1024 * 1024,
    });
    return { code: r.status, out: r.stdout, err: r.stderr };
};

const dir = mkdtempSync(join(tmpdir(), 'dt-image-'));
after(() => rmSync(dir, { recursive: true, force: true }));

/** write a fixture and return its path */
function fixture(name: string, data: Buffer | string): string {
    const path = join(dir, name);
    writeFileSync(path, data);
    return path;
}

interface Info {
    path: string;
    ok: boolean;
    error?: string;
    // Own CLI's parsed JSON; tests read many format-specific fields, so an index signature is the pragmatic shape.
    [key: string]: any;
}

/** inspect one file with --json */
function inspect(path: string): Info {
    const r = cli(['image', '--json', path]);
    return (JSON.parse(r.out) as { files: Info[] }).files[0]!;
}

const be32 = (n: number) => {
    const b = Buffer.alloc(4);
    b.writeUInt32BE(n);
    return b;
};

// ---- PNG ----

const CRC_TABLE = Array.from({ length: 256 }, (_, n) => {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    return c >>> 0;
});
const crc32 = (data: Buffer) => {
    let c = 0xffffffff;
    for (const byte of data) c = CRC_TABLE[(c ^ byte) & 0xff]! ^ (c >>> 8);
    return (c ^ 0xffffffff) >>> 0;
};
const pngChunk = (type: string, data: Buffer) => {
    const body = Buffer.concat([Buffer.from(type, 'latin1'), data]);
    return Buffer.concat([be32(data.length), body, be32(crc32(body))]);
};
const PNG_SIG = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);
const ihdr = (width: number, height: number, depth = 8, colorType = 6, interlace = 0) =>
    pngChunk('IHDR', Buffer.concat([be32(width), be32(height), Buffer.from([depth, colorType, 0, 0, interlace])]));
const pngFile = (...chunks: Buffer[]) => Buffer.concat([PNG_SIG, ...chunks]);
const idat = pngChunk('IDAT', deflateSync(Buffer.from([0, 0, 0, 0, 0])));
const iend = pngChunk('IEND', Buffer.alloc(0));

describe('image: PNG', () => {
    test('reads IHDR, pHYs and text chunks', () => {
        const path = fixture(
            'full.png',
            pngFile(
                ihdr(1920, 1080),
                pngChunk('pHYs', Buffer.concat([be32(2835), be32(2835), Buffer.from([1])])),
                pngChunk('tEXt', Buffer.from('Title\0Halo', 'latin1')),
                pngChunk('iTXt', Buffer.from('Author\0\0\0\0\0Budi', 'utf8')),
                idat,
                iend,
            ),
        );
        const info = inspect(path);
        assert.equal(info.ok, true);
        assert.equal(info.format, 'PNG');
        assert.equal(info.mime, 'image/png');
        assert.equal(info.width, 1920);
        assert.equal(info.height, 1080);
        assert.equal(info.aspectRatio, '16:9');
        assert.equal(info.bitDepth, 8);
        assert.equal(info.colorType, 'truecolor+alpha');
        assert.equal(info.interlaced, false);
        assert.equal(info.alpha, true);
        assert.deepEqual(info.dpi, { x: 72.01, y: 72.01 });
        assert.equal(info.textCount, 2);
        assert.deepEqual(info.textKeys, ['Title', 'Author']);
        assert.equal(info.truncated, false);
        assert.deepEqual(info.warnings, []);
        assert.equal(info.size, readFileSync(path).length);
    });

    test('text output lists a section prefixed with the path', () => {
        const path = fixture('text.png', pngFile(ihdr(2, 2, 1, 3, 1), pngChunk('tRNS', Buffer.from([0])), idat, iend));
        const r = cli(['image', path]);
        assert.equal(r.code, 0);
        assert.ok(r.out.startsWith(`${path}:\n`));
        assert.match(r.out, /format\s+PNG/);
        assert.match(r.out, /colorType\s+indexed/);
        assert.match(r.out, /interlaced\s+ya/);
        assert.match(r.out, /alpha\s+ya/); // tRNS
        assert.match(r.out, /aspectRatio\s+1:1/);
    });

    test('alpha follows color type, 1x1 grayscale has none', () => {
        const path = fixture('gray.png', pngFile(ihdr(1, 1, 8, 0), idat, iend));
        const info = inspect(path);
        assert.equal(info.colorType, 'grayscale');
        assert.equal(info.alpha, false);
        assert.equal(info.dpi, null);
    });

    test('caps listed text keys while counting every text chunk and sanitizing controls', () => {
        const key = pngChunk('tEXt', Buffer.from('A\x1b\x7f\x85\0value', 'latin1'));
        const empty = pngChunk('tEXt', Buffer.alloc(0));
        const path = fixture(
            'many-text.png',
            pngFile(ihdr(1, 1), key, Buffer.concat(Array<Buffer>(340000).fill(empty)), iend),
        );
        const info = inspect(path);
        assert.equal(info.ok, true);
        assert.equal(info.textCount, 340001);
        assert.equal(info.textKeys.length, 100);
        assert.equal(info.textKeys[0], 'A\uFFFD\uFFFD\uFFFD');
        assert.equal(info.truncated, false);
        const text = cli(['image', path]);
        assert.equal(text.code, 0);
        assert.ok(text.out.includes('A\uFFFD\uFFFD\uFFFD'));
        assert.ok(!text.out.includes('\x1b'));
    });

    test('metadata validation allows no IDAT and nonempty IEND but rejects ancillary CRC damage', () => {
        assert.equal(inspect(fixture('no-idat.png', pngFile(ihdr(1, 1), iend))).ok, true);
        assert.equal(
            inspect(fixture('nonempty-iend.png', pngFile(ihdr(1, 1), pngChunk('IEND', Buffer.from([1]))))).ok,
            true,
        );
        const text = pngChunk('tEXt', Buffer.from('Title\0value'));
        text[text.length - 1] = text[text.length - 1]! ^ 1;
        const info = inspect(fixture('ancillary-crc.png', pngFile(ihdr(1, 1), text, iend)));
        assert.equal(info.ok, false);
        assert.match(info.error!, /CRC chunk tEXt salah/);
    });

    test('corrupt and truncated files fail cleanly', () => {
        const good = pngFile(ihdr(1, 1), idat, iend);
        const cases: [string, Buffer, RegExp][] = [
            ['signature only', good.subarray(0, 8), /terpotong/],
            ['cut inside IHDR', good.subarray(0, 20), /terpotong/],
            ['cut inside IDAT', good.subarray(0, 8 + 25 + 10), /terpotong \(chunk IDAT\)/],
            ['IEND missing', good.subarray(0, good.length - 12), /tidak ada chunk IEND/],
            ['cut inside IEND', good.subarray(0, good.length - 3), /terpotong/],
            [
                'bad IHDR CRC',
                Buffer.concat([good.subarray(0, 8 + 8 + 13), Buffer.alloc(4), good.subarray(8 + 25)]),
                /CRC chunk IHDR/,
            ],
            ['no IHDR first', pngFile(idat, iend), /IHDR tidak ada di awal/],
            ['zero width', pngFile(ihdr(0, 1), iend), /ukuran PNG tidak valid/],
            ['bad depth for type', pngFile(ihdr(1, 1, 16, 3), iend), /kedalaman bit 16/],
            ['bad color type', pngFile(ihdr(1, 1, 8, 5), iend), /tipe warna PNG tidak valid: 5/],
        ];
        for (const [name, data, pattern] of cases) {
            const path = fixture(`bad-${name.replaceAll(' ', '-')}.png`, data);
            const r = cli(['image', path]);
            assert.equal(r.code, 1, name);
            assert.equal(r.out, '', name);
            assert.match(r.err, new RegExp(`${path.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}: `), name);
            assert.match(r.err, pattern, name);
        }
    });

    test('files larger than the read limit are flagged as truncated', () => {
        // 5 MiB IDAT: pHYs after it is beyond the 4 MiB limit
        const big = pngChunk('IDAT', Buffer.alloc(5 * 1024 * 1024, 7));
        const path = fixture(
            'big.png',
            pngFile(
                ihdr(10, 20),
                big,
                pngChunk('pHYs', Buffer.concat([be32(3780), be32(3780), Buffer.from([1])])),
                iend,
            ),
        );
        const info = inspect(path);
        assert.equal(info.ok, true);
        assert.equal(info.truncated, true);
        assert.equal(info.width, 10);
        assert.equal(info.dpi, null);
        assert.match(info.warnings[0], /4 MiB pertama/);
    });
});

// ---- JPEG ----

const u16 = (n: number, le: boolean) => {
    const b = Buffer.alloc(2);
    if (le) b.writeUInt16LE(n);
    else b.writeUInt16BE(n);
    return b;
};
const u32 = (n: number, le: boolean) => {
    const b = Buffer.alloc(4);
    if (le) b.writeUInt32LE(n);
    else b.writeUInt32BE(n);
    return b;
};

/** TIFF block with Make, Model, Software, Orientation and an Exif sub-IFD with DateTimeOriginal. */
function tiff(le: boolean, orientation = 6): Buffer {
    const ifd0Size = 2 + 5 * 12 + 4;
    const exifIfdOffset = 8 + ifd0Size;
    const dataStart = exifIfdOffset + 2 + 12 + 4;
    const data: Buffer[] = [];
    let dataLength = 0;
    const ascii = (text: string) => {
        const value = Buffer.from(`${text}\0`);
        if (value.length <= 4)
            return { count: value.length, value: Buffer.concat([value, Buffer.alloc(4 - value.length)]) };
        const offset = dataStart + dataLength;
        data.push(value);
        dataLength += value.length;
        return { count: value.length, value: u32(offset, le) };
    };
    const entry = (tag: number, type: number, count: number, value: Buffer) =>
        Buffer.concat([u16(tag, le), u16(type, le), u32(count, le), value]);
    const str = (tag: number, text: string) => {
        const { count, value } = ascii(text);
        return entry(tag, 2, count, value);
    };
    const ifd0 = Buffer.concat([
        u16(5, le),
        str(0x010f, 'Acme Corp'),
        str(0x0110, 'Cam 1'),
        entry(0x0112, 3, 1, Buffer.concat([u16(orientation, le), Buffer.alloc(2)])),
        str(0x0131, 'v1'),
        entry(0x8769, 4, 1, u32(exifIfdOffset, le)),
        u32(0, le),
    ]);
    const exifIfd = Buffer.concat([u16(1, le), str(0x9003, '2024:01:02 03:04:05'), u32(0, le)]);
    return Buffer.concat([Buffer.from(le ? 'II' : 'MM'), u16(42, le), u32(8, le), ifd0, exifIfd, ...data]);
}

const segment = (marker: number, data: Buffer) =>
    Buffer.concat([Buffer.from([0xff, marker]), u16(data.length + 2, false), data]);
const jfifSegment = segment(0xe0, Buffer.from([0x4a, 0x46, 0x49, 0x46, 0, 1, 2, 1, 0, 72, 0, 72, 0, 0]));
const exifSegment = (le: boolean, orientation?: number) =>
    segment(0xe1, Buffer.concat([Buffer.from('Exif\0\0'), tiff(le, orientation)]));
const sof = (marker: number, width: number, height: number, components = 3) =>
    segment(
        marker,
        Buffer.concat([
            Buffer.from([8]),
            u16(height, false),
            u16(width, false),
            Buffer.from([components]),
            Buffer.alloc(3 * components),
        ]),
    );
const SOI = Buffer.from([0xff, 0xd8]);
const SOS = segment(0xda, Buffer.from([1, 1, 0, 0, 63, 0]));
const EOI = Buffer.from([0xff, 0xd9]);
const jpegFile = (...parts: Buffer[]) => Buffer.concat([SOI, ...parts, Buffer.from([0x12, 0x34]), EOI]);

describe('image: JPEG', () => {
    test('baseline with JFIF and EXIF in both byte orders', () => {
        for (const le of [true, false]) {
            const path = fixture(`exif-${le}.jpg`, jpegFile(jfifSegment, exifSegment(le), sof(0xc0, 640, 480), SOS));
            const info = inspect(path);
            assert.equal(info.ok, true, String(le));
            assert.equal(info.format, 'JPEG');
            assert.equal(info.mime, 'image/jpeg');
            assert.equal(info.width, 640);
            assert.equal(info.height, 480);
            assert.equal(info.aspectRatio, '4:3');
            assert.equal(info.precision, 8);
            assert.equal(info.components, 3);
            assert.equal(info.progressive, false);
            assert.deepEqual(info.jfif, { version: '1.02', density: { x: 72, y: 72, unit: 'dpi' } });
            assert.deepEqual(info.exif, {
                orientation: 6,
                make: 'Acme Corp',
                model: 'Cam 1',
                software: 'v1',
                dateTimeOriginal: '2024:01:02 03:04:05',
            });
            assert.deepEqual(info.warnings, []);
        }
    });

    test('progressive SOF2 and no EXIF', () => {
        const info = inspect(fixture('prog.jpg', jpegFile(sof(0xc2, 3, 1, 1), SOS)));
        assert.equal(info.progressive, true);
        assert.equal(info.sof, 'SOF2');
        assert.equal(info.exif, null);
        assert.equal(info.jfif, null);
        assert.equal(info.aspectRatio, '3:1');
    });

    test('damaged EXIF is a warning, image data is still reported', () => {
        const bad = segment(0xe1, Buffer.concat([Buffer.from('Exif\0\0'), tiff(true).subarray(0, 30)]));
        const info = inspect(fixture('badexif.jpg', jpegFile(bad, sof(0xc0, 8, 8), SOS)));
        assert.equal(info.ok, true);
        assert.equal(info.width, 8);
        assert.match(info.warnings[0], /EXIF rusak/);
        const orient = inspect(fixture('orient.jpg', jpegFile(exifSegment(true, 9), sof(0xc0, 8, 8), SOS)));
        assert.equal(orient.exif.orientation, null);
        assert.match(orient.warnings[0], /orientasi tidak valid: 9/);
    });

    test('bad EXIF tags do not hide later tags or the ExifIFD in either byte order', () => {
        for (const le of [true, false]) {
            const data = tiff(le);
            u32(data.length + 100, le).copy(data, 8 + 2 + 8); // invalid Make string offset
            const app1 = segment(0xe1, Buffer.concat([Buffer.from('Exif\0\0'), data]));
            const info = inspect(fixture(`bad-make-${le}.jpg`, jpegFile(app1, sof(0xc0, 8, 8), SOS)));
            assert.equal(info.ok, true);
            assert.equal(info.exif.make, null);
            assert.equal(info.exif.model, 'Cam 1');
            assert.equal(info.exif.orientation, 6);
            assert.equal(info.exif.dateTimeOriginal, '2024:01:02 03:04:05');
            assert.match(info.warnings[0], /EXIF rusak/);

            // ExifIFD precedes IFD0: its pointer is readable before IFD0 runs out of entries.
            const entry = (tag: number, type: number, count: number, value: Buffer) =>
                Buffer.concat([u16(tag, le), u16(type, le), u32(count, le), value]);
            const partial = Buffer.concat([
                Buffer.from(le ? 'II' : 'MM'),
                u16(42, le),
                u32(26, le),
                u16(1, le),
                entry(0x9003, 2, 4, Buffer.from('now\0')),
                u32(0, le),
                u16(2, le),
                entry(0x8769, 4, 1, u32(8, le)),
            ]);
            const recovered = inspect(
                fixture(
                    `partial-ifd-${le}.jpg`,
                    jpegFile(segment(0xe1, Buffer.concat([Buffer.from('Exif\0\0'), partial])), sof(0xc0, 1, 1), SOS),
                ),
            );
            assert.equal(recovered.ok, true);
            assert.equal(recovered.exif.dateTimeOriginal, 'now');
            assert.match(recovered.warnings[0], /EXIF rusak/);
        }
    });

    test('EXIF strings replace terminal controls in JSON and text', () => {
        const data = tiff(true);
        const at = data.readUInt32LE(18);
        Buffer.from('A\x1b\n\x7f Corp').copy(data, at);
        const path = fixture(
            'control-exif.jpg',
            jpegFile(segment(0xe1, Buffer.concat([Buffer.from('Exif\0\0'), data])), sof(0xc0, 1, 1), SOS),
        );
        assert.equal(inspect(path).exif.make, 'A\uFFFD\uFFFD\uFFFD Corp');
        const text = cli(['image', path]);
        assert.equal(text.code, 0);
        assert.ok(text.out.includes('A\uFFFD\uFFFD\uFFFD Corp'));
        assert.ok(!text.out.includes('\x1b'));
    });

    test('segment lengths cut by the read limit preserve known SOF, but actual damage fails', () => {
        const prefix = Buffer.concat([SOI, sof(0xc0, 3, 2)]);
        for (const tail of [Buffer.from([0xff, 0xe1]), Buffer.from([0xff, 0xe1, 0])]) {
            const data = Buffer.concat([prefix, tail]);
            const result = parseJpeg(new Src(data, data.length + 10));
            assert.equal(result.width, 3);
            assert.equal(result.truncated, true);
            assert.match(result.warnings[0]!, /pertama.*dibaca/);
            assert.throws(() => parseJpeg(new Src(data, data.length)), /panjang segmen/);
            const noSof = Buffer.concat([SOI, tail]);
            assert.throws(() => parseJpeg(new Src(noSof, noSof.length + 10)), /panjang segmen/);
        }
        const limit = 4 * 1024 * 1024;
        const data = Buffer.alloc(limit + 10, 0xff);
        prefix.copy(data);
        Buffer.from([0xff, 0xe1, 0, 2, 0xff, 0xd9]).copy(data, limit - 3);
        const info = inspect(fixture('length-limit.jpg', data));
        assert.equal(info.ok, true);
        assert.equal(info.truncated, true);
        assert.equal(info.width, 3);
        assert.match(info.warnings[0], /4 MiB pertama/);
    });

    test('truncated and corrupt files fail cleanly', () => {
        const good = jpegFile(jfifSegment, sof(0xc0, 640, 480), SOS);
        const sofAt = 2 + jfifSegment.length;
        const cases: [string, Buffer, RegExp][] = [
            ['only SOI marker', good.subarray(0, 3), /terpotong/],
            ['cut before SOF', good.subarray(0, sofAt), /SOF tidak ditemukan/],
            ['cut inside segment', good.subarray(0, sofAt + 6), /terpotong/],
            ['cut after SOF', good.subarray(0, sofAt + sof(0xc0, 1, 1).length), /tidak ada SOS\/EOI/],
            ['EOI without SOF', Buffer.concat([SOI, jfifSegment, EOI]), /SOF tidak ditemukan/],
            ['garbage marker', Buffer.concat([SOI, jfifSegment, Buffer.from([0x11, 0x22])]), /marker JPEG tidak valid/],
            ['bad length', Buffer.concat([SOI, Buffer.from([0xff, 0xe0, 0x00, 0x01])]), /panjang segmen/],
            ['zero width', jpegFile(sof(0xc0, 0, 5), SOS), /ukuran JPEG tidak valid/],
        ];
        for (const [name, data, pattern] of cases) {
            const r = cli(['image', fixture(`bad-${name.replaceAll(' ', '-')}.jpg`, data)]);
            assert.equal(r.code, 1, name);
            assert.match(r.err, pattern, name);
        }
    });
});

// ---- GIF ----

const gifHeader = (version: string, width: number, height: number, colors = 2) =>
    Buffer.concat([
        Buffer.from(`GIF${version}`),
        u16(width, true),
        u16(height, true),
        Buffer.from([0x80 | (Math.log2(colors) - 1), 0, 0]),
        Buffer.alloc(colors * 3),
    ]);
const netscape = (loops: number) =>
    Buffer.concat([
        Buffer.from([0x21, 0xff, 11]),
        Buffer.from('NETSCAPE2.0'),
        Buffer.from([3, 1]),
        u16(loops, true),
        Buffer.from([0]),
    ]);
const gifFrame = Buffer.concat([
    Buffer.from([0x21, 0xf9, 4, 0, 10, 0, 0, 0]), // graphic control extension
    Buffer.from([0x2c, 0, 0, 0, 0, 1, 0, 1, 0, 0]),
    Buffer.from([2, 2, 0x44, 0x01, 0]),
]);
const TRAILER = Buffer.from([0x3b]);

describe('image: GIF', () => {
    test('2-frame GIF89a with NETSCAPE loop', () => {
        const info = inspect(
            fixture('anim.gif', Buffer.concat([gifHeader('89a', 3, 2, 4), netscape(5), gifFrame, gifFrame, TRAILER])),
        );
        assert.equal(info.ok, true);
        assert.equal(info.format, 'GIF');
        assert.equal(info.mime, 'image/gif');
        assert.equal(info.version, '89a');
        assert.equal(info.width, 3);
        assert.equal(info.height, 2);
        assert.equal(info.aspectRatio, '3:2');
        assert.equal(info.colorTableSize, 4);
        assert.equal(info.frames, 2);
        assert.equal(info.framesAtLeast, false);
        assert.equal(info.animated, true);
        assert.equal(info.loopCount, 5);
        assert.equal(info.truncated, false);
    });

    test('single frame 87a is not animated, no loop info, local color table is skipped', () => {
        const local = Buffer.concat([
            Buffer.from([0x2c, 0, 0, 0, 0, 1, 0, 1, 0, 0x80]),
            Buffer.alloc(6),
            Buffer.from([2, 2, 0x44, 0x01, 0]),
        ]);
        const info = inspect(fixture('still.gif', Buffer.concat([gifHeader('87a', 1, 1), local, TRAILER])));
        assert.equal(info.version, '87a');
        assert.equal(info.frames, 1);
        assert.equal(info.animated, false);
        assert.equal(info.loopCount, null);
        const infinite = inspect(
            fixture('inf.gif', Buffer.concat([gifHeader('89a', 1, 1), netscape(0), gifFrame, TRAILER])),
        );
        assert.equal(infinite.loopCount, 0);
    });

    test('truncated and corrupt files fail cleanly', () => {
        const good = Buffer.concat([gifHeader('89a', 3, 2), gifFrame, gifFrame, TRAILER]);
        const cases: [string, Buffer, RegExp][] = [
            ['header only', good.subarray(0, 8), /terpotong/],
            ['cut in color table', good.subarray(0, 15), /terpotong/],
            ['cut in frame', good.subarray(0, good.length - 6), /terpotong/],
            ['no trailer', good.subarray(0, good.length - 1), /terpotong/],
            [
                'bad block',
                Buffer.concat([gifHeader('89a', 1, 1), Buffer.from([0x99, 0x3b])]),
                /blok GIF tidak valid \(0x99\)/,
            ],
            ['no frames', Buffer.concat([gifHeader('89a', 1, 1), TRAILER]), /tidak berisi frame/],
            ['zero size', Buffer.concat([gifHeader('89a', 0, 1), gifFrame, TRAILER]), /ukuran GIF tidak valid/],
        ];
        for (const [name, data, pattern] of cases) {
            const r = cli(['image', fixture(`bad-${name.replaceAll(' ', '-')}.gif`, data)]);
            assert.equal(r.code, 1, name);
            assert.match(r.err, pattern, name);
        }
    });

    test('frame count at a bounded read is reported as "at least" without a huge fixture', () => {
        const full = Buffer.concat([
            gifHeader('89a', 2, 2),
            gifFrame,
            gifFrame,
            Buffer.from([0x21, 0xfe, 255]),
            Buffer.alloc(255),
            Buffer.from([0]),
            gifFrame,
            TRAILER,
        ]);
        const readLength = gifHeader('89a', 2, 2).length + 2 * gifFrame.length + 10;
        const result = parseGif(new Src(full.subarray(0, readLength), full.length));
        assert.equal(result.details.frames, 2);
        assert.equal(result.details.framesAtLeast, true);
        assert.equal(result.details.animated, true);
        assert.equal(result.truncated, true);
        assert.match(result.warnings[0]!, /pertama.*dibaca/);
        assert.throws(() => parseGif(new Src(full.subarray(0, readLength), readLength)), /terpotong/);
        assert.equal(parseGif(new Src(full, full.length)).details.frames, 3);
    });
});

// ---- WebP ----

const le32 = (n: number) => u32(n, true);
const riffChunk = (type: string, data: Buffer) =>
    Buffer.concat([Buffer.from(type, 'latin1'), le32(data.length), data, Buffer.alloc(data.length & 1)]);
const webpFile = (...chunks: Buffer[]) => {
    const body = Buffer.concat([Buffer.from('WEBP'), ...chunks]);
    return Buffer.concat([Buffer.from('RIFF'), le32(body.length), body]);
};
const vp8l = (width: number, height: number, alpha: boolean) =>
    riffChunk(
        'VP8L',
        Buffer.concat([
            Buffer.from([0x2f]),
            le32((width - 1) | ((height - 1) << 14) | (alpha ? 1 << 28 : 0)),
            Buffer.from([0, 0]),
        ]),
    );
const vp8x = (flags: number, width: number, height: number) =>
    riffChunk(
        'VP8X',
        Buffer.concat([Buffer.from([flags, 0, 0, 0]), le32(width - 1).subarray(0, 3), le32(height - 1).subarray(0, 3)]),
    );

describe('image: WebP', () => {
    test('VP8L lossless with alpha flag', () => {
        const info = inspect(fixture('ll.webp', webpFile(vp8l(100, 50, true))));
        assert.equal(info.format, 'WebP');
        assert.equal(info.mime, 'image/webp');
        assert.equal(info.variant, 'VP8L');
        assert.equal(info.width, 100);
        assert.equal(info.height, 50);
        assert.equal(info.aspectRatio, '2:1');
        assert.equal(info.alpha, true);
        assert.equal(info.animated, false);
        assert.equal(info.frames, 1);
        assert.equal(inspect(fixture('ll2.webp', webpFile(vp8l(1, 1, false)))).alpha, false);
    });

    test('VP8 lossy keyframe', () => {
        const frame = Buffer.concat([
            Buffer.from([0x10, 0, 0, 0x9d, 0x01, 0x2a]),
            u16(320, true),
            u16(240, true),
            Buffer.alloc(4),
        ]);
        const info = inspect(fixture('lossy.webp', webpFile(riffChunk('VP8 ', frame))));
        assert.equal(info.variant, 'VP8');
        assert.equal(info.width, 320);
        assert.equal(info.height, 240);
        assert.equal(info.aspectRatio, '4:3');
        assert.equal(info.alpha, false);
    });

    test('VP8X flags, canvas size and animation frames', () => {
        const anim = riffChunk('ANIM', Buffer.concat([Buffer.alloc(4), u16(3, true)]));
        const frame = riffChunk('ANMF', Buffer.alloc(17));
        const info = inspect(
            fixture('anim.webp', webpFile(vp8x(0x20 | 0x10 | 0x08 | 0x02, 16777216 / 2, 3), anim, frame, frame, frame)),
        );
        assert.equal(info.variant, 'VP8X');
        assert.equal(info.width, 8388608);
        assert.equal(info.height, 3);
        assert.equal(info.animated, true);
        assert.equal(info.alpha, true);
        assert.equal(info.icc, true);
        assert.equal(info.exif, true);
        assert.equal(info.xmp, false);
        assert.equal(info.frames, 3);
        assert.equal(info.framesAtLeast, false);
        assert.equal(info.loopCount, 3);
        const still = inspect(fixture('x.webp', webpFile(vp8x(0x04, 10, 10), vp8l(10, 10, false))));
        assert.equal(still.animated, false);
        assert.equal(still.xmp, true);
        assert.equal(still.loopCount, null);
    });

    test('large metadata before animation frames yields a lower bound, but still frames stay exact', () => {
        const icc = riffChunk('ICCP', Buffer.alloc(5 * 1024 * 1024));
        const animatedPath = fixture(
            'large-animated.webp',
            webpFile(vp8x(0x22, 10, 10), icc, riffChunk('ANIM', Buffer.alloc(6)), riffChunk('ANMF', Buffer.alloc(17))),
        );
        const animated = inspect(animatedPath);
        assert.equal(animated.ok, true);
        assert.equal(animated.animated, true);
        assert.equal(animated.frames, 0);
        assert.equal(animated.framesAtLeast, true);
        assert.equal(animated.truncated, true);
        assert.match(animated.warnings[0], /4 MiB pertama/);
        const text = cli(['image', animatedPath]);
        assert.equal(text.code, 0);
        assert.match(text.out, /frames\s+minimal 0/);
        assert.match(text.out, /peringatan:/);
        for (const [type, flags] of [
            ['ICCP', 0x20],
            ['EXIF', 0x08],
            ['XMP ', 0x04],
        ] as const) {
            const still = inspect(
                fixture(
                    `large-still-${type.trim()}.webp`,
                    webpFile(vp8x(flags, 10, 10), riffChunk(type, Buffer.alloc(5 * 1024 * 1024)), vp8l(10, 10, false)),
                ),
            );
            assert.equal(still.ok, true);
            assert.equal(still.frames, 1);
            assert.equal(still.framesAtLeast, false);
            assert.equal(still.truncated, false);
            assert.deepEqual(still.warnings, []);
        }
    });

    test('truncated and corrupt files fail cleanly', () => {
        const good = webpFile(vp8l(100, 50, false));
        const cases: [string, Buffer, RegExp][] = [
            ['header only', good.subarray(0, 12), /terpotong/],
            ['cut in chunk', good.subarray(0, 22), /terpotong \(RIFF/],
            ['unknown chunk', webpFile(riffChunk('JUNK', Buffer.alloc(4))), /chunk pertama WebP tidak dikenal: JUNK/],
            ['bad signature', webpFile(riffChunk('VP8L', Buffer.from([1, 0, 0, 0, 0, 0]))), /signature VP8L/],
            ['short VP8', webpFile(riffChunk('VP8 ', Buffer.alloc(4))), /terlalu pendek/],
            ['animation without frames', webpFile(vp8x(0x02, 5, 5)), /tidak berisi frame/],
            [
                'chunk past riff end',
                Buffer.concat([good.subarray(0, 16), le32(1000), good.subarray(20)]),
                /melewati akhir RIFF/,
            ],
        ];
        for (const [name, data, pattern] of cases) {
            const r = cli(['image', fixture(`bad-${name.replaceAll(' ', '-')}.webp`, data)]);
            assert.equal(r.code, 1, name);
            assert.match(r.err, pattern, name);
        }
    });
});

// ---- CLI behaviour ----

describe('image: CLI', () => {
    test('detects by magic bytes, not extension', () => {
        const path = fixture('actually-a-gif.png', Buffer.concat([gifHeader('89a', 1, 1), gifFrame, TRAILER]));
        assert.equal(inspect(path).format, 'GIF');
    });

    test('unknown format, missing file and directory are errors', () => {
        const text = fixture('note.png', 'bukan gambar sama sekali');
        const r = cli(['image', text]);
        assert.equal(r.code, 1);
        assert.equal(r.out, '');
        assert.equal(r.err.trim(), `format gambar tidak dikenal: ${text}`);
        assert.equal(cli(['image', fixture('empty.png', '')]).code, 1);

        const missing = join(dir, 'nope.png');
        assert.equal(cli(['image', missing]).err.trim(), `${missing}: file tidak ditemukan`);
        const sub = join(dir, 'sub');
        mkdirSync(sub);
        assert.equal(cli(['image', sub]).code, 1);
    });

    test('rejects a FIFO before opening it', (t) => {
        const path = join(dir, 'fifo');
        const created = spawnSync('mkfifo', [path], { encoding: 'utf8', timeout: 5000 });
        if (created.error && 'code' in created.error && created.error.code === 'ENOENT') {
            t.skip('mkfifo unavailable');
            return;
        }
        assert.equal(created.status, 0, created.stderr);
        const result = spawnSync(process.execPath, [CLI, 'image', '--json', path], { encoding: 'utf8', timeout: 5000 });
        assert.equal(result.error, undefined);
        assert.equal(result.status, 1);
        assert.match(result.stderr, /bukan file biasa/);
        const parsed = JSON.parse(result.stdout);
        assert.ok(Array.isArray(parsed.files));
        assert.equal(parsed.files[0].ok, false);
    });

    test('several files: all processed, errors to stderr, exit 1 if any failed', () => {
        const a = fixture('multi-a.png', pngFile(ihdr(4, 4), idat, iend));
        const b = fixture('multi-b.bin', 'zzz');
        const c = fixture('multi-c.gif', Buffer.concat([gifHeader('89a', 5, 5), gifFrame, TRAILER]));
        const r = cli(['image', a, b, c]);
        assert.equal(r.code, 1);
        assert.ok(r.out.includes(`${a}:\n`));
        assert.ok(r.out.includes(`${c}:\n`));
        assert.ok(!r.out.includes(b));
        assert.equal(r.out.split('\n\n').length, 2);
        assert.equal(r.err.trim(), `format gambar tidak dikenal: ${b}`);

        const j = cli(['image', '--json', a, b, c]);
        assert.equal(j.code, 1);
        const files = (JSON.parse(j.out) as { files: Info[] }).files;
        assert.deepEqual(
            files.map((f) => [f.path, f.ok]),
            [
                [a, true],
                [b, false],
                [c, true],
            ],
        );
        assert.equal(files[1]!.error, `format gambar tidak dikenal: ${b}`);
        assert.equal(files[1]!.format, undefined);
        assert.equal(files[0]!.size, pngFile(ihdr(4, 4), idat, iend).length);
        assert.equal(j.err.trim(), `format gambar tidak dikenal: ${b}`);
    });

    test('no file is a usage error', () => {
        assert.equal(cli(['image']).code, 2);
        assert.equal(cli(['image', '--bogus']).code, 2);
    });
});
