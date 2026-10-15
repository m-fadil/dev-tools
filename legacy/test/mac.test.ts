import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { describe, test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { macTool } from '../src/tools/mac/index.js';

const CLI = fileURLToPath(new URL('../src/cli.js', import.meta.url));

const cli = (args: string[], input = '') => {
    const r = spawnSync(process.execPath, [CLI, ...args], { encoding: 'utf8', input, env: { PATH: process.env.PATH } });
    return { code: r.status, out: r.stdout, err: r.stderr };
};

describe('mac', () => {
    test('all input forms normalise to the same address', () => {
        for (const form of ['00:1A:2B:3C:4D:5E', '00-1a-2b-3c-4d-5e', '001a.2b3c.4d5e', '001A2B3C4D5E']) {
            assert.equal(cli(['mac', '-f', 'colon', form]).out, '00:1a:2b:3c:4d:5e\n', form);
        }
    });

    test('formats, --upper, stdin', () => {
        assert.equal(cli(['mac', '-f', 'hyphen', 'aabb.ccdd.eeff']).out, 'aa-bb-cc-dd-ee-ff\n');
        assert.equal(cli(['mac', '--format', 'dot', '--upper', 'aa:bb:cc:dd:ee:ff']).out, 'AABB.CCDD.EEFF\n');
        assert.equal(cli(['mac', '-f', 'plain'], '00-1A-2B-3C-4D-5E\n').out, '001a2b3c4d5e\n');
        assert.equal(
            cli(['mac', '00:1a:2b:3c:4d:5e']).out,
            'colon   00:1a:2b:3c:4d:5e\nhyphen  00-1a-2b-3c-4d-5e\ndot     001a.2b3c.4d5e\nplain   001a2b3c4d5e\n' +
                'tipe    unicast\nadmin   universally administered\n',
        );
    });

    test('flags', () => {
        const flags = (a: string) => JSON.parse(cli(['mac', a, '--json']).out) as Record<string, unknown>;
        assert.equal(flags('00:00:00:00:00:00').multicast, false);
        assert.equal(flags('00:00:00:00:00:00').locallyAdministered, false);
        assert.equal(flags('01:00:5e:00:00:01').multicast, true);
        assert.equal(flags('02:00:00:00:00:00').locallyAdministered, true);
        assert.equal(flags('ff:ff:ff:ff:ff:ff').multicast, true);
        assert.match(cli(['mac', '03:00:00:00:00:00']).out, /multicast[\s\S]*locally administered/);
    });

    test('json shape', () => {
        const d = JSON.parse(cli(['mac', '-f', 'dot', '--upper', '--json', 'aa:bb:cc:dd:ee:ff']).out);
        assert.deepEqual(d, {
            address: 'AABB.CCDD.EEFF',
            formats: {
                colon: 'AA:BB:CC:DD:EE:FF',
                hyphen: 'AA-BB-CC-DD-EE-FF',
                dot: 'AABB.CCDD.EEFF',
                plain: 'AABBCCDDEEFF',
            },
            multicast: false,
            locallyAdministered: true,
        });
    });

    test('invalid addresses are usage errors (exit 2)', () => {
        for (const bad of [
            'aa:bb:cc:dd:ee',
            'aa:bb:cc:dd:ee:gg',
            'aa-bb:cc-dd:ee-ff',
            'aabb.ccdd',
            'aabbccddeeff00',
            'a:b:c:d:e:f',
            '',
        ]) {
            assert.equal(cli(['mac', bad], '').code, 2, `argument ${bad}`);
            assert.equal(cli(['mac'], bad).code, 2, `stdin ${bad}`);
        }
    });

    test('surrounding whitespace is trimmed for argument and stdin', () => {
        for (const padded of ['aa:bb:cc:dd:ee:ff\n', '  aa:bb:cc:dd:ee:ff \t', '\naabbccddeeff']) {
            const arg = cli(['mac', '-f', 'hyphen', padded]);
            assert.equal(arg.code, 0, JSON.stringify(padded));
            assert.equal(arg.out, 'aa-bb-cc-dd-ee-ff\n');
            assert.deepEqual(cli(['mac', '-f', 'hyphen'], padded), arg);
        }
    });

    test('error echoes the trimmed text without surrounding newline', () => {
        const input = ' \tzz:bb:cc:dd:ee:ff\n';
        const r = cli(['mac', input]);
        assert.equal(r.code, 2);
        assert.ok(r.err.includes('zz:bb:cc:dd:ee:ff ('), r.err);
        assert.deepEqual(cli(['mac'], input), r);
    });

    test('usage errors', () => {
        assert.equal(cli(['mac', '-f', 'octal', 'aabbccddeeff']).code, 2);
        assert.equal(cli(['mac', 'aabbccddeeff', '112233445566']).code, 2);
        assert.equal(cli(['mac', '--random', 'aabbccddeeff']).code, 2);
    });

    test('--random is locally administered unicast', () => {
        for (let i = 0; i < 3; i++) {
            const d = JSON.parse(cli(['mac', '--random', '--json']).out);
            assert.equal(d.multicast, false);
            assert.equal(d.locallyAdministered, true);
            assert.match(d.address, /^[0-9a-f]{2}(:[0-9a-f]{2}){5}$/);
        }
        assert.match(cli(['mac', '--random', '-f', 'plain', '--upper']).out, /^[0-9A-F]{12}\n$/);
    });

    test('--random varies across runs and fills the free 46 bits', async () => {
        // One process: run the tool repeatedly with console.log captured.
        const original = console.log;
        const addresses: string[] = [];
        try {
            console.log = (line: string) => addresses.push(line);
            for (let i = 0; i < 64; i++) {
                await macTool.run(['--random', '-f', 'plain']);
            }
        } finally {
            console.log = original;
        }
        assert.equal(addresses.length, 64);
        assert.ok(new Set(addresses).size > 60, 'addresses must not repeat');
        let seenOne = 0n;
        let seenZero = 0n;
        for (const a of addresses) {
            const bits = BigInt(`0x${a}`);
            assert.equal((bits >> 40n) & 0x03n, 0x02n, 'U/L set, I/G clear');
            seenOne |= bits;
            seenZero |= ~bits & 0xffffffffffffn;
        }
        // Every bit except the two fixed ones has been observed both as 0 and as 1.
        const free = 0xffffffffffffn ^ (0x03n << 40n);
        assert.equal(seenOne & free, free);
        assert.equal(seenZero & free, free);
    });
});
