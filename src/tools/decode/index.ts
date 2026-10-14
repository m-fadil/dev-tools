import { CODECS, decode } from '../../codec.js';
import { choice, parseToolArgs, readInput, writeOutput } from '../../io.js';
import type { Tool } from '../../tool.js';

function help(): void {
    console.log(`dt decode [options] [teks]

Decode teks, file, atau stdin. Tanpa teks/--file/stdin: prompt (terminal).
  -t, --type <jenis>  ${CODECS.join(', ')} (default: base64)
  -f, --file <path>   Decode isi file
  -h, --help          Tampilkan bantuan

Hasil ditulis apa adanya (byte mentah, tanpa newline tambahan saat dipipe),
sehingga data biner aman diarahkan ke file. Input tidak valid: exit 1.
Contoh: dt decode -t hex 68616c6f`);
}

async function run(args: string[]): Promise<void> {
    const { values, positionals } = parseToolArgs(args, {
        type: { type: 'string', short: 't' },
        file: { type: 'string', short: 'f' },
        help: { type: 'boolean', short: 'h' },
    });
    if (values.help) {
        help();
        return;
    }
    const codec = choice(values.type, CODECS, 'base64', 'type');
    const text = (await readInput(positionals, values.file, 'Teks yang akan di-decode')).toString('utf8');
    writeOutput(decode(text, codec));
}

export const decodeTool: Tool = {
    name: 'decode',
    description: 'Decode base64, base64url, hex, atau URL ke teks/biner',
    category: 'Encoding',
    run,
};
