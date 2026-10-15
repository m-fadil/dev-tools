import { randomBytes } from 'node:crypto';
import { UsageError } from '../../errors.js';
import { choice, parseToolArgs, printJson, readInput } from '../../io.js';
import type { Tool } from '../../tool.js';
import { FORMATS, formatMac, macFlags, parseMac } from './mac.js';

function help(): void {
    console.log(`dt mac [options] [address]

Parse dan konversi alamat MAC (EUI-48). Tanpa address, dibaca dari stdin.
Bentuk input: aa:bb:cc:dd:ee:ff, aa-bb-cc-dd-ee-ff, aabb.ccdd.eeff (Cisco), aabbccddeeff.
  -f, --format <jenis>  Cetak satu bentuk saja: ${FORMATS.join(', ')}
      --upper           Huruf kapital
      --random          Buat alamat acak: unicast, locally administered (tanpa address)
      --json            Keluaran JSON
  -h, --help            Tampilkan bantuan

Tanpa --format dicetak semua bentuk beserta flag unicast/multicast (bit I/G) dan
universally/locally administered (bit U/L). Alamat tidak valid: exit 2.
Contoh: dt mac AA-BB-CC-DD-EE-FF   |   dt mac -f dot 00:1a:2b:3c:4d:5e   |   dt mac --random`);
}

async function run(args: string[]): Promise<void> {
    const { values, positionals } = parseToolArgs(args, {
        format: { type: 'string', short: 'f' },
        upper: { type: 'boolean' },
        random: { type: 'boolean' },
        json: { type: 'boolean' },
        help: { type: 'boolean', short: 'h' },
    });
    if (values.help) {
        help();
        return;
    }
    const format = values.format === undefined ? undefined : choice(values.format, FORMATS, 'colon', 'format');
    const upper = values.upper ?? false;

    let hex: string;
    if (values.random) {
        if (positionals.length) throw new UsageError('--random tidak menerima address.');
        const bytes = randomBytes(6);
        // Clear the multicast bit, set the locally administered bit.
        bytes[0] = ((bytes[0] as number) & 0xfe) | 0x02;
        hex = bytes.toString('hex');
    } else {
        if (positionals.length > 1) throw new UsageError(`Argumen berlebih: ${positionals[1]}`);
        const raw = positionals.length
            ? (positionals[0] as string)
            : (await readInput([], undefined, 'Alamat MAC')).toString('utf8');
        hex = parseMac(raw);
    }

    const flags = macFlags(hex);
    if (values.json) {
        printJson({
            address: formatMac(hex, format ?? 'colon', upper),
            formats: Object.fromEntries(FORMATS.map((f) => [f, formatMac(hex, f, upper)])),
            multicast: flags.multicast,
            locallyAdministered: flags.locallyAdministered,
        });
        return;
    }
    if (format) {
        console.log(formatMac(hex, format, upper));
        return;
    }
    const lines = FORMATS.map((f) => `${f.padEnd(7)} ${formatMac(hex, f, upper)}`);
    lines.push(
        `${'tipe'.padEnd(7)} ${flags.multicast ? 'multicast' : 'unicast'}`,
        `${'admin'.padEnd(7)} ${flags.locallyAdministered ? 'locally administered' : 'universally administered'}`,
    );
    console.log(lines.join('\n'));
}

export const macTool: Tool = {
    name: 'mac',
    description: 'Parse dan konversi alamat MAC (colon, hyphen, dot, plain), flag, dan alamat acak',
    category: 'Jaringan',
    run,
};
