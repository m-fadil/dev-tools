import { UsageError } from '../../errors.js';
import { integer, parseToolArgs, printJson } from '../../io.js';
import type { Tool } from '../../tool.js';
import { rngFromSeed } from '../fake/rng.js';
import { generate, LIMITS, type Unit } from './text.js';

function help(): void {
    console.log(`dt lorem [options]

Buat teks lorem ipsum.
  -w, --words <n>        Jumlah kata, 1-${LIMITS.words}
  -s, --sentences <n>    Jumlah kalimat, 1-${LIMITS.sentences}
  -p, --paragraphs <n>   Jumlah paragraf, 1-${LIMITS.paragraphs} (default: 3)
      --seed <n>         Seed bilangan bulat 0-4294967295 agar hasil dapat diulang
      --classic          Mulai dengan "Lorem ipsum dolor sit amet, consectetur adipiscing elit"
      --json             Keluaran JSON: {unit, count, text}
  -h, --help             Tampilkan bantuan

Pilih tepat satu satuan. Kalimat 5-15 kata, paragraf 3-7 kalimat dipisah baris kosong.
Contoh: dt lorem -w 20 --seed 1   |   dt lorem -p 2 --classic`);
}

async function run(args: string[]): Promise<void> {
    const { values, positionals } = parseToolArgs(args, {
        words: { type: 'string', short: 'w' },
        sentences: { type: 'string', short: 's' },
        paragraphs: { type: 'string', short: 'p' },
        seed: { type: 'string' },
        classic: { type: 'boolean' },
        json: { type: 'boolean' },
        help: { type: 'boolean', short: 'h' },
    });
    if (values.help) {
        help();
        return;
    }
    if (positionals.length) throw new UsageError(`Argumen tidak dikenal: ${positionals[0]}`);
    const given = (['words', 'sentences', 'paragraphs'] as const).filter((u) => values[u] !== undefined);
    if (given.length > 1) throw new UsageError('Pilih tepat satu: --words, --sentences, atau --paragraphs.');
    const unit: Unit = given[0] ?? 'paragraphs';
    const count = integer(values[unit], unit === 'paragraphs' ? 3 : 1, 1, LIMITS[unit], unit);
    const text = generate(rngFromSeed(values.seed), unit, count, values.classic === true);
    if (values.json) printJson({ unit, count, text });
    else console.log(text);
}

export const loremTool: Tool = {
    name: 'lorem',
    description: 'Buat teks lorem ipsum (kata, kalimat, atau paragraf)',
    category: 'Utilitas',
    run,
};
