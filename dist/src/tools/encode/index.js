import { CODECS, encode } from '../../codec.js';
import { choice, parseToolArgs, printJson, readInput, writeOutput } from '../../io.js';
function help() {
    console.log(`dt encode [options] [teks]

Encode teks, file, atau stdin. Tanpa teks/--file/stdin: prompt (terminal).
  -t, --type <jenis>  ${CODECS.join(', ')} (default: base64)
  -f, --file <path>   Encode isi file
      --json          Keluaran JSON: {type, output}
  -h, --help          Tampilkan bantuan

Kebalikannya: dt decode. Contoh: dt encode -t url "a b&c"`);
}
async function run(args) {
    const { values, positionals } = parseToolArgs(args, {
        type: { type: 'string', short: 't' },
        file: { type: 'string', short: 'f' },
        json: { type: 'boolean' },
        help: { type: 'boolean', short: 'h' },
    });
    if (values.help) {
        help();
        return;
    }
    const codec = choice(values.type, CODECS, 'base64', 'type');
    const data = await readInput(positionals, values.file, 'Teks yang akan di-encode');
    const output = encode(data, codec);
    if (values.json)
        printJson({ type: codec, output });
    else
        writeOutput(`${output}\n`);
}
export const encodeTool = {
    name: 'encode',
    description: 'Encode teks/file ke base64, base64url, hex, atau URL',
    category: 'Encoding',
    run,
};
//# sourceMappingURL=index.js.map