import { randomUUID } from 'node:crypto';
import { UsageError } from '../../errors.js';
import { choice, integer, parseToolArgs, printJson } from '../../io.js';
import type { Tool } from '../../tool.js';
import { TOKEN_FORMATS, tokenByBytes, tokenByLength } from './token.js';
import { UUID_VERSIONS, createV1Generator, createV7Generator } from './version.js';

function help(): void {
    console.log(`dt uuid [options]

Buat UUID v4 (default), v7, v1, atau token acak dari CSPRNG (node:crypto).
UUID berformat baku 36 karakter; panjangnya tidak bisa diubah.
  -v, --version <n>       Versi UUID: ${UUID_VERSIONS.join(', ')} (default: 4)
  -t, --token             Buat token acak, bukan UUID
  -l, --length <n>        Panjang token dalam karakter, 1-4096
  -b, --bytes <n>         Panjang token dalam byte, 1-1024
      --bits <n>          Panjang token dalam bit, kelipatan 8, 8-8192
  -f, --format <jenis>    ${TOKEN_FORMATS.join(', ')} (default: hex)
  -c, --count <n>         Jumlah hasil, 1-1000 (default: 1), satu per baris
      --json              Keluaran JSON: {type, version?, format?, values}
  -h, --help              Tampilkan bantuan

v4: 122 bit acak. v7: timestamp ms + acak, urut naik ketat dalam satu eksekusi.
v1: timestamp 100ns + node acak (bukan MAC asli), unik dalam satu eksekusi.
Pilih salah satu dari --length, --bytes, --bits (default: 32 byte, atau 32
karakter untuk alnum). Karakter per byte: hex 2, base64url ~1,33; --length
memotong hasil, tanpa padding "=". alnum (A-Za-z0-9) hanya mendukung --length.
Opsi ukuran/format memerlukan --token; --version tidak bisa dengan --token.
Contoh: dt uuid -c 3   |   dt uuid -t -l 24 -f alnum   |   dt uuid -t --bits 256`);
}

async function run(args: string[]): Promise<void> {
    const { values, positionals } = parseToolArgs(args, {
        token: { type: 'boolean', short: 't' },
        length: { type: 'string', short: 'l' },
        bytes: { type: 'string', short: 'b' },
        bits: { type: 'string' },
        format: { type: 'string', short: 'f' },
        count: { type: 'string', short: 'c' },
        version: { type: 'string', short: 'v' },
        json: { type: 'boolean' },
        help: { type: 'boolean', short: 'h' },
    });
    if (values.help) {
        help();
        return;
    }
    if (positionals.length) throw new UsageError(`Argumen tidak dikenal: ${positionals[0]}`);
    const count = integer(values.count, 1, 1, 1000, 'count');
    const sizes = [values.length, values.bytes, values.bits].filter((v) => v !== undefined);
    if (!values.token) {
        if (sizes.length || values.format !== undefined) {
            throw new UsageError('--length, --bytes, --bits, dan --format memerlukan --token.');
        }
        const version = choice(values.version, UUID_VERSIONS, '4', 'version');
        const next = version === '7' ? createV7Generator() : version === '1' ? createV1Generator() : randomUUID;
        const list = Array.from({ length: count }, () => next());
        if (values.json) printJson({ type: 'uuid', version: Number(version), values: list });
        else console.log(list.join('\n'));
        return;
    }
    if (values.version !== undefined) throw new UsageError('--version tidak bisa dipakai dengan --token.');
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

export const uuid: Tool = {
    name: 'uuid',
    description: 'Buat UUID (v1/v4/v7) atau token acak yang aman secara kriptografis',
    category: 'Utilitas',
    run,
};
