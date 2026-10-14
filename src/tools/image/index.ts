import type { Tool } from '../../tool.js';
import { UsageError } from '../../errors.js';
import { parseToolArgs, printJson } from '../../io.js';
import { inspectFile, type FileResult } from './inspect.js';

function help(): void {
    console.log(`dt image [options] <file...>

Tampilkan metadata gambar tanpa dependensi: format (dari magic bytes, bukan ekstensi),
ukuran, rasio, dan detail per format. Didukung: PNG, JPEG (+EXIF), GIF, WebP.
  --json        Keluaran JSON: { files: [{ path, ok, format, mime, size, width, height, ... | error }] }
  -h, --help    Tampilkan bantuan

Membaca maksimal 4 MiB pertama (PNG/JPEG/WebP) atau 64 MiB (GIF, untuk menghitung frame);
bila terpotong, peringatan dicetak dan jumlah frame berarti "minimal".
Beberapa file diproses semua; kegagalan dicetak ke stderr.
Exit: 0 sukses, 1 ada file gagal dibaca/rusak/tidak dikenal, 2 opsi tidak valid.
Contoh: dt image foto.jpg logo.png   |   dt image --json anim.gif`);
}

const NOT_PRINTED = new Set(['path', 'ok', 'error', 'warnings', 'framesAtLeast', 'truncated']);

function format(key: string, value: unknown, result: FileResult): string {
    if (value === null || value === undefined) return '-';
    if (typeof value === 'boolean') return value ? 'ya' : 'tidak';
    if (key === 'size') return `${value} B`;
    if (key === 'frames' && result.framesAtLeast) return `minimal ${value}`;
    if (Array.isArray(value)) return value.length ? value.join(', ') : '-';
    return String(value);
}

/** Flattens nested objects to `parent.child` rows. */
function rows(result: FileResult): [string, string][] {
    const out: [string, string][] = [];
    const add = (key: string, value: unknown) => {
        if (value !== null && typeof value === 'object' && !Array.isArray(value)) {
            for (const [k, v] of Object.entries(value)) add(`${key}.${k}`, v);
        } else out.push([key, format(key, value, result)]);
    };
    for (const [key, value] of Object.entries(result)) if (!NOT_PRINTED.has(key)) add(key, value);
    return out;
}

function printSection(result: FileResult): void {
    console.log(`${result.path}:`);
    const table = rows(result);
    const width = Math.max(...table.map(([key]) => key.length));
    for (const [key, value] of table) console.log(`  ${key.padEnd(width)}  ${value}`);
    for (const warning of result.warnings as string[]) console.log(`  peringatan: ${warning}`);
}

async function run(args: string[]): Promise<void> {
    const { values, positionals } = parseToolArgs(args, {
        json: { type: 'boolean' },
        help: { type: 'boolean', short: 'h' },
    });
    if (values.help) {
        help();
        return;
    }
    if (!positionals.length) throw new UsageError('Sebutkan minimal satu file gambar.');

    const results: FileResult[] = [];
    for (const path of positionals) results.push(await inspectFile(path));
    const failed = results.filter((result) => !result.ok);

    if (values.json) printJson({ files: results });
    else {
        const good = results.filter((result) => result.ok);
        good.forEach((result, index) => {
            if (index) console.log('');
            printSection(result);
        });
    }
    for (const result of failed) console.error(result.error);
    if (failed.length) process.exitCode = 1;
}

export const imageTool: Tool = {
    name: 'image',
    description: 'Metadata gambar (PNG, JPEG, GIF, WebP): ukuran, rasio, EXIF, frame',
    category: 'Media',
    run,
};
