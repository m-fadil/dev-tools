import { choice, parseToolArgs, printJson, readInput } from '../../io.js';
import { STYLES, convert, splitLines } from './convert.js';
function help() {
    console.log(`dt case [options] [teks]

Ubah gaya penulisan teks/identifier. Kata dikenali dari spasi, _ - . /, batas
camelCase, akronim (HTTPServerError -> http server error), angka, dan huruf Unicode.
Teks multibaris: tiap baris dikonversi sendiri. Tanpa teks: baca stdin.
  -t, --to <gaya>     ${STYLES.join(', ')}, all
                      (default: all, tampilkan semua gaya berlabel)
  -f, --file <path>   Baca teks dari file
      --json          Keluaran JSON
  -h, --help          Tampilkan bantuan

Contoh: dt case -t snake parseURL2Json   |   echo "hello world" | dt case`);
}
async function run(args) {
    const { values, positionals } = parseToolArgs(args, {
        to: { type: 'string', short: 't' },
        file: { type: 'string', short: 'f' },
        json: { type: 'boolean' },
        help: { type: 'boolean', short: 'h' },
    });
    if (values.help) {
        help();
        return;
    }
    const to = choice(values.to, [...STYLES, 'all'], 'all', 'to');
    const data = await readInput(positionals, values.file, 'Teks yang akan dikonversi');
    const lines = splitLines(data.toString('utf8'));
    const styles = to === 'all' ? STYLES : [to];
    if (values.json) {
        printJson({
            style: to,
            lines: lines.map((input) => ({
                input,
                output: to === 'all' ? Object.fromEntries(STYLES.map((s) => [s, convert(input, s)])) : convert(input, to),
            })),
        });
        return;
    }
    if (to !== 'all') {
        for (const line of lines)
            console.log(convert(line, to));
        return;
    }
    const width = Math.max(...styles.map((s) => s.length));
    lines.forEach((line, i) => {
        if (i > 0)
            console.log();
        for (const s of styles)
            console.log(`${s.padEnd(width)}  ${convert(line, s)}`);
    });
}
export const caseTool = {
    name: 'case',
    description: 'Ubah gaya penulisan teks (camel, snake, kebab, title, dst.)',
    category: 'Data',
    run,
};
//# sourceMappingURL=index.js.map