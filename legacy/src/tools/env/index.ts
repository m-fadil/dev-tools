import { readFile } from 'node:fs/promises';
import { UsageError } from '../../errors.js';
import { parseToolArgs, printJson } from '../../io.js';
import type { Tool } from '../../tool.js';
import { diffEnv, parseEnv, type EnvParseResult } from './envfile.js';

function help(): void {
    console.log(`dt env [options] <a> <b>

Bandingkan dua file .env: kunci yang hanya ada di satu sisi dan nilai yang berbeda.
  -s, --show-values  Tampilkan nilai (default: nilai disamarkan, hanya nama kunci)
  -k, --keys-only    Abaikan perbedaan nilai, bandingkan kunci saja
      --json         Keluaran satu dokumen JSON (onlyA, onlyB, different, same, invalidLines)
  -h, --help         Tampilkan bantuan

Format: KEY=VALUE, awalan export, komentar #, nilai '...' atau "..." (\\n dan \\" didukung).
Tanpa interpolasi variabel (\${VAR} dibaca apa adanya). Kunci duplikat: yang terakhir dipakai (peringatan di stderr).
Baris tidak valid dilaporkan di stderr dengan nomor baris.
Exit: 0 tidak ada perbedaan, 1 ada perbedaan atau baris tidak valid, 2 penggunaan salah.
Contoh: dt env .env .env.example   |   dt env -k .env .env.production`);
}

async function load(path: string, label: string): Promise<EnvParseResult> {
    let content: string;
    try {
        content = await readFile(path, 'utf8');
    } catch (error) {
        throw new Error(`Tidak bisa membaca ${path}: ${(error as Error).message}`, { cause: error });
    }
    const result = parseEnv(content);
    for (const { key, line, previous } of result.duplicates) {
        console.error(`${label}:${line}: peringatan: ${key} duplikat (baris ${previous}), nilai terakhir dipakai`);
    }
    for (const line of result.invalid) console.error(`${label}:${line}: baris tidak valid`);
    return result;
}

const show = (value: string | undefined) => JSON.stringify(value).slice(1, -1);

async function run(args: string[]): Promise<void> {
    const { values, positionals } = parseToolArgs(args, {
        'show-values': { type: 'boolean', short: 's' },
        'keys-only': { type: 'boolean', short: 'k' },
        json: { type: 'boolean' },
        help: { type: 'boolean', short: 'h' },
    });
    if (values.help) {
        help();
        return;
    }
    const [pathA, pathB] = positionals;
    if (pathA === undefined || pathB === undefined || positionals.length !== 2)
        throw new UsageError('Berikan tepat dua file: dt env <a> <b>');
    const a = await load(pathA, 'a');
    const b = await load(pathB, 'b');
    const invalid = a.invalid.length + b.invalid.length;
    const diff = diffEnv(a.values, b.values, Boolean(values['keys-only']));

    if (values.json) {
        printJson({
            onlyA: diff.onlyA,
            onlyB: diff.onlyB,
            different: diff.changed.map((key) =>
                values['show-values'] ? { key, a: a.values.get(key), b: b.values.get(key) } : { key },
            ),
            same: diff.same,
            invalidLines: [
                ...a.invalid.map((line) => ({ file: pathA, line })),
                ...b.invalid.map((line) => ({ file: pathB, line })),
            ],
        });
        if (diff.onlyA.length || diff.onlyB.length || diff.changed.length || invalid) process.exitCode = 1;
        return;
    }
    if (diff.onlyA.length) console.log(`Hanya di a:\n${diff.onlyA.map((key) => `  ${key}`).join('\n')}`);
    if (diff.onlyB.length) console.log(`Hanya di b:\n${diff.onlyB.map((key) => `  ${key}`).join('\n')}`);
    if (diff.changed.length) {
        const rows = diff.changed.map((key) =>
            values['show-values'] ? `  ${key}: ${show(a.values.get(key))} -> ${show(b.values.get(key))}` : `  ${key}`,
        );
        console.log(`Beda nilai:\n${rows.join('\n')}`);
    }
    console.log(
        `${diff.same} sama, ${diff.changed.length} beda, ${diff.onlyA.length} hanya a, ${diff.onlyB.length} hanya b`,
    );
    if (invalid) console.error(`${invalid} baris tidak valid`);
    if (diff.onlyA.length || diff.onlyB.length || diff.changed.length || invalid) process.exitCode = 1;
}

export const envTool: Tool = {
    name: 'env',
    description: 'Bandingkan dua file .env (kunci dan nilai, nilai disamarkan)',
    category: 'Data',
    run,
};
