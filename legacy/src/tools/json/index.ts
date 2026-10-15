import type { Tool } from '../../tool.js';
import { UsageError } from '../../errors.js';
import { integer, parseToolArgs, readInput } from '../../io.js';
import { getPath, JsonTooDeepError, MAX_DEPTH, parseJson, parseQuery, sortKeys, stringifyJson } from './json.js';

function help(): void {
    console.log(`dt json [options] [teks]

Format, validasi, atau query JSON dari teks, file, atau stdin. Tanpa teks/--file/stdin: prompt (terminal).
  -m, --minify         Satu baris tanpa spasi (tidak bisa dengan --indent)
      --indent <n>     Lebar indentasi 0-8 (default: 2)
  -s, --sort-keys      Urutkan key objek secara rekursif
      --check          Validasi saja: cetak "valid" (exit 0) atau error baris:kolom (exit 1)
  -q, --query <path>   Ambil nilai, mis. a.b[0].c, $.a, ["key.dengan.titik"]
  -r, --raw            Dengan --query: cetak string tanpa tanda kutip
  -f, --file <path>    Baca JSON dari file
  -h, --help           Tampilkan bantuan

Exit: 0 sukses, 1 JSON tidak valid/terlalu dalam atau path tidak ditemukan, 2 penggunaan salah.
Angka bulat di atas 2^53 kehilangan presisi (JSON.parse); key bilangan bulat selalu diurutkan lebih dulu oleh JavaScript.
Nesting di atas ${MAX_DEPTH} level ditolak (exit 1). Indeks query tanpa nol di depan dan maks 2^53-1.
Contoh: dt json -s -f data.json   |   curl -s api | dt json -q 'items[0].id' -r`);
}

async function run(args: string[]): Promise<void> {
    const { values, positionals } = parseToolArgs(args, {
        minify: { type: 'boolean', short: 'm' },
        indent: { type: 'string' },
        'sort-keys': { type: 'boolean', short: 's' },
        check: { type: 'boolean' },
        query: { type: 'string', short: 'q' },
        raw: { type: 'boolean', short: 'r' },
        file: { type: 'string', short: 'f' },
        help: { type: 'boolean', short: 'h' },
    });
    if (values.help) {
        help();
        return;
    }
    const indent = integer(values.indent, 2, 0, 8, 'indent');
    if (values.minify && values.indent !== undefined) throw new UsageError('--minify tidak bisa dengan --indent.');
    if (values.raw && values.query === undefined) throw new UsageError('--raw hanya berlaku dengan --query.');
    if (values.check) {
        const conflict = (['minify', 'query', 'raw', 'sort-keys', 'indent'] as const).find(
            (name) => values[name] !== undefined && values[name] !== false,
        );
        if (conflict) throw new UsageError(`--check tidak bisa dengan --${conflict}.`);
    }
    const segments = values.query === undefined ? undefined : parseQuery(values.query);

    const text = (await readInput(positionals, values.file, 'JSON')).toString('utf8');
    let data: unknown;
    try {
        data = parseJson(text);
    } catch (error) {
        if (error instanceof JsonTooDeepError) throw error;
        throw new Error(`invalid: ${(error as Error).message}`, { cause: error });
    }
    if (values.check) {
        console.log('valid');
        return;
    }
    if (values['sort-keys']) data = sortKeys(data);
    if (segments) {
        const result = getPath(data, segments);
        if (!result.found) throw new Error(`path tidak ditemukan: ${values.query}`);
        data = result.value;
        if (values.raw && typeof data === 'string') {
            console.log(data);
            return;
        }
    }
    console.log(stringifyJson(data, values.minify ? undefined : indent));
}

export const jsonTool: Tool = {
    name: 'json',
    description: 'Format, minify, urutkan key, validasi, atau query JSON',
    category: 'Data',
    run,
};
