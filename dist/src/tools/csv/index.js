import { UsageError } from '../../errors.js';
import { choice, integer, parseToolArgs, printJson, readInput } from '../../io.js';
import { buildTable, findDuplicate, resolveSelect } from './data.js';
import { detectDelimiter, parseCsv, resolveDelimiter, toCsvLine } from './parse.js';
import { renderTable } from './table.js';
const FORMATS = ['table', 'json', 'tsv', 'csv'];
/** Ragged warnings printed before collapsing into a count. */
const MAX_WARNINGS = 20;
function help() {
    console.log(`dt csv [options] [teks]

Baca CSV (RFC 4180) dan tampilkan sebagai tabel, JSON, TSV, atau CSV.
Input dari argumen, --file, atau stdin. Satu BOM awal dibuang; CRLF/LF diterima.
  -d, --delimiter <d>     comma | semicolon | tab | pipe | satu karakter (default: comma)
      --detect            Tebak delimiter dari rekaman pertama; seri/tidak ada = koma (tidak dengan -d)
      --no-header         Baris pertama adalah data; kolom diberi nama c1..cn
  -F, --format <fmt>      table | json | tsv | csv (default: table)
      --max-width <n>     Lebar maks. sel pada tabel, 1-1000 (default: 40), lebihnya dipotong …
      --select <kolom,..> Pilih kolom menurut nama header atau nomor (mulai 1)
      --limit <n>         Tampilkan hanya n baris data pertama
      --stats             Cetak jumlah baris/kolom dan sel kosong per kolom
      --strict            Baris dengan jumlah kolom berbeda = error (default: peringatan)
      --json              Sama dengan -F json
      --file <path>       Baca CSV dari berkas
  -h, --help              Tampilkan bantuan

JSON: array objek (nilai selalu string), atau array array dengan --no-header.
Baris kosong dilewati. Baris tidak rata diberi peringatan di stderr (nomor baris
= urutan rekaman, header = 1) dan dilengkapi sel kosong. Keluaran csv selalu
ber-delimiter koma; tsv me-escape tab/newline sebagai \\t \\n \\r \\\\.
Exit: 0 sukses, 1 CSV tidak valid / kolom tidak ada / --strict dilanggar, 2 penggunaan salah.
Contoh: dt csv --detect --file data.csv   |   printf 'a,b\\n1,2\\n' | dt csv --json`);
}
function tsvCell(cell) {
    return cell.replaceAll('\\', '\\\\').replaceAll('\t', '\\t').replaceAll('\n', '\\n').replaceAll('\r', '\\r');
}
async function run(args) {
    const { values, positionals } = parseToolArgs(args, {
        delimiter: { type: 'string', short: 'd' },
        detect: { type: 'boolean' },
        'no-header': { type: 'boolean' },
        format: { type: 'string', short: 'F' },
        'max-width': { type: 'string' },
        select: { type: 'string' },
        limit: { type: 'string' },
        stats: { type: 'boolean' },
        strict: { type: 'boolean' },
        json: { type: 'boolean' },
        file: { type: 'string' },
        help: { type: 'boolean', short: 'h' },
    });
    if (values.help) {
        help();
        return;
    }
    if (values.detect && values.delimiter !== undefined)
        throw new UsageError('--detect tidak bisa digabung dengan --delimiter.');
    let format = choice(values.format, FORMATS, 'table', 'format');
    if (values.json) {
        if (values.format !== undefined && format !== 'json') {
            throw new UsageError(`--json bertentangan dengan --format ${format}.`);
        }
        format = 'json';
    }
    let delimiter = ',';
    if (values.delimiter !== undefined) {
        const resolved = resolveDelimiter(values.delimiter);
        if (resolved === undefined) {
            throw new UsageError(`Nilai --delimiter tidak valid: ${JSON.stringify(values.delimiter)}. Pakai comma, semicolon, tab, pipe, atau satu karakter selain " dan newline.`);
        }
        delimiter = resolved;
    }
    const maxWidth = integer(values['max-width'], 40, 1, 1000, 'max-width');
    const limit = values.limit === undefined ? undefined : integer(values.limit, 0, 1, Number.MAX_SAFE_INTEGER, 'limit');
    if (values.select !== undefined && !values.select.trim())
        throw new UsageError('--select tidak boleh kosong.');
    const text = (await readInput(positionals, values.file, 'CSV:')).toString('utf8');
    if (values.detect) {
        delimiter = detectDelimiter(text);
        console.error(`Delimiter terdeteksi: ${delimiter === '\t' ? 'tab' : delimiter}`);
    }
    const hasHeader = !values['no-header'];
    const table = buildTable(parseCsv(text, delimiter), hasHeader);
    if (table.ragged.length) {
        const [firstBad] = table.ragged;
        if (values.strict) {
            throw new Error(`Baris ${firstBad.row} punya ${firstBad.cols} kolom, diharapkan ${table.expected} (--strict).`);
        }
        for (const r of table.ragged.slice(0, MAX_WARNINGS)) {
            console.error(`peringatan: baris ${r.row} punya ${r.cols} kolom, diharapkan ${table.expected}`);
        }
        if (table.ragged.length > MAX_WARNINGS) {
            console.error(`peringatan: ${table.ragged.length - MAX_WARNINGS} baris tidak rata lainnya`);
        }
    }
    let names = table.names;
    let rows = table.rows;
    if (values.select !== undefined) {
        const picked = resolveSelect(values.select, names);
        names = picked.map((i) => names[i]);
        rows = rows.map((r) => picked.map((i) => r[i]));
    }
    if (values.stats) {
        const empty = names.map((_, c) => rows.filter((r) => r[c] === '').length);
        if (format === 'json') {
            printJson({
                rows: rows.length,
                columns: names.length,
                ragged: table.ragged.map((r) => r.row),
                columnStats: names.map((name, index) => ({ index: index + 1, name, empty: empty[index] })),
            });
        }
        else {
            const lines = [`Baris data: ${rows.length}`, `Kolom: ${names.length}`];
            if (table.ragged.length)
                lines.push(`Baris tidak rata: ${table.ragged.map((r) => r.row).join(', ')}`);
            lines.push('Sel kosong per kolom:');
            for (let i = 0; i < names.length; i++) {
                lines.push(`  ${i + 1} ${JSON.stringify(names[i])}: ${empty[i]}`);
            }
            console.log(lines.join('\n'));
        }
        return;
    }
    if (limit !== undefined)
        rows = rows.slice(0, limit);
    switch (format) {
        case 'json': {
            if (!hasHeader) {
                printJson(rows);
                break;
            }
            const dup = findDuplicate(names);
            if (dup)
                throw new Error(`Header duplikat ${dup}; JSON objek tidak bisa dibuat. Pakai --select atau --no-header.`);
            printJson(rows.map((r) => Object.fromEntries(names.map((name, i) => [name, r[i]]))));
            break;
        }
        case 'table':
            if (names.length)
                console.log(renderTable(names, rows, maxWidth));
            break;
        case 'tsv':
        case 'csv': {
            const line = format === 'tsv' ? (r) => r.map(tsvCell).join('\t') : (r) => toCsvLine(r, ',');
            const out = (hasHeader ? [names, ...rows] : rows).map(line);
            if (out.length)
                console.log(out.join('\n'));
            break;
        }
    }
}
export const csvTool = {
    name: 'csv',
    description: 'Baca CSV: tabel, JSON, TSV, pilih kolom, statistik',
    category: 'Data',
    run,
};
//# sourceMappingURL=index.js.map