import { UsageError } from '../../errors.js';
import { integer, parseToolArgs, printJson } from '../../io.js';
import type { Tool } from '../../tool.js';
import { explain, formatTime, HORIZON_YEARS, nextFires, parseCron, parseFrom } from './schedule.js';

function help(): void {
    console.log(`dt cron [options] <ekspresi>

Jelaskan ekspresi cron 5 field (menit jam tanggal bulan hari) dan tampilkan
waktu eksekusi berikutnya. Ekspresi diberikan sebagai satu argumen dikutip,
atau 5 argumen terpisah.
  -n, --next <n>      Jumlah waktu berikutnya, 1-50 (default: 5)
      --from <waktu>  Mulai dihitung setelah waktu ISO-8601 ini (default: sekarang)
      --utc           Pakai UTC, bukan zona waktu sistem
      --json          Keluaran JSON tunggal (tanpa penjelasan teks)
  -h, --help          Tampilkan bantuan

Field: * | a | a,b | a-b | */n | a-b/n | a/n (a sampai maksimum). Nama bulan
jan-dec dan hari sun-sat boleh dipakai (tanpa membedakan huruf besar/kecil).
Hari 0-7 (0 dan 7 = Minggu); rentang tidak melingkar, tulis 5-7, bukan fri-sun.
Makro: @yearly @annually @monthly @weekly @daily @midnight @hourly.
Jika tanggal DAN hari sama-sama dibatasi (tidak diawali *), waktu cocok bila
salah satunya terpenuhi. --from tanpa offset dibaca di zona yang dipakai.
DST: jam lokal yang terlewat (maju) tidak dijalankan; jam yang berulang
(mundur) hanya dijalankan sekali, pada kemunculan pertama.
Exit: 0 sukses, 1 tidak ada waktu dalam ${HORIZON_YEARS} tahun (mis. 0 0 31 2 *), 2 penggunaan salah.
Contoh: dt cron "*/15 9-17 * * mon-fri"   |   dt cron -n 3 --utc @daily`);
}

async function run(args: string[]): Promise<void> {
    const { values, positionals } = parseToolArgs(args, {
        next: { type: 'string', short: 'n' },
        from: { type: 'string' },
        utc: { type: 'boolean' },
        json: { type: 'boolean' },
        help: { type: 'boolean', short: 'h' },
    });
    if (values.help) {
        help();
        return;
    }
    const tokens = positionals.flatMap((p) => p.trim().split(/\s+/)).filter(Boolean);
    if (!tokens.length) throw new UsageError('Ekspresi cron wajib diisi, mis. "*/5 * * * *".');
    const utc = values.utc ?? false;
    const count = integer(values.next, 5, 1, 50, 'next');
    const from = values.from === undefined ? new Date() : parseFrom(values.from, utc);
    const schedule = parseCron(tokens);

    const { times, exhausted } = nextFires(schedule, from, count, utc);
    if (values.json) {
        printJson({
            expression: tokens.join(' '),
            fields: {
                minute: schedule.minute,
                hour: schedule.hour,
                dayOfMonth: schedule.dom,
                month: schedule.month,
                dayOfWeek: schedule.dow,
            },
            from: formatTime(from, utc),
            timezone: utc ? 'utc' : 'local',
            next: times.map((t) => formatTime(t, utc)),
        });
    } else {
        console.log(explain(schedule).join('\n'));
        console.log('');
        if (times.length) console.log(times.map((t) => formatTime(t, utc)).join('\n'));
    }
    if (exhausted) {
        throw new Error(
            times.length
                ? `Hanya ${times.length} waktu ditemukan; tidak ada lagi dalam ${HORIZON_YEARS} tahun setelahnya.`
                : `Tidak ada waktu yang cocok dalam ${HORIZON_YEARS} tahun sejak ${formatTime(from, utc)}.`,
        );
    }
}

export const cronTool: Tool = {
    name: 'cron',
    description: 'Jelaskan ekspresi cron dan tampilkan waktu eksekusi berikutnya',
    category: 'Utilitas',
    run,
};
