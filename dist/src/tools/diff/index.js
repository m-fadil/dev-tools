import { createHash } from 'node:crypto';
import { createReadStream } from 'node:fs';
import { open, readFile } from 'node:fs/promises';
import { UsageError } from '../../errors.js';
import { integer, parseToolArgs, printJson } from '../../io.js';
import { buildHunks, diffLines, formatUnified, splitLines } from './lines.js';
const SNIFF_BYTES = 8000;
const MAX_TEXT_DIFF_BYTES = 5 * 1024 * 1024;
const MAX_LINE_PRODUCT = 25_000_000;
const NOTE = 'Gunakan `git diff --no-index <a> <b>` untuk diff yang lebih besar.';
function help() {
    console.log(`dt diff [options] <a> <b>

Bandingkan dua file. Identik: cetak sha256 dan ukuran. Berbeda: cetak ukuran dan
sha256 keduanya, lalu diff baris (teks) atau offset byte pertama yang berbeda (biner).
  -U, --context <n>  Baris konteks di sekitar perubahan, 0-20 (default: 3)
      --json         Keluaran satu dokumen JSON (identical, a, b, kind, firstDifferingByte, hunks)
  -h, --help         Tampilkan bantuan

File dianggap biner jika ada byte NUL di 8000 byte pertama. Diff baris dilewati
(hanya ringkasan) jika file > 5 MiB atau jumlah baris a x b > 25.000.000.
Exit: 0 identik, 1 berbeda ATAU error (error ditulis ke stderr, hasil beda ke stdout), 2 penggunaan salah.
Contoh: dt diff -U 1 lama.txt baru.txt`);
}
async function summarize(path) {
    const hash = createHash('sha256');
    const heads = [];
    let headLen = 0;
    let size = 0;
    try {
        for await (const chunk of createReadStream(path)) {
            hash.update(chunk);
            size += chunk.length;
            if (headLen < SNIFF_BYTES) {
                const part = chunk.subarray(0, SNIFF_BYTES - headLen);
                heads.push(part);
                headLen += part.length;
            }
        }
    }
    catch (error) {
        const code = error.code;
        if (code === 'ENOENT')
            throw new Error(`File tidak ditemukan: ${path}`, { cause: error });
        if (code === 'EISDIR')
            throw new Error(`Bukan file (direktori): ${path}`, { cause: error });
        throw new Error(`Gagal membaca ${path}: ${error.message}`, { cause: error });
    }
    return { path, size, sha256: hash.digest('hex'), head: Buffer.concat(heads) };
}
/** Offset of the first differing byte, streaming both files in blocks. */
async function firstDifference(pathA, pathB) {
    const [fa, fb] = await Promise.all([open(pathA, 'r'), open(pathB, 'r')]);
    try {
        const bufA = Buffer.allocUnsafe(65536);
        const bufB = Buffer.allocUnsafe(65536);
        let offset = 0;
        for (;;) {
            const [ra, rb] = await Promise.all([
                fa.read(bufA, 0, bufA.length, offset),
                fb.read(bufB, 0, bufB.length, offset),
            ]);
            const n = Math.min(ra.bytesRead, rb.bytesRead);
            for (let i = 0; i < n; i++) {
                if (bufA[i] !== bufB[i])
                    return offset + i;
            }
            // Short reads on regular files only happen at EOF; the shorter file ends here.
            if (ra.bytesRead !== rb.bytesRead || n === 0)
                return offset + n;
            offset += n;
        }
    }
    finally {
        await Promise.all([fa.close(), fb.close()]);
    }
}
const isText = (head) => !head.includes(0);
function describe(label, s) {
    return `  ${label}: ${s.path}  ${s.size} byte  sha256 ${s.sha256}`;
}
async function run(args) {
    const { values, positionals } = parseToolArgs(args, {
        context: { type: 'string', short: 'U' },
        json: { type: 'boolean' },
        help: { type: 'boolean', short: 'h' },
    });
    if (values.help) {
        help();
        return;
    }
    if (positionals.length !== 2)
        throw new UsageError('Butuh tepat dua file: dt diff <a> <b>');
    const context = integer(values.context, 3, 0, 20, 'context');
    const [pathA, pathB] = positionals;
    const a = await summarize(pathA);
    const b = await summarize(pathB);
    const json = Boolean(values.json);
    const file = (s) => ({ path: s.path, size: s.size, sha256: s.sha256 });
    const emit = (doc) => printJson({ ...doc, a: file(a), b: file(b) });
    if (a.sha256 === b.sha256 && a.size === b.size) {
        if (json)
            emit({ identical: true, kind: 'identical' });
        else
            console.log(`Identik (sha256 ${a.sha256}, ${a.size} byte)`);
        return;
    }
    process.exitCode = 1;
    if (!json) {
        console.log('Berbeda');
        console.log(describe('a', a));
        console.log(describe('b', b));
    }
    if (!isText(a.head) || !isText(b.head)) {
        const offset = await firstDifference(pathA, pathB);
        if (json)
            emit({ identical: false, kind: 'binary', firstDifferingByte: offset });
        else
            console.log(`Biner, berbeda di byte offset ${offset}`);
        return;
    }
    if (a.size > MAX_TEXT_DIFF_BYTES || b.size > MAX_TEXT_DIFF_BYTES) {
        if (json)
            emit({ identical: false, kind: 'summary-only' });
        else
            console.log(`File lebih dari 5 MiB, diff baris dilewati. ${NOTE}`);
        return;
    }
    const linesA = splitLines(await readFile(pathA));
    const linesB = splitLines(await readFile(pathB));
    if (linesA.length * linesB.length > MAX_LINE_PRODUCT) {
        if (json)
            emit({ identical: false, kind: 'summary-only' });
        else
            console.log(`Terlalu banyak baris (${linesA.length} x ${linesB.length}), diff baris dilewati. ${NOTE}`);
        return;
    }
    const hunks = buildHunks(diffLines(linesA, linesB), context);
    if (json) {
        emit({
            identical: false,
            kind: 'text',
            hunks: hunks.map(({ noEol: _noEol, ...hunk }) => hunk),
        });
        return;
    }
    console.log(`--- ${pathA}`);
    console.log(`+++ ${pathB}`);
    const body = formatUnified(hunks);
    if (body)
        console.log(body);
}
export const diffTool = {
    name: 'diff',
    description: 'Bandingkan dua file (sha256, diff baris, atau offset byte beda)',
    category: 'Data',
    run,
};
//# sourceMappingURL=index.js.map