import { UsageError } from '../../errors.js';
import { integer, parseToolArgs, printJson } from '../../io.js';
import type { Tool } from '../../tool.js';
import { checkPort, findFreePort } from './probe.js';

function help(): void {
    console.log(`dt port <check|free> [options]

Periksa port TCP atau cari port yang masih kosong.
  check <port>            Coba koneksi TCP; cetak "<host>:<port> terbuka|tertutup|timeout"
  free                    Cetak port pertama yang bisa di-bind dalam rentang --from..--to
      --host <host>       Host (default: 127.0.0.1)
      --timeout <ms>      Batas waktu koneksi untuk check, 1-60000 (default: 1000)
      --from <n>          Awal rentang free, 1-65535 (default: 3000)
      --to <n>            Akhir rentang free, 1-65535 (default: 65535)
      --json              Keluaran satu dokumen JSON (check: host, port, status open|closed|timeout; free: host, port)
  -h, --help              Tampilkan bantuan

Exit: check 0 jika terbuka, 1 jika tertutup/timeout/galat soket; free 1 jika
tidak ada port kosong; 2 penggunaan salah.
free benar-benar listen lalu menutup port, jadi port bisa terpakai lagi sebelum Anda memakainya.
Contoh: dt port check 5432   |   dt port free --from 8000 --to 8100`);
}

function parsePort(value: string): number {
    if (!/^\d+$/.test(value) || Number(value) < 1 || Number(value) > 65535) {
        throw new UsageError(`Port harus bilangan bulat 1-65535: ${value}`);
    }
    return Number(value);
}

async function run(args: string[]): Promise<void> {
    const { values, positionals } = parseToolArgs(args, {
        host: { type: 'string' },
        timeout: { type: 'string' },
        from: { type: 'string' },
        to: { type: 'string' },
        json: { type: 'boolean' },
        help: { type: 'boolean', short: 'h' },
    });
    if (values.help) {
        help();
        return;
    }
    const [command, ...rest] = positionals;
    if (command !== 'check' && command !== 'free') {
        throw new UsageError(
            command === undefined ? 'Subcommand diperlukan: check atau free.' : `Subcommand tidak dikenal: ${command}`,
        );
    }
    const host = values.host ?? '127.0.0.1';
    if (!host) throw new UsageError('Nilai --host tidak boleh kosong.');

    if (command === 'check') {
        if (rest.length !== 1) throw new UsageError('Gunakan: dt port check <port>');
        if (values.from !== undefined || values.to !== undefined) {
            throw new UsageError('--from dan --to hanya untuk free.');
        }
        const port = parsePort(rest[0]!);
        const timeout = integer(values.timeout, 1000, 1, 60000, 'timeout');
        const result = await checkPort(host, port, timeout);
        if (values.json) {
            const status = { terbuka: 'open', tertutup: 'closed', timeout: 'timeout' }[result];
            printJson({ host, port, status });
        } else console.log(`${host}:${port} ${result}`);
        if (result !== 'terbuka') process.exitCode = 1;
        return;
    }

    if (rest.length) throw new UsageError(`Argumen tidak dikenal: ${rest[0]}`);
    if (values.timeout !== undefined) throw new UsageError('--timeout hanya untuk check.');
    const from = integer(values.from, 3000, 1, 65535, 'from');
    const to = integer(values.to, 65535, 1, 65535, 'to');
    if (from > to) throw new UsageError(`--from (${from}) tidak boleh lebih besar dari --to (${to}).`);
    const port = await findFreePort(host, from, to);
    if (port === undefined) throw new Error(`Tidak ada port kosong di ${host} pada rentang ${from}-${to}.`);
    if (values.json) printJson({ host, port });
    else console.log(port);
}

export const portTool: Tool = {
    name: 'port',
    description: 'Periksa port TCP terbuka atau cari port kosong',
    category: 'Jaringan',
    run,
};
