import { UsageError } from '../../errors.js';
import { choice, parseToolArgs, printJson } from '../../io.js';
import type { Tool } from '../../tool.js';
import { checkRange, formatInZone, parseInstant, relative, resolveZone } from './time.js';

function help(): void {
    console.log(`dt time [options] [nilai]

Tampilkan satu waktu dalam UTC, zona pilihan, epoch, dan waktu relatif.
Tanpa nilai: waktu sekarang.
  -z, --timezone <IANA>   Zona waktu, mis. Asia/Jakarta (default: zona sistem)
  -u, --unit <s|ms>       Satuan epoch, juga untuk --now (default: deteksi, < 1e11 detik, selain itu milidetik).
                          Ditolak untuk nilai ISO atau bila tidak ada epoch
      --now <waktu>       Ganti jam sekarang untuk hitungan relatif (ISO atau epoch)
      --json              Keluaran satu dokumen JSON (input, utc, zone, epochSeconds, epochMillis, relative)
  -h, --help              Tampilkan bantuan

Nilai: epoch bulat (boleh negatif) atau ISO-8601: 2026-01-02, 2026-01-02T10:04:05,
2026-01-02T10:04:05.123+07:00, 2026-01-02T03:04:05Z. Tanpa offset, waktu dibaca
di zona --timezone (tanggal saja = tengah malam zona itu). Epoch negatif perlu
"--" di depannya: dt time -- -86400
Rentang: ±8,64e15 ms. Input tidak valid keluar dengan kode 2.
Contoh: dt time 1767323045   |   dt time -z Asia/Jakarta 2026-01-02T10:04:05`);
}

async function run(args: string[]): Promise<void> {
    const { values, positionals } = parseToolArgs(args, {
        timezone: { type: 'string', short: 'z' },
        unit: { type: 'string', short: 'u' },
        now: { type: 'string' },
        json: { type: 'boolean' },
        help: { type: 'boolean', short: 'h' },
    });
    if (values.help) {
        help();
        return;
    }
    if (positionals.length > 1) throw new UsageError(`Terlalu banyak argumen: ${positionals[1]}`);
    const zone = resolveZone(values.timezone);
    const unit = values.unit === undefined ? undefined : choice(values.unit, ['s', 'ms'] as const, 's', 'unit');
    if (unit !== undefined && positionals.length === 0 && values.now === undefined) {
        throw new UsageError('--unit hanya berlaku untuk nilai epoch');
    }
    const now =
        values.now !== undefined ? parseInstant(values.now, zone, unit) : checkRange(Date.now(), 'Waktu sekarang');
    const ms = positionals.length ? parseInstant(positionals[0]!, zone, unit) : now;

    if (values.json) {
        printJson({
            input: positionals[0] ?? null,
            utc: new Date(ms).toISOString(),
            zone: { name: zone, iso: formatInZone(ms, zone) },
            epochSeconds: Math.floor(ms / 1000),
            epochMillis: ms,
            relative: relative(ms, now),
        });
        return;
    }
    const rows: [string, string][] = [
        ['UTC', new Date(ms).toISOString()],
        [zone, formatInZone(ms, zone)],
        ['Epoch (s)', String(Math.floor(ms / 1000))],
        ['Epoch (ms)', String(ms)],
        ['Relatif', relative(ms, now)],
    ];
    const width = Math.max(...rows.map(([label]) => label.length)) + 2;
    console.log(rows.map(([label, value]) => `${label}:`.padEnd(width) + value).join('\n'));
}

export const timeTool: Tool = {
    name: 'time',
    description: 'Konversi epoch/ISO-8601 ke UTC, zona waktu, dan waktu relatif',
    category: 'Utilitas',
    aliases: ['ts'],
    run,
};
