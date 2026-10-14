import { UsageError } from '../../errors.js';
import { parseToolArgs, printJson } from '../../io.js';
import { describe, label, parseValue, table } from './chars.js';
function help() {
    console.log(`dt ascii [nilai]

Tabel ASCII atau info satu karakter.
nilai: satu karakter (A), kode desimal (65), atau hex (0x41). Satu karakter
diutamakan: "5" adalah karakter 5; kode 0-9 desimal tulis sebagai 0x05.
Karakter non-ASCII menampilkan code point Unicode, byte UTF-8, dan nilai desimal.
  -t, --table     Tabel 0-127 dalam 4 kolom (default tanpa nilai)
      --json      Keluaran JSON
  -h, --help      Tampilkan bantuan

Contoh: dt ascii A   |   dt ascii 0x0a   |   dt ascii é`);
}
async function run(args) {
    const { values, positionals } = parseToolArgs(args, {
        table: { type: 'boolean', short: 't' },
        json: { type: 'boolean' },
        help: { type: 'boolean', short: 'h' },
    });
    if (values.help) {
        help();
        return;
    }
    if (positionals.length > 1)
        throw new UsageError(`Hanya satu nilai yang diizinkan: ${positionals.join(' ')}`);
    if (values.table && positionals.length)
        throw new UsageError('--table tidak dapat digabung dengan nilai.');
    if (!positionals.length) {
        const rows = table();
        if (values.json)
            return printJson({ table: rows });
        const cell = (i) => {
            const c = rows[i];
            return `${String(c.decimal).padStart(3)} ${c.hex.slice(2)} ${c.octal} ${label(c).padEnd(3)}`;
        };
        const header = Array(4).fill('dec hx oct chr').join(' | ');
        console.log(header);
        console.log(header.replace(/[^|]/g, '-').replace(/\|/g, '+'));
        for (let r = 0; r < 32; r++)
            console.log([0, 32, 64, 96]
                .map((c) => cell(c + r))
                .join(' | ')
                .trimEnd());
        return;
    }
    const info = describe(parseValue(positionals[0]));
    if (values.json)
        return printJson(info);
    console.log(`desimal   ${info.decimal}`);
    console.log(`hex       ${info.hex}`);
    console.log(`oktal     ${info.octal}`);
    console.log(`biner     ${info.binary}`);
    if (!info.ascii) {
        console.log(`unicode   ${info.unicode}`);
        console.log(`utf-8     ${info.utf8.join(' ')}`);
    }
    if (info.name)
        console.log(`nama      ${info.name}`);
    if (info.character !== null)
        console.log(`karakter  ${info.character}`);
    else if (!info.name)
        console.log('karakter  (tidak tercetak)');
    if (info.description)
        console.log(`deskripsi ${info.description}`);
}
export const asciiTool = {
    name: 'ascii',
    description: 'Tabel ASCII dan info karakter (desimal, hex, oktal, biner, Unicode)',
    category: 'Utilitas',
    run,
};
//# sourceMappingURL=index.js.map