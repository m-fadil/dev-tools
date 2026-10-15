import { readFile } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';
import type { Tool } from '../../tool.js';
import { UsageError } from '../../errors.js';
import { choice, parseToolArgs, printJson } from '../../io.js';
import { HASH_ALGORITHMS } from '../hash/hash.js';
import { HashFileError, digestLength, digestsEqual, hashFile, parseSums } from './checksum.js';

function help(): void {
    console.log(`dt checksum [options] <file...>

Cetak checksum tiap file dengan format sha256sum: "<hex>  <path>".
  -a, --algorithm <nama>  ${HASH_ALGORITHMS.join(', ')} (default: sha256)
  -c, --check <sums>      Verifikasi file di daftar checksum (tanpa argumen file)
      --json              Keluaran JSON tunggal ke stdout (ringkasan tetap ke stderr)
  -h, --help              Tampilkan bantuan

File dibaca streaming, aman untuk file besar. Direktori dan file non-reguler (FIFO, device) ditolak.
Mode --check: baris "<hex>  <path>" atau "<hex> *<path>"; baris kosong dan
"# komentar" diabaikan. Path relatif terhadap lokasi file daftar. Panjang digest
harus cocok dengan --algorithm, kalau tidak baris dianggap tidak valid.
Hasil per file: OK, GAGAL, TIDAK ADA, atau ERROR (alasan); ringkasan ke stderr.
Exit: 0 semua cocok, 1 ada yang gagal/tidak ada/tidak valid atau error, 2 penggunaan salah.
Contoh: dt checksum a.bin b.bin > SUMS   |   dt checksum -c SUMS`);
}

async function check(sumsFile: string, algorithm: (typeof HASH_ALGORITHMS)[number], json: boolean): Promise<void> {
    let raw: string;
    try {
        raw = await readFile(sumsFile, 'utf8');
    } catch (error) {
        const e = error as NodeJS.ErrnoException;
        throw new Error(`${sumsFile}: tidak dapat dibaca (${e.code ?? e.message})`, { cause: error });
    }
    const text = raw.replace(/^\uFEFF/, '');
    const { entries, invalid } = parseSums(text, digestLength(algorithm));
    for (const line of invalid) console.error(`${sumsFile}: baris ${line} tidak valid`);
    if (!entries.length && !invalid.length) throw new Error(`Tidak ada baris checksum di ${sumsFile}`);

    type Status = 'ok' | 'failed' | 'missing' | 'error';
    const base = dirname(resolve(sumsFile));
    let failed = 0;
    let missing = 0;
    let errors = 0;
    const results: { path: string; status: Status; error?: string }[] = [];
    const say = (path: string, status: Status, label: string, reason?: string): void => {
        results.push(reason === undefined ? { path, status } : { path, status, error: reason });
        if (!json) console.log(`${path}: ${label}`);
    };
    for (const entry of entries) {
        let actual: string;
        try {
            actual = await hashFile(resolve(base, entry.path), algorithm);
        } catch (error) {
            if (error instanceof HashFileError && error.code === 'ENOENT') {
                say(entry.path, 'missing', 'TIDAK ADA');
                missing++;
                continue;
            }
            const reason = error instanceof HashFileError ? error.reason : (error as Error).message;
            say(entry.path, 'error', `ERROR (${reason})`, reason);
            errors++;
            continue;
        }
        if (digestsEqual(actual, entry.digest)) {
            say(entry.path, 'ok', 'OK');
        } else {
            say(entry.path, 'failed', 'GAGAL');
            failed++;
        }
    }
    const ok = entries.length - failed - missing - errors;
    console.error(
        `checksum: ${ok} OK, ${failed} GAGAL, ${missing} TIDAK ADA, ${errors} ERROR, ${invalid.length} baris tidak valid`,
    );
    const passed = !(failed || missing || errors || invalid.length);
    if (json) printJson({ algorithm, results, malformed: invalid, ok, failed, missing, errors, passed });
    if (!passed) process.exitCode = 1;
}

async function run(args: string[]): Promise<void> {
    const { values, positionals } = parseToolArgs(args, {
        algorithm: { type: 'string', short: 'a' },
        check: { type: 'string', short: 'c' },
        json: { type: 'boolean' },
        help: { type: 'boolean', short: 'h' },
    });
    if (values.help) {
        help();
        return;
    }
    const algorithm = choice(values.algorithm, HASH_ALGORITHMS, 'sha256', 'algorithm');
    if (values.check !== undefined) {
        if (positionals.length) throw new UsageError('--check tidak bisa dipakai bersama argumen file.');
        await check(values.check, algorithm, values.json ?? false);
        return;
    }
    if (!positionals.length) throw new UsageError('Butuh minimal satu file. Lihat: dt checksum --help');
    const files: { path: string; digest: string }[] = [];
    const errors: { path: string; message: string }[] = [];
    for (const path of positionals) {
        try {
            const digest = await hashFile(path, algorithm);
            if (values.json) files.push({ path, digest });
            else console.log(`${digest}  ${path}`);
        } catch (error) {
            const message = error instanceof HashFileError ? error.reason : (error as Error).message;
            errors.push({ path, message });
            console.error(`${path}: ${message}`);
        }
    }
    if (values.json) printJson({ algorithm, files, errors });
    if (errors.length) process.exitCode = 1;
}

export const checksumTool: Tool = {
    name: 'checksum',
    description: 'Hitung dan verifikasi checksum file (format sha256sum)',
    category: 'Keamanan & Kripto',
    aliases: ['sum'],
    run,
};
