import { UsageError } from '../../errors.js';
import { choice, integer, parseToolArgs, printJson, readInput } from '../../io.js';
import type { Tool } from '../../tool.js';
import { BASES, WIDTHS, formatBase, parseNumber, twosComplement, type Base } from './radix.js';

function help(): void {
    console.log(`dt number <nilai> [options]

Ubah bilangan bulat antar basis 2, 8, 10, 16 (ukuran bebas, BigInt).
Nilai: desimal atau berprefix 0x (hex), 0b (biner), 0o (oktal); boleh bertanda
- atau +, dan pemisah _ di antara digit (0xFF_FF). Tanpa argumen dibaca dari stdin.
  -f, --from <basis>      auto, 2, 8, 10, 16 (default: auto = menurut prefix)
  -t, --to <basis>        Cetak hanya basis itu: 2, 8, 10, 16 (tanpa prefix)
      --prefix            Dengan --to: sertakan prefix (0x, 0b, 0o)
  -g, --group             Kelompokkan biner per nibble (4 bit) dan hex per byte
                          dengan _, diisi nol di kiri
  -w, --width <bit>       Tampilkan juga komplemen dua: 8, 16, 32, 64. Nilai harus
                          muat dalam rentang bertanda atau tak bertanda
      --json              Keluaran JSON (tidak bisa dengan --to/--prefix/--group)
  -h, --help              Tampilkan bantuan

Bilangan negatif ditampilkan sebagai -0x.. Nilai negatif seperti -5 bisa
langsung ditulis sebagai argumen.
Contoh: dt number 255   |   dt number -f 2 1010 -t 16   |   dt number -1 -w 16`);
}

async function run(args: string[]): Promise<void> {
    // Lift negative positionals out, but keep negative option values attached for parseArgs.
    const rest: string[] = [];
    const negatives: string[] = [];
    for (let i = 0; i < args.length; i++) {
        const arg = args[i]!;
        if (arg === '--') {
            rest.push(...args.slice(i));
            break;
        }
        const option =
            arg === '-f' || arg === '--from'
                ? 'from'
                : arg === '-t' || arg === '--to'
                  ? 'to'
                  : arg === '-w' || arg === '--width'
                    ? 'width'
                    : undefined;
        if (option !== undefined && /^-\d/.test(args[i + 1] ?? '')) {
            rest.push(`--${option}=${args[++i]}`);
        } else if (/^-\d/.test(arg)) {
            negatives.push(arg);
        } else {
            rest.push(arg);
        }
    }
    const { values, positionals } = parseToolArgs(rest, {
        from: { type: 'string', short: 'f' },
        to: { type: 'string', short: 't' },
        prefix: { type: 'boolean' },
        group: { type: 'boolean', short: 'g' },
        width: { type: 'string', short: 'w' },
        json: { type: 'boolean' },
        help: { type: 'boolean', short: 'h' },
    });
    if (values.help) {
        help();
        return;
    }
    const from = choice(values.from, ['auto', '2', '8', '10', '16'] as const, 'auto', 'from');
    const to =
        values.to === undefined ? undefined : Number(choice(values.to, ['2', '8', '10', '16'] as const, '10', 'to'));
    const width = values.width === undefined ? undefined : integer(values.width, 0, 1, 64, 'width');
    if (width !== undefined && !(WIDTHS as readonly number[]).includes(width)) {
        throw new UsageError(`Nilai --width harus salah satu dari ${WIDTHS.join(', ')}: ${values.width}`);
    }
    if (values.prefix && to === undefined) throw new UsageError('--prefix memerlukan --to.');
    if (values.json && (to !== undefined || values.prefix || values.group)) {
        throw new UsageError('--json tidak bisa digabung dengan --to, --prefix, atau --group.');
    }
    const words = [...negatives, ...positionals];
    if (words.length > 1) throw new UsageError(`Hanya satu nilai yang diperbolehkan, ada ${words.length}.`);
    const text = words[0] ?? (await readInput([], undefined, 'Nilai:')).toString('utf8');
    const { value, base } = parseNumber(text, from === 'auto' ? from : (Number(from) as Base));
    const pattern = width === undefined ? undefined : twosComplement(value, width);

    if (to !== undefined) {
        const shown = pattern ?? value;
        console.log(
            formatBase(shown, to as Base, {
                prefix: values.prefix,
                group: values.group,
                pad: pattern === undefined ? undefined : padFor(to as Base, width!),
            }),
        );
        return;
    }
    const all = (n: bigint, pad?: Record<Base, number>) =>
        Object.fromEntries(
            BASES.map((b) => [
                { 2: 'binary', 8: 'octal', 10: 'decimal', 16: 'hex' }[b],
                formatBase(n, b, { prefix: true, group: values.group && b !== 10, pad: pad?.[b] }),
            ]),
        ) as Record<'decimal' | 'hex' | 'binary' | 'octal', string>;
    const main = all(value);
    const twos =
        pattern === undefined
            ? undefined
            : all(pattern, { 2: width!, 8: padFor(8, width!), 10: 0, 16: padFor(16, width!) });
    if (values.json) {
        printJson({
            input: text.trim(),
            base,
            negative: value < 0n,
            ...main,
            ...(twos && { twosComplement: { width, ...twos } }),
        });
        return;
    }
    const lines = (r: typeof main) => [
        `Desimal: ${r.decimal}`,
        `Hex:     ${r.hex}`,
        `Biner:   ${r.binary}`,
        `Oktal:   ${r.octal}`,
    ];
    const out = lines(main);
    if (twos) out.push('', `Komplemen dua (${width} bit):`, ...lines(twos));
    console.log(out.join('\n'));
}

function padFor(base: Base, width: number): number {
    return base === 10 ? 0 : Math.ceil(width / Math.log2(base));
}

export const numberTool: Tool = {
    name: 'number',
    aliases: ['num'],
    description: 'Ubah bilangan antar basis 2/8/10/16 dengan ukuran bebas',
    category: 'Matematika',
    run,
};
