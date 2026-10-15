import { randomUUID } from 'node:crypto';
import { UsageError } from '../../errors.js';
import { choice, integer, parseToolArgs, printJson } from '../../io.js';
import type { Tool } from '../../tool.js';
import { TOKEN_FORMATS, tokenByBytes, tokenByLength } from './token.js';
import { UUID_VERSIONS, createV1Generator, createV7Generator } from './version.js';

const commonOptions = {
    count: { type: 'string', short: 'c' },
    json: { type: 'boolean' },
    help: { type: 'boolean', short: 'h' },
} as const;

function help(): void {
    console.log(`dt random <uuid|token> [options]

  uuid   Buat identifier UUID v1/v4/v7 dengan format baku
  token  Buat token acak dengan ukuran dan encoding pilihan

Gunakan dt random uuid -h atau dt random token -h untuk opsi masing-masing.
Tanpa subcommand: tampilkan bantuan. Tidak menerima teks, file, atau stdin.
Exit: 0 sukses, 1 gagal, 2 penggunaan salah.`);
}

function uuidHelp(): void {
    console.log(`dt random uuid [options]

Buat UUID; panjang selalu 36 karakter, tidak bisa diubah.
  -v, --version <n>  ${UUID_VERSIONS.join(', ')} (default: 4)
  -c, --count <n>    Jumlah UUID, 1-1000 (default: 1), satu per baris
      --json         Keluaran JSON: {type: "uuid", version, values}
  -h, --help         Tampilkan bantuan

v4: 122 bit acak. v7: timestamp ms + acak, urut naik ketat dalam satu eksekusi.
v1: timestamp 100ns + node acak (bukan MAC asli), unik dalam satu eksekusi.
UUID adalah identifier, bukan token rahasia. Untuk token: dt random token.
Tidak menerima argumen teks atau opsi ukuran/format.
Contoh: dt random uuid -v 7 -c 3`);
}

function tokenHelp(): void {
    console.log(`dt random token [options]

Buat token acak dari CSPRNG (node:crypto).
  -l, --length <n>     Panjang dalam karakter, 1-4096
  -b, --bytes <n>      Panjang dalam byte, 1-1024
      --bits <n>       Panjang dalam bit, kelipatan 8, 8-8192
  -f, --format <jenis> ${TOKEN_FORMATS.join(', ')} (default: hex)
  -c, --count <n>      Jumlah token, 1-1000 (default: 1), satu per baris
      --json           Keluaran JSON: {type: "token", format, values}
  -h, --help           Tampilkan bantuan

Pilih salah satu: --length, --bytes, --bits. Default: 32 byte, atau 32 karakter
untuk alnum (A-Za-z0-9). alnum hanya mendukung --length.
--length menghasilkan jumlah karakter persis, tanpa padding "=".
Ukuran byte/bit menentukan data acak sebelum encoding, bukan jumlah karakter.
Tidak menerima argumen teks atau --version; tidak perlu flag --token.
Contoh: dt random token -l 24 -f alnum
        dt random token --bits 256 -f base64url`);
}

async function runUuid(args: string[]): Promise<void> {
    const { values, positionals } = parseToolArgs(args, {
        ...commonOptions,
        version: { type: 'string', short: 'v' },
    });
    if (values.help) {
        uuidHelp();
        return;
    }
    if (positionals.length) throw new UsageError(`Argumen tidak dikenal: ${positionals[0]}`);
    const count = integer(values.count, 1, 1, 1000, 'count');
    const version = choice(values.version, UUID_VERSIONS, '4', 'version');
    const next = version === '7' ? createV7Generator() : version === '1' ? createV1Generator() : randomUUID;
    const list = Array.from({ length: count }, () => next());
    if (values.json) printJson({ type: 'uuid', version: Number(version), values: list });
    else console.log(list.join('\n'));
}

async function runToken(args: string[]): Promise<void> {
    const { values, positionals } = parseToolArgs(args, {
        ...commonOptions,
        length: { type: 'string', short: 'l' },
        bytes: { type: 'string', short: 'b' },
        bits: { type: 'string' },
        format: { type: 'string', short: 'f' },
    });
    if (values.help) {
        tokenHelp();
        return;
    }
    if (positionals.length) throw new UsageError(`Argumen tidak dikenal: ${positionals[0]}`);
    const count = integer(values.count, 1, 1, 1000, 'count');
    const sizes = [values.length, values.bytes, values.bits].filter((v) => v !== undefined);
    if (sizes.length > 1) throw new UsageError('Pilih salah satu: --length, --bytes, atau --bits.');
    const format = choice(values.format, TOKEN_FORMATS, 'hex', 'format');
    if (format === 'alnum' && (values.bytes !== undefined || values.bits !== undefined)) {
        throw new UsageError('Format alnum hanya mendukung --length.');
    }
    let make: () => string;
    if (values.length !== undefined || format === 'alnum') {
        const length = integer(values.length, 32, 1, 4096, 'length');
        make = () => tokenByLength(format, length);
    } else {
        const bits = values.bits !== undefined ? integer(values.bits, 0, 8, 8192, 'bits') : undefined;
        if (bits !== undefined && bits % 8) throw new UsageError(`Nilai --bits harus kelipatan 8: ${bits}`);
        const bytes = bits !== undefined ? bits / 8 : integer(values.bytes, 32, 1, 1024, 'bytes');
        make = () => tokenByBytes(format as Exclude<typeof format, 'alnum'>, bytes);
    }
    const list = Array.from({ length: count }, make);
    if (values.json) printJson({ type: 'token', format, values: list });
    else console.log(list.join('\n'));
}

async function run(args: string[]): Promise<void> {
    const [command, ...options] = args;
    if (command === undefined || command === '--help' || command === '-h') {
        help();
        return;
    }
    if (command === 'uuid') return runUuid(options);
    if (command === 'token') return runToken(options);
    throw new UsageError(`Subcommand random tidak dikenal: ${command}. Pilihan: uuid, token.`);
}

export const randomTool: Tool = {
    name: 'random',
    description: 'Buat UUID atau token acak melalui subcommand uuid/token',
    category: 'Utilitas',
    run,
};
