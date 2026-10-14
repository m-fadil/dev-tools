import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { describe, test } from 'node:test';
import { fileURLToPath } from 'node:url';

const CLI = fileURLToPath(new URL('../src/cli.js', import.meta.url));

const cli = (args: string[], input = '') => {
    const r = spawnSync(process.execPath, [CLI, ...args], { encoding: 'utf8', input, env: { PATH: process.env.PATH } });
    return { code: r.status, out: r.stdout, err: r.stderr };
};

const json = (args: string[]) => JSON.parse(cli([...args, '--json']).out) as Record<string, unknown>;

describe('cidr IPv4', () => {
    test('/24 details', () => {
        const d = json(['cidr', '192.168.1.10/24']);
        assert.equal(d.network, '192.168.1.0');
        assert.equal(d.netmask, '255.255.255.0');
        assert.equal(d.wildcard, '0.0.0.255');
        assert.equal(d.broadcast, '192.168.1.255');
        assert.equal(d.firstHost, '192.168.1.1');
        assert.equal(d.lastHost, '192.168.1.254');
        assert.equal(d.totalAddresses, '256');
        assert.equal(d.usableHosts, '254');
        assert.equal(d.private, true);
        assert.equal(d.loopback, false);
        assert.match(cli(['cidr', '192.168.1.10/24']).out, /Netmask\s+255\.255\.255\.0/);
    });

    test('netmask form equals prefix form', () => {
        assert.deepEqual(json(['cidr', '10.1.2.3', '255.255.0.0']), json(['cidr', '10.1.2.3/16']));
    });

    test('/31, /32, /0 boundaries', () => {
        const p31 = json(['cidr', '10.0.0.1/31']);
        assert.equal(p31.usableHosts, '2');
        assert.equal(p31.firstHost, '10.0.0.0');
        assert.equal(p31.lastHost, '10.0.0.1');
        const p32 = json(['cidr', '8.8.8.8/32']);
        assert.equal(p32.usableHosts, '1');
        assert.equal(p32.firstHost, '8.8.8.8');
        assert.equal(p32.lastHost, '8.8.8.8');
        assert.equal(p32.private, false);
        const p0 = json(['cidr', '1.2.3.4/0']);
        assert.equal(p0.netmask, '0.0.0.0');
        assert.equal(p0.totalAddresses, '4294967296');
        assert.equal(p0.usableHosts, '4294967294');
    });

    test('flags describe the whole subnet, not the input host', () => {
        assert.equal(json(['cidr', '172.16.5.1/12']).private, true);
        assert.equal(json(['cidr', '172.32.0.1/12']).private, false);
        assert.equal(json(['cidr', '127.0.0.1/8']).loopback, true);
        assert.equal(json(['cidr', '169.254.1.1/16']).linkLocal, true);
        assert.match(cli(['cidr', '10.0.0.1/8']).out, /Seluruh subnet\s+private \(RFC 1918\)/);
        // Network equal to the block, or inside it, is flagged.
        assert.equal(json(['cidr', '10.1.2.3/8']).private, true);
        assert.equal(json(['cidr', '10.1.2.3/32']).private, true);
        assert.equal(json(['cidr', '192.168.255.255/16']).private, true);
        // Prefixes shorter than the block cover addresses outside it.
        const allIPv4 = json(['cidr', '10.1.2.3/0']);
        assert.equal(allIPv4.private, false);
        assert.equal(allIPv4.loopback, false);
        assert.equal(allIPv4.linkLocal, false);
        assert.equal(json(['cidr', '10.0.0.0/7']).private, false);
        assert.equal(json(['cidr', '192.168.1.1/8']).private, false);
        assert.equal(json(['cidr', '172.16.0.1/11']).private, false);
        assert.equal(json(['cidr', '127.0.0.1/7']).loopback, false);
        assert.equal(json(['cidr', '127.0.0.1/0']).loopback, false);
        assert.equal(json(['cidr', '169.254.1.1/15']).linkLocal, false);
        assert.doesNotMatch(cli(['cidr', '10.1.2.3/0']).out, /private/);
    });

    test('strict parsing', () => {
        for (const bad of [
            '010.0.0.1/24',
            '1.2.3/24',
            '256.0.0.1/24',
            '1.2.3.4',
            '1.2.3.4/33',
            '1.2.3.4/024',
            '1.2.3.4/',
        ]) {
            assert.equal(cli(['cidr', bad]).code, 2, bad);
        }
        const r = cli(['cidr', '10.0.0.0', '255.0.255.0']);
        assert.equal(r.code, 2);
    });
});

describe('cidr IPv6', () => {
    test('details and compression', () => {
        const d = json(['cidr', '2001:db8:0:0:0:0:0:1/32']);
        assert.equal(d.network, '2001:db8::');
        assert.equal(d.address, '2001:db8::1');
        assert.equal(d.first, '2001:db8::');
        assert.equal(d.last, '2001:db8:ffff:ffff:ffff:ffff:ffff:ffff');
        assert.equal(d.totalAddresses, '79228162514264337593543950336');
        assert.equal(json(['cidr', '::/0']).totalAddresses, '340282366920938463463374607431768211456');
        assert.equal(json(['cidr', '::1/128']).totalAddresses, '1');
    });

    test('IPv4-mapped and strictness', () => {
        assert.equal(json(['cidr', '::ffff:1.2.3.4/128']).address, '::ffff:1.2.3.4');
        for (const bad of [
            '1::2::3/64',
            '1:2:3:4:5:6:7:8:9/64',
            '12345::/16',
            '::g/16',
            'fe80::1%eth0/64',
            '1:2:3:4:5:6:7/64',
            '::1/129',
        ]) {
            assert.equal(cli(['cidr', bad]).code, 2, bad);
        }
        assert.equal(cli(['cidr', '2001:db8::1', 'ffff::']).code, 2);
    });

    test('IPv6 formatter follows RFC 5952', () => {
        const addr = (text: string) => json(['cidr', `${text}/128`]).address;
        // Longest zero run wins; first one on a tie.
        assert.equal(addr('2001:0:0:1:0:0:0:1'), '2001:0:0:1::1');
        assert.equal(addr('2001:db8:0:0:1:0:0:1'), '2001:db8::1:0:0:1');
        // A single zero group is not compressed.
        assert.equal(addr('2001:db8:0:1:1:1:1:1'), '2001:db8:0:1:1:1:1:1');
        assert.equal(addr('1:2:3:4:5:6:7:0'), '1:2:3:4:5:6:7:0');
        // Trailing and leading runs.
        assert.equal(addr('1:0:0:0:0:0:0:0'), '1::');
        assert.equal(addr('0:0:0:0:0:0:0:1'), '::1');
        // Uppercase input is lowercased.
        assert.equal(addr('2001:DB8::ABCD'), '2001:db8::abcd');
        // A dotted tail outside ::ffff:/96 is printed as hex groups.
        assert.equal(addr('::1.2.3.4'), '::102:304');
    });

    test('"::" may stand for exactly one group', () => {
        const addr = (text: string) => json(['cidr', `${text}/128`]).address;
        assert.equal(addr('1:2:3:4:5:6:7::'), '1:2:3:4:5:6:7:0');
        assert.equal(addr('::2:3:4:5:6:7:8'), '0:2:3:4:5:6:7:8');
        assert.equal(addr('1:2:3:4::6:7:8'), '1:2:3:4:0:6:7:8');
        assert.equal(cli(['cidr', '1:2:3:4:5:6:7:8::/128']).code, 2);
    });
});

describe('cidr --contains', () => {
    test('yes/no with exit code', () => {
        assert.deepEqual(cli(['cidr', '--contains', '10.0.0.77', '10.0.0.0/24']), { code: 0, out: 'yes\n', err: '' });
        const no = cli(['cidr', '--contains', '10.0.1.0', '10.0.0.0/24']);
        assert.equal(no.code, 1);
        assert.equal(no.out, 'no\n');
        assert.equal(cli(['cidr', '--contains', '2001:db8::5', '2001:db8::/32']).code, 0);
    });

    test('version mismatch is no; invalid ip is usage error', () => {
        assert.equal(cli(['cidr', '--contains', '::ffff:10.0.0.1', '10.0.0.0/8']).code, 1);
        assert.equal(cli(['cidr', '--contains', '10.0.0.1', '::/0']).code, 1);
        assert.equal(cli(['cidr', '--contains', 'nope', '10.0.0.0/8']).code, 2);
    });

    test('json shape for a false result', () => {
        const r = cli(['cidr', '--contains', '10.0.1.0', '10.0.0.5/24', '--json']);
        assert.equal(r.code, 1);
        assert.deepEqual(JSON.parse(r.out), { network: '10.0.0.0/24', address: '10.0.1.0', contains: false });
    });

    test('json shape for a true result', () => {
        const r = cli(['cidr', '--contains', '10.0.0.77', '10.0.0.5/24', '--json']);
        assert.equal(r.code, 0);
        assert.deepEqual(JSON.parse(r.out), { network: '10.0.0.0/24', address: '10.0.0.77', contains: true });
    });
});

describe('cidr --split', () => {
    test('lists subnets', () => {
        assert.equal(
            cli(['cidr', '--split', '26', '10.0.0.0/24']).out,
            '10.0.0.0/26\n10.0.0.64/26\n10.0.0.128/26\n10.0.0.192/26\n',
        );
        assert.equal(cli(['cidr', '--split', '24', '10.0.0.0/24']).out, '10.0.0.0/24\n');
        assert.equal(
            cli(['cidr', '--split', '34', '2001:db8::/32']).out.trim().split('\n').at(-1),
            '2001:db8:c000::/34',
        );
        const d = json(['cidr', '--split', '25', '10.0.0.0/24']);
        assert.deepEqual(d, {
            network: '10.0.0.0/24',
            prefix: 25,
            count: 2,
            subnets: ['10.0.0.0/25', '10.0.0.128/25'],
        });
    });

    test('limits', () => {
        assert.equal(cli(['cidr', '--split', '23', '10.0.0.0/24']).code, 2);
        assert.equal(cli(['cidr', '--split', '33', '10.0.0.0/24']).code, 2);
        assert.equal(cli(['cidr', '--split', '24', '10.0.0.0/12']).out.trim().split('\n').length, 4096);
        assert.equal(cli(['cidr', '--split', '25', '10.0.0.0/12']).code, 2);
        assert.equal(cli(['cidr', '--split', '26', '--contains', '1.1.1.1', '10.0.0.0/24']).code, 2);
    });
});
