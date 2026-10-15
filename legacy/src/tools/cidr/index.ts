import { UsageError } from '../../errors.js';
import { integer, parseToolArgs, printJson } from '../../io.js';
import type { Tool } from '../../tool.js';
import { describe, render } from './describe.js';
import { BITS, formatAddress, makeNetwork, parseAddress, parseNetwork } from './ip.js';

const MAX_SPLIT = 4096;

function help(): void {
    console.log(`dt cidr [options] <ip/prefix | ip netmask>

Hitung detail subnet IPv4 dan IPv6 (aritmetika BigInt, parsing ketat).
  --contains <ip>     Cek apakah ip ada di subnet: cetak yes/no, exit 0 (ya) / 1 (tidak)
  --split <prefix>    Daftar subnet dengan panjang prefix tersebut (maks ${MAX_SPLIT} subnet)
      --json          Keluaran JSON
  -h, --help          Tampilkan bantuan

Bentuk "ip netmask" (mis. 10.0.0.0 255.255.255.0) hanya untuk IPv4. Prefix wajib.
Oktet berawalan nol (010) ditolak karena ambigu. Versi IP berbeda dianggap bukan anggota.
Exit: 0 sukses, 2 argumen/alamat tidak valid; dengan --contains, 1 berarti "no".
Contoh: dt cidr 192.168.1.10/24   |   dt cidr --split 26 10.0.0.0/24
        dt cidr --contains 2001:db8::1 2001:db8::/32`);
}

async function run(args: string[]): Promise<void> {
    const { values, positionals } = parseToolArgs(args, {
        contains: { type: 'string' },
        split: { type: 'string' },
        json: { type: 'boolean' },
        help: { type: 'boolean', short: 'h' },
    });
    if (values.help) {
        help();
        return;
    }
    if (values.contains !== undefined && values.split !== undefined) {
        throw new UsageError('--contains dan --split tidak bisa digabung.');
    }
    const parse = <T>(fn: () => T): T => {
        try {
            return fn();
        } catch (error) {
            throw new UsageError((error as Error).message, { cause: error });
        }
    };
    const net = parse(() => parseNetwork(positionals));

    if (values.contains !== undefined) {
        const ip = parse(() => parseAddress(values.contains as string));
        const yes = ip.version === net.version && ip.value >= net.network && ip.value <= net.last;
        if (values.json) {
            printJson({
                network: `${formatAddress(net.version, net.network)}/${net.prefix}`,
                address: formatAddress(ip.version, ip.value),
                contains: yes,
            });
        } else {
            console.log(yes ? 'yes' : 'no');
        }
        if (!yes) process.exitCode = 1;
        return;
    }

    if (values.split !== undefined) {
        const bits = BITS[net.version];
        const target = integer(values.split, 0, 0, bits, 'split');
        if (target < net.prefix) {
            throw new UsageError(`Prefix --split (${target}) tidak boleh lebih pendek dari /${net.prefix}.`);
        }
        const count = 1n << BigInt(target - net.prefix);
        if (count > BigInt(MAX_SPLIT)) {
            throw new UsageError(`Hasil ${count} subnet melebihi batas ${MAX_SPLIT}; pilih prefix lebih pendek.`);
        }
        const size = 1n << BigInt(bits - target);
        const subnets = Array.from(
            { length: Number(count) },
            (_, i) => `${formatAddress(net.version, net.network + BigInt(i) * size)}/${target}`,
        );
        if (values.json) {
            printJson({
                network: `${formatAddress(net.version, net.network)}/${net.prefix}`,
                prefix: target,
                count: subnets.length,
                subnets,
            });
        } else {
            console.log(subnets.join('\n'));
        }
        return;
    }

    const description = describe(makeNetwork(net.version, net.address, net.prefix));
    if (values.json) printJson(description);
    else console.log(render(description));
}

export const cidrTool: Tool = {
    name: 'cidr',
    description: 'Hitung subnet IPv4/IPv6: netmask, broadcast, rentang host, contains, split',
    category: 'Jaringan',
    run,
};
