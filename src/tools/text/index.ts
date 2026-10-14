import { UsageError } from '../../errors.js';
import { integer, parseToolArgs, printJson, readInput } from '../../io.js';
import type { Tool } from '../../tool.js';
import { body, frequency, processLines, reverseText, slugify, stats } from './analyze.js';

const SUBCOMMANDS = ['stats', 'reverse', 'slug', 'freq', 'lines'] as const;
type Sub = (typeof SUBCOMMANDS)[number];

/** Options each subcommand accepts besides --file/--json/--help. */
const SPECIFIC: Record<Sub, readonly string[]> = {
    stats: [],
    reverse: ['lines'],
    slug: ['sep'],
    freq: ['top', 'case-sensitive', 'min-length'],
    lines: ['sort', 'unique', 'numbered', 'reverse', 'trim'],
};

function help(): void {
    console.log(`dt text <perintah> [opsi] [teks]

Alat teks. Tanpa teks: baca stdin atau --file. Satu newline di akhir input diabaikan.
Perintah:
  stats     Statistik: karakter (code point), grafem, byte input asli, kata, baris,
            kalimat (sederhana), baris terpanjang, rata-rata panjang kata
  reverse   Balik tiap baris per grafem
              --lines             Balik urutan baris saja
  slug      Slug URL: hilangkan diakritik U+0300–U+036F, huruf kecil
              --sep <char>        Pemisah satu karakter (default: -)
  freq      Frekuensi kata
              --top <n>           Tampilkan n teratas (default: 10)
              --case-sensitive    Bedakan huruf besar/kecil (default: tidak)
              --min-length <n>    Abaikan kata lebih pendek dari n (default: 1)
  lines     Olah baris (urutan: trim, unique, sort, reverse, lalu numbered)
              --trim              Pangkas spasi di ujung tiap baris
              --unique            Hapus duplikat (kemunculan pertama)
              --sort              Urutkan menurut code point Unicode
              --reverse           Balik urutan
              --numbered          Beri nomor baris (tidak dengan --json)
Opsi umum:
  -f, --file <path>   Baca teks dari file
      --json          Keluaran JSON
  -h, --help          Tampilkan bantuan

Slug kosong menghasilkan galat (exit 1).
Contoh: dt text slug "Héllo Wörld!"   |   cat a.txt | dt text freq --top 5`);
}

async function run(args: string[]): Promise<void> {
    const { values, positionals } = parseToolArgs(args, {
        file: { type: 'string', short: 'f' },
        json: { type: 'boolean' },
        help: { type: 'boolean', short: 'h' },
        lines: { type: 'boolean' },
        sep: { type: 'string' },
        top: { type: 'string' },
        'case-sensitive': { type: 'boolean' },
        'min-length': { type: 'string' },
        sort: { type: 'boolean' },
        unique: { type: 'boolean' },
        numbered: { type: 'boolean' },
        reverse: { type: 'boolean' },
        trim: { type: 'boolean' },
    });
    if (values.help) {
        help();
        return;
    }
    const sub = positionals.shift();
    if (sub === undefined) throw new UsageError(`Perintah wajib: ${SUBCOMMANDS.join(', ')}`);
    if (!(SUBCOMMANDS as readonly string[]).includes(sub)) {
        throw new UsageError(`Perintah tidak dikenal: ${sub}. Pilihan: ${SUBCOMMANDS.join(', ')}`);
    }
    const command = sub as Sub;
    const generic = ['file', 'json', 'help'];
    for (const name of Object.keys(values)) {
        if (!generic.includes(name) && !SPECIFIC[command].includes(name)) {
            throw new UsageError(`Opsi --${name} tidak berlaku untuk "${command}".`);
        }
    }
    if (values.json && values.numbered) throw new UsageError('--numbered tidak dapat digabung dengan --json.');

    // Validate options before reading input so mistakes fail fast.
    let sep = '-';
    if (command === 'slug' && values.sep !== undefined) {
        sep = values.sep;
        if (Array.from(sep).length !== 1 || /[\p{L}\p{N}\s]/u.test(sep)) {
            throw new UsageError(`--sep harus satu karakter selain huruf, angka, atau spasi: ${JSON.stringify(sep)}`);
        }
    }
    const top = integer(values.top, 10, 1, 1_000_000, 'top');
    const minLength = integer(values['min-length'], 1, 1, 1_000_000, 'min-length');

    const input = await readInput(positionals, values.file, 'Teks');
    const raw = input.toString('utf8');
    switch (command) {
        case 'stats': {
            const s = stats(raw, input);
            if (values.json) return printJson(s);
            console.log(`karakter            ${s.characters}`);
            console.log(`grafem              ${s.graphemes}`);
            console.log(`byte (UTF-8)        ${s.bytes}`);
            console.log(`kata                ${s.words}`);
            console.log(`baris               ${s.lines}`);
            console.log(`kalimat             ${s.sentences}`);
            console.log(
                `baris terpanjang    ${s.longestLine.length}` +
                    (s.longestLine.number ? ` grafem (baris ${s.longestLine.number})` : ''),
            );
            console.log(`rata-rata kata      ${s.averageWordLength}`);
            return;
        }
        case 'reverse': {
            const out = reverseText(raw, Boolean(values.lines));
            if (values.json) return printJson({ lines: out });
            if (out.length) console.log(out.join('\n'));
            return;
        }
        case 'slug': {
            const slug = slugify(body(raw), sep);
            if (!slug) throw new Error('Slug kosong: teks tidak mengandung huruf atau angka.');
            if (values.json) return printJson({ slug });
            console.log(slug);
            return;
        }
        case 'freq': {
            const r = frequency(raw, { top, caseSensitive: Boolean(values['case-sensitive']), minLength });
            if (values.json) return printJson(r);
            const width = String(r.top[0]?.count ?? 0).length;
            for (const { word, count } of r.top) console.log(`${String(count).padStart(width)}  ${word}`);
            return;
        }
        case 'lines': {
            const out = processLines(raw, {
                trim: Boolean(values.trim),
                unique: Boolean(values.unique),
                sort: Boolean(values.sort),
                reverse: Boolean(values.reverse),
            });
            if (values.json) return printJson({ count: out.length, lines: out });
            const width = String(out.length).length;
            const shown = values.numbered ? out.map((l, i) => `${String(i + 1).padStart(width)}  ${l}`) : out;
            if (shown.length) console.log(shown.join('\n'));
            return;
        }
    }
}

export const textTool: Tool = {
    name: 'text',
    description: 'Alat teks: statistik, balik, slug, frekuensi kata, olah baris',
    category: 'Data',
    run,
};
