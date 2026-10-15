import { domainToASCII } from 'node:url';
import { UsageError } from '../../errors.js';
import { choice, integer, parseToolArgs, printJson } from '../../io.js';
import type { Tool } from '../../tool.js';
import { LOCALES } from './data.js';
import { make, TYPES, type FakeType, type Params, type Value } from './generate.js';
import { rngFromSeed } from './rng.js';

const MAX_SPAN = 4_294_967_295;
const YEAR_MS = 365.25 * 86_400_000;

const specialHelp: { types: FakeType[]; options: string[] }[] = [
    { types: ['email'], options: ['--domain <domain>  default: example.com'] },
    {
        types: ['date', 'datetime'],
        options: ['--from <tanggal>   batas awal (ISO 8601)', '--to <tanggal>     batas akhir (ISO 8601)'],
    },
    { types: ['number'], options: ['--min <n>          default: 0', '--max <n>          default: 1000'] },
    { types: ['hex'], options: ['--length <n>       default: 16'] },
];

function help(type?: FakeType): void {
    const groups = specialHelp.filter((group) => type === undefined || group.types.includes(type));
    const special = groups.length
        ? `\nOpsi khusus:\n${groups
              .map(
                  (group) =>
                      `  ${type ?? group.types.join(', ')}\n${group.options.map((line) => `    ${line}`).join('\n')}`,
              )
              .join('\n')}\n`
        : '';
    const phone =
        type === undefined || type === 'phone'
            ? '\nTelepon id hanya mirip format nomor asli; jangan dipakai untuk menghubungi siapa pun.\n'
            : '';
    console.log(`dt fake ${type ?? '<jenis>'} [options]
${type === undefined ? `\nJenis: ${TYPES.join(', ')}\n` : ''}
Opsi umum:
  -c, --count <n>   jumlah hasil, 1-1000 (default: 1)
      --locale <l>  id atau en (default: en)
      --seed <n>    hasil bisa diulang
      --json        keluaran JSON
  -h, --help        bantuan
${special}${phone}
Contoh: ${type === 'email' ? 'dt fake email --domain perusahaan.test -c 5' : `dt fake ${type ?? 'name'} -c 5 --seed 7`}`);
}

/** Normalize a DNS email domain without accepting URL/userinfo syntax or querying DNS. */
function parseDomain(value: string): string {
    if (/[\s@:/\\?#%[\]]/u.test(value))
        throw new UsageError('--domain harus domain saja, tanpa @, URL, port, atau path.');
    const domain = domainToASCII(value).toLowerCase();
    const labels = domain.split('.');
    if (
        !domain ||
        domain.length > 253 ||
        labels.length < 2 ||
        labels.some((label) => label.length > 63 || !/^[a-z0-9](?:[a-z0-9-]*[a-z0-9])?$/.test(label))
    ) {
        throw new UsageError(`Domain email tidak valid: ${value}`);
    }
    return domain;
}

/** Strict ISO 8601 date or date-time without fractional seconds; date-only and zone-less values are UTC. */
function parseDate(value: string, option: string): number {
    const m = /^(\d{4})-(\d{2})-(\d{2})(?:[T ](\d{2}):(\d{2})(?::(\d{2}))?(Z|[+-]\d{2}:\d{2})?)?$/.exec(value);
    const ms = m
        ? Date.parse(`${m[1]}-${m[2]}-${m[3]}T${m[4] ?? '00'}:${m[5] ?? '00'}:${m[6] ?? '00'}${m[7] ?? 'Z'}`)
        : NaN;
    // Date.parse may roll Feb 30 over; verify the calendar day survives a round trip
    const day = m ? new Date(Date.UTC(Number(m[1]), Number(m[2]) - 1, Number(m[3]))) : undefined;
    const valid = m !== null && !Number.isNaN(ms) && day?.getUTCDate() === Number(m[3]) && Number(m[4] ?? 0) < 24;
    if (!valid) throw new UsageError(`Nilai --${option} bukan tanggal ISO 8601 yang valid: ${value}`);
    return ms;
}

function signedInt(value: string | undefined, fallback: number, option: string): number {
    if (value === undefined) return fallback;
    const n = /^-?\d+$/.test(value) ? Number(value) : NaN;
    if (!Number.isSafeInteger(n)) throw new UsageError(`Nilai --${option} harus bilangan bulat: ${value}`);
    return n;
}

async function run(args: string[]): Promise<void> {
    const { values, positionals } = parseToolArgs(args, {
        count: { type: 'string', short: 'c' },
        locale: { type: 'string' },
        seed: { type: 'string' },
        domain: { type: 'string' },
        from: { type: 'string' },
        to: { type: 'string' },
        min: { type: 'string' },
        max: { type: 'string' },
        length: { type: 'string' },
        json: { type: 'boolean' },
        help: { type: 'boolean', short: 'h' },
    });
    if (positionals.length > 1) throw new UsageError(`Argumen tidak dikenal: ${positionals[1]}`);
    const type = positionals[0] as FakeType | undefined;
    if (type !== undefined && !TYPES.includes(type)) {
        throw new UsageError(`Jenis tidak dikenal: ${type}. Pilihan: ${TYPES.join(', ')}`);
    }
    const only = (names: string[], types: FakeType[]) => {
        for (const n of names) {
            if (values[n as keyof typeof values] !== undefined && (type === undefined || !types.includes(type))) {
                throw new UsageError(`--${n} hanya berlaku untuk jenis: ${types.join(', ')}`);
            }
        }
    };
    only(['domain'], ['email']);
    only(['from', 'to'], ['date', 'datetime']);
    only(['min', 'max'], ['number']);
    only(['length'], ['hex']);
    if (values.help) {
        help(type);
        return;
    }
    if (type === undefined) {
        console.log(`Jenis yang tersedia:\n${TYPES.map((t) => `  ${t}`).join('\n')}`);
        return;
    }

    const count = integer(values.count, 1, 1, 1000, 'count');
    const locale = choice(values.locale, LOCALES, 'en', 'locale');
    const now = Date.now();
    const from = values.from === undefined ? now - 10 * YEAR_MS : parseDate(values.from, 'from');
    const to = values.to === undefined ? now : parseDate(values.to, 'to');
    if (from > to) throw new UsageError('--from tidak boleh setelah --to.');
    const min = signedInt(values.min, 0, 'min');
    const max = signedInt(values.max, Math.max(1000, min), 'max');
    if (min > max) throw new UsageError('--min tidak boleh lebih besar dari --max.');
    // the rng is 32-bit: wider spans would silently lose precision
    if (max - min > MAX_SPAN) throw new UsageError(`Selisih --max dan --min maksimal ${MAX_SPAN} (2^32-1).`);
    const params: Params = {
        rng: rngFromSeed(values.seed),
        locale,
        emailDomain: values.domain === undefined ? 'example.com' : parseDomain(values.domain),
        from,
        to,
        min,
        max,
        length: integer(values.length, 16, 1, 1024, 'length'),
    };
    const out: Value[] = Array.from({ length: count }, () => make(type, params));
    if (values.json) printJson({ type, locale, values: out });
    else console.log(out.join('\n'));
}

export const fakeTool: Tool = {
    name: 'fake',
    description: 'Buat data palsu (nama, email, alamat, IP, tanggal, dll.) untuk pengujian',
    category: 'Utilitas',
    run,
};
