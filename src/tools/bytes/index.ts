import { UsageError } from '../../errors.js';
import { integer, parseToolArgs, printJson } from '../../io.js';
import type { Tool } from '../../tool.js';
import { BINARY_UNITS, DECIMAL_UNITS, UNIT_NAMES, findUnit, formatValue, parseSize, readable } from './units.js';

function help(): void {
    console.log(`dt bytes <ukuran> [options]

Konversi ukuran data antar satuan dengan aritmetika eksak (tanpa galat float).
Ukuran: angka + satuan opsional, mis. 1536, 1.5GB, "2 GiB", 10kb. Angka tanpa
satuan = byte; desimal boleh; negatif ditolak.
Satuan: B; desimal (basis 1000) KB/kB, MB, GB, TB, PB; biner (basis 1024)
KiB, MiB, GiB, TiB, PiB. Huruf besar/kecil tidak dibedakan: kb=1000, kib=1024.
  -t, --to <satuan>       Cetak hanya nilai konversi ke satuan itu (angka saja)
  -p, --precision <n>     Angka desimal, 0-10 (default: 2); nol di ujung dibuang
      --json              Keluaran JSON
  -h, --help              Tampilkan bantuan

Hasil byte yang tidak bulat dibulatkan (setengah ke atas) dan ditandai.
Contoh: dt bytes 1.5GB   |   dt bytes "2 GiB" --to MB`);
}

async function run(args: string[]): Promise<void> {
    // Keep negative option values attached so parseArgs does not mistake them for options.
    const rest: string[] = [];
    for (let i = 0; i < args.length; i++) {
        const arg = args[i]!;
        if (arg === '--') {
            rest.push(...args.slice(i));
            break;
        }
        const option =
            arg === '-p' || arg === '--precision' ? 'precision' : arg === '-t' || arg === '--to' ? 'to' : undefined;
        if (option !== undefined && /^-\.?\d/.test(args[i + 1] ?? '')) {
            rest.push(`--${option}=${args[++i]}`);
        } else if (/^-\.?\d/.test(arg)) {
            throw new UsageError(`Ukuran tidak boleh negatif: ${arg}`);
        } else {
            rest.push(arg);
        }
    }
    const { values, positionals } = parseToolArgs(rest, {
        to: { type: 'string', short: 't' },
        precision: { type: 'string', short: 'p' },
        json: { type: 'boolean' },
        help: { type: 'boolean', short: 'h' },
    });
    if (values.help) {
        help();
        return;
    }
    if (!positionals.length) throw new UsageError('Ukuran wajib diisi. Contoh: dt bytes 1.5GB');
    const precision = integer(values.precision, 2, 0, 10, 'precision');
    const target = values.to === undefined ? undefined : findUnit(values.to);
    if (values.to !== undefined && !target) {
        throw new UsageError(`Nilai --to tidak valid: ${values.to}. Pilihan: ${UNIT_NAMES.join(', ')}`);
    }
    const input = positionals.join(' ');
    const { bytes, rounded } = parseSize(input);
    if (rounded) console.error(`Peringatan: ukuran bukan byte bulat, dibulatkan menjadi ${bytes} B.`);

    if (target) {
        const value = formatValue(bytes, target.factor, precision);
        if (values.json) printJson({ input, bytes: bytes.toString(), rounded, unit: target.name, value });
        else console.log(value);
        return;
    }
    const convert = (units: typeof DECIMAL_UNITS) =>
        Object.fromEntries(units.map((u) => [u.name, formatValue(bytes, u.factor, precision)]));
    const decimal = readable(bytes, DECIMAL_UNITS, precision);
    const binary = readable(bytes, BINARY_UNITS, precision);
    if (values.json) {
        printJson({
            input,
            bytes: bytes.toString(),
            rounded,
            precision,
            decimal: convert(DECIMAL_UNITS),
            binary: convert(BINARY_UNITS),
            readable: { decimal, binary },
        });
        return;
    }
    const lines = [`Byte: ${bytes}${rounded ? ' (dibulatkan)' : ''}`];
    for (const units of [DECIMAL_UNITS, BINARY_UNITS]) {
        const table = units.map((u) => [u.name, formatValue(bytes, u.factor, precision)] as const);
        const w = Math.max(...table.map(([, v]) => v.length));
        lines.push('', ...table.map(([name, v]) => `${name.padEnd(4)} ${v.padStart(w)}`));
    }
    lines.push('', `Terbaca: ${decimal} | ${binary}`);
    console.log(lines.join('\n'));
}

export const bytesTool: Tool = {
    name: 'bytes',
    description: 'Konversi ukuran data (KB/MB/GB dan KiB/MiB/GiB) secara eksak',
    category: 'Utilitas',
    run,
};
