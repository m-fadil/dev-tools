import { UsageError } from '../../errors.js';
import { choice, integer, parseToolArgs, printJson } from '../../io.js';
import { LOCALES } from './data.js';
import { make, TYPES } from './generate.js';
import { rngFromSeed } from './rng.js';
const MAX_SPAN = 4_294_967_295;
const YEAR_MS = 365.25 * 86_400_000;
function help() {
    console.log(`dt fake <jenis> [options]

Buat data acak untuk pengujian (lihat catatan telepon; bukan untuk keamanan).
Jenis: ${TYPES.join(', ')}
  -c, --count <n>      Jumlah hasil, 1-1000 (default: 1), satu per baris
      --locale <l>     id atau en (default: en)
      --seed <n>       Seed bilangan bulat 0-4294967295 agar hasil dapat diulang
      --from <tanggal> Batas awal date/datetime, ISO 8601 tanpa pecahan detik (default: 10 tahun lalu)
      --to <tanggal>   Batas akhir date/datetime, ISO 8601 tanpa pecahan detik (default: sekarang)
      --min <n>        Batas bawah number (default: 0)
      --max <n>        Batas atas number (default: 1000, atau --min bila lebih besar);
                       selisih --max - --min maks. 4294967295
      --length <n>     Panjang hex dalam karakter, 1-1024 (default: 16)
      --json           Keluaran JSON: {type, locale, values: []}
  -h, --help           Tampilkan bantuan

Email memakai example.com/.org/.net, IP dari rentang dokumentasi (RFC 5737/3849),
telepon en dari rentang fiktif NANP 555-01xx; telepon id hanya berformat mirip
nomor seluler dan bisa milik pelanggan nyata (jangan dipakai untuk menghubungi siapa pun). Tanpa jenis, daftar jenis ditampilkan.
Tanpa --seed hasil berbeda tiap eksekusi; default --from/--to bergantung pada waktu sekarang.
Contoh: dt fake name -c 5 --locale id   |   dt fake number --min 10 --max 20 --seed 7`);
}
/** Strict ISO 8601 date or date-time without fractional seconds; date-only and zone-less values are UTC. */
function parseDate(value, option) {
    const m = /^(\d{4})-(\d{2})-(\d{2})(?:[T ](\d{2}):(\d{2})(?::(\d{2}))?(Z|[+-]\d{2}:\d{2})?)?$/.exec(value);
    const ms = m
        ? Date.parse(`${m[1]}-${m[2]}-${m[3]}T${m[4] ?? '00'}:${m[5] ?? '00'}:${m[6] ?? '00'}${m[7] ?? 'Z'}`)
        : NaN;
    // Date.parse may roll Feb 30 over; verify the calendar day survives a round trip
    const day = m ? new Date(Date.UTC(Number(m[1]), Number(m[2]) - 1, Number(m[3]))) : undefined;
    const valid = m !== null && !Number.isNaN(ms) && day?.getUTCDate() === Number(m[3]) && Number(m[4] ?? 0) < 24;
    if (!valid)
        throw new UsageError(`Nilai --${option} bukan tanggal ISO 8601 yang valid: ${value}`);
    return ms;
}
function signedInt(value, fallback, option) {
    if (value === undefined)
        return fallback;
    const n = /^-?\d+$/.test(value) ? Number(value) : NaN;
    if (!Number.isSafeInteger(n))
        throw new UsageError(`Nilai --${option} harus bilangan bulat: ${value}`);
    return n;
}
async function run(args) {
    const { values, positionals } = parseToolArgs(args, {
        count: { type: 'string', short: 'c' },
        locale: { type: 'string' },
        seed: { type: 'string' },
        from: { type: 'string' },
        to: { type: 'string' },
        min: { type: 'string' },
        max: { type: 'string' },
        length: { type: 'string' },
        json: { type: 'boolean' },
        help: { type: 'boolean', short: 'h' },
    });
    if (values.help) {
        help();
        return;
    }
    if (positionals.length === 0) {
        console.log(`Jenis yang tersedia:\n${TYPES.map((t) => `  ${t}`).join('\n')}`);
        return;
    }
    if (positionals.length > 1)
        throw new UsageError(`Argumen tidak dikenal: ${positionals[1]}`);
    const type = positionals[0];
    if (!TYPES.includes(type))
        throw new UsageError(`Jenis tidak dikenal: ${type}. Pilihan: ${TYPES.join(', ')}`);
    const only = (names, types) => {
        for (const n of names) {
            if (values[n] !== undefined && !types.includes(type)) {
                throw new UsageError(`--${n} hanya berlaku untuk jenis: ${types.join(', ')}`);
            }
        }
    };
    only(['from', 'to'], ['date', 'datetime']);
    only(['min', 'max'], ['number']);
    only(['length'], ['hex']);
    const count = integer(values.count, 1, 1, 1000, 'count');
    const locale = choice(values.locale, LOCALES, 'en', 'locale');
    const now = Date.now();
    const from = values.from === undefined ? now - 10 * YEAR_MS : parseDate(values.from, 'from');
    const to = values.to === undefined ? now : parseDate(values.to, 'to');
    if (from > to)
        throw new UsageError('--from tidak boleh setelah --to.');
    const min = signedInt(values.min, 0, 'min');
    const max = signedInt(values.max, Math.max(1000, min), 'max');
    if (min > max)
        throw new UsageError('--min tidak boleh lebih besar dari --max.');
    // the rng is 32-bit: wider spans would silently lose precision
    if (max - min > MAX_SPAN)
        throw new UsageError(`Selisih --max dan --min maksimal ${MAX_SPAN} (2^32-1).`);
    const params = {
        rng: rngFromSeed(values.seed),
        locale,
        from,
        to,
        min,
        max,
        length: integer(values.length, 16, 1, 1024, 'length'),
    };
    const out = Array.from({ length: count }, () => make(type, params));
    if (values.json)
        printJson({ type, locale, values: out });
    else
        console.log(out.join('\n'));
}
export const fakeTool = {
    name: 'fake',
    description: 'Buat data palsu (nama, email, alamat, IP, tanggal, dll.) untuk pengujian',
    category: 'Utilitas',
    run,
};
//# sourceMappingURL=index.js.map