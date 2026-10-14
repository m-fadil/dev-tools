import { UsageError } from '../../errors.js';
import { parseToolArgs, printJson, readInput } from '../../io.js';
import { formatUrl, parseUrl } from './url.js';
function help() {
    console.log(`dt url [options] [url]

Uraikan URL absolut (WHATWG URL) menjadi komponen dan parameter query.
Input dari argumen, --file, atau stdin (spasi di tepi dibuang).
  -q, --query <key>       Cetak hanya nilai terdekode dari key itu, satu per baris
      --json              Cetak seluruh struktur sebagai JSON (query: array [key, value])
      --show-password     Tampilkan password (default: disamarkan ***)
      --file <path>       Baca URL dari berkas
  -h, --help              Tampilkan bantuan

Query ditampilkan terdekode, urutan asli; key berulang menjadi baris terpisah.
Port tanpa nilai eksplisit ditampilkan "(default n)" untuk http, https, ftp, ws, wss.
Skema wajib ada; URL relatif tidak diselesaikan. --json tidak bisa dengan --query.
Exit: 0 sukses, 1 URL tidak valid atau key tidak ada, 2 penggunaan salah.
Contoh: dt url "https://a.com/p?x=1&x=2#h"   |   dt url -q x "https://a.com/?x=1&x=2"`);
}
async function run(args) {
    const { values, positionals } = parseToolArgs(args, {
        query: { type: 'string', short: 'q' },
        json: { type: 'boolean' },
        'show-password': { type: 'boolean' },
        file: { type: 'string' },
        help: { type: 'boolean', short: 'h' },
    });
    if (values.help) {
        help();
        return;
    }
    if (values.json && values.query !== undefined)
        throw new UsageError('--json tidak bisa digabung dengan --query.');
    const text = (await readInput(positionals, values.file, 'URL:')).toString('utf8').trim();
    const parsed = parseUrl(text, values['show-password'] ?? false);
    if (values.query !== undefined) {
        const found = parsed.query.filter(([key]) => key === values.query).map(([, value]) => value);
        if (!found.length)
            throw new Error(`Key query tidak ditemukan: ${values.query}`);
        console.log(found.join('\n'));
        return;
    }
    if (values.json)
        printJson(parsed);
    else
        console.log(formatUrl(parsed));
}
export const urlTool = {
    name: 'url',
    description: 'Uraikan URL menjadi komponen dan parameter query',
    category: 'Encoding',
    run,
};
//# sourceMappingURL=index.js.map