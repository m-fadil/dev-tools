import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { describe, test } from 'node:test';
import { fileURLToPath } from 'node:url';

const CLI = fileURLToPath(new URL('../src/cli.js', import.meta.url));

const cli = (args: string[], input = '') => {
    const r = spawnSync(process.execPath, [CLI, ...args], { encoding: 'utf8', input, env: { PATH: process.env.PATH } });
    return { code: r.status, out: r.stdout, err: r.stderr };
};

const lines = (args: string[]) => {
    const r = cli(['fake', ...args]);
    assert.equal(r.code, 0, r.err);
    return r.out.trimEnd().split('\n');
};

const TYPES =
    'name firstname lastname email username phone company address city country ip ipv6 mac uuid date datetime number bool color url word sentence hex'.split(
        ' ',
    );

describe('fake', () => {
    test('no type lists types, unknown type is a usage error', () => {
        const r = cli(['fake']);
        assert.equal(r.code, 0);
        for (const t of TYPES) assert.ok(r.out.includes(`  ${t}\n`), t);
        const bad = cli(['fake', 'nope']);
        assert.equal(bad.code, 2);
        assert.ok(bad.err.includes('hex'));
    });

    test('every type works and is reproducible with --seed', () => {
        for (const type of TYPES) {
            // default date bounds depend on the current time
            const range = type.startsWith('date') ? ['--from', '2015-01-01', '--to', '2024-01-01'] : [];
            for (const locale of ['en', 'id']) {
                const a = lines([type, '-c', '5', '--seed', '9', '--locale', locale, ...range]);
                assert.equal(a.length, 5, type);
                assert.deepEqual(lines([type, '-c', '5', '--seed', '9', '--locale', locale, ...range]), a, type);
            }
        }
    });

    test('emails use reserved domains', () => {
        for (const e of lines(['email', '-c', '50', '--seed', '1']))
            assert.match(e, /^[a-z]+\.[a-z]+\d*@example\.(com|org|net)$/);
        for (const e of lines(['email', '-c', '50', '--seed', '1', '--locale', 'id']))
            assert.match(e, /@example\.(com|org|net)$/);
    });

    test('en phones use fictional ranges; id phones are only format-plausible', () => {
        for (const p of lines(['phone', '-c', '30', '--seed', '2'])) assert.match(p, /^\+1 \d{3}-555-01\d{2}$/);
        for (const p of lines(['phone', '-c', '30', '--seed', '2', '--locale', 'id']))
            assert.match(p, /^\+62 8\d{2}-\d{4}-\d{4}$/);
    });

    test('help warns that id phone numbers may belong to real subscribers', () => {
        const r = cli(['fake', '--help']);
        assert.equal(r.code, 0, r.err);
        assert.match(r.out, /telepon en dari rentang fiktif NANP 555-01xx/);
        assert.match(r.out, /pelanggan nyata/);
        assert.match(r.out, /jangan dipakai untuk menghubungi siapa pun/);
    });

    test('network values stay in documentation ranges', () => {
        for (const ip of lines(['ip', '-c', '60', '--seed', '3']))
            assert.match(ip, /^(192\.0\.2|198\.51\.100|203\.0\.113)\.\d{1,3}$/);
        for (const ip of lines(['ipv6', '-c', '20', '--seed', '3'])) assert.match(ip, /^2001:db8(:[0-9a-f]{1,4}){6}$/);
        for (const mac of lines(['mac', '-c', '20', '--seed', '3'])) {
            assert.match(mac, /^([0-9a-f]{2}:){5}[0-9a-f]{2}$/);
            assert.equal(parseInt(mac.slice(0, 2), 16) & 3, 2);
        }
    });

    test('uuid, color, hex, bool, url formats', () => {
        for (const u of lines(['uuid', '-c', '20', '--seed', '4']))
            assert.match(u, /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/);
        for (const c of lines(['color', '-c', '10', '--seed', '4'])) assert.match(c, /^#[0-9a-f]{6}$/);
        for (const h of lines(['hex', '-c', '3', '--length', '40', '--seed', '4'])) assert.match(h, /^[0-9a-f]{40}$/);
        assert.match(lines(['hex'])[0]!, /^[0-9a-f]{16}$/);
        for (const b of lines(['bool', '-c', '10', '--seed', '4'])) assert.match(b, /^(true|false)$/);
        for (const u of lines(['url', '-c', '10', '--seed', '4']))
            assert.match(u, /^https:\/\/www\.example\.(com|org|net)\//);
    });

    test('number honours bounds', () => {
        for (const n of lines(['number', '-c', '200', '--min=-5', '--max', '5', '--seed', '5'])) {
            assert.ok(Number(n) >= -5 && Number(n) <= 5 && Number.isInteger(Number(n)));
        }
        assert.deepEqual(lines(['number', '-c', '3', '--min', '7', '--max', '7']), ['7', '7', '7']);
        assert.equal(cli(['fake', 'number', '--min', '9', '--max', '1']).code, 2);
        assert.equal(cli(['fake', 'number', '--min', 'x']).code, 2);
    });

    test('number derives the implicit maximum and rejects spans beyond the 32-bit RNG', () => {
        assert.deepEqual(lines(['number', '--min', '5000', '-c', '3']), ['5000', '5000', '5000']);
        assert.deepEqual(lines(['number', '--min', '9007199254740991']), ['9007199254740991']);
        for (const args of [
            ['--max', '4294967296'],
            ['--max', '9007199254740991'],
            ['--min=-2147483648', '--max', '2147483648'],
        ]) {
            const r = cli(['fake', 'number', ...args]);
            assert.equal(r.code, 2, args.join(' '));
            assert.equal(r.out, '');
            assert.match(r.err, /4294967295 \(2\^32-1\)/);
        }
        for (const n of lines(['number', '--max', '4294967295', '--seed', '5', '-c', '30'])) {
            assert.ok(Number.isInteger(Number(n)) && Number(n) >= 0 && Number(n) <= 4294967295);
        }
        for (const n of lines([
            'number',
            '--min',
            '9007194959773696',
            '--max',
            '9007199254740991',
            '--seed',
            '5',
            '-c',
            '30',
        ])) {
            assert.ok(
                Number.isSafeInteger(Number(n)) && Number(n) >= 9007194959773696 && Number(n) <= 9007199254740991,
            );
        }
    });

    test('date and datetime honour bounds', () => {
        for (const d of lines(['date', '-c', '100', '--from', '2020-02-28', '--to', '2020-03-01', '--seed', '6'])) {
            assert.ok(['2020-02-28', '2020-02-29', '2020-03-01'].includes(d), d);
        }
        for (const d of lines([
            'datetime',
            '-c',
            '50',
            '--from',
            '2020-01-01T00:00:00Z',
            '--to',
            '2020-01-01T00:00:10Z',
            '--seed',
            '6',
        ])) {
            assert.match(d, /^2020-01-01T00:00:(0\d|10)Z$/);
        }
        assert.match(lines(['date'])[0]!, /^\d{4}-\d{2}-\d{2}$/);
        for (const args of [
            ['--from', '2020-02-30'],
            ['--from', 'abc'],
            ['--from', '2021-01-01', '--to', '2020-01-01'],
        ]) {
            assert.equal(cli(['fake', 'date', ...args]).code, 2, args.join(' '));
        }
    });

    test('fractional date bounds are rejected rather than silently dropping milliseconds', () => {
        for (const type of ['date', 'datetime']) {
            for (const option of ['--from', '--to']) {
                const r = cli(['fake', type, option, '2020-01-01T00:00:00.125Z']);
                assert.equal(r.code, 2, `${type} ${option}`);
                assert.equal(r.out, '');
                assert.match(r.err, /ISO 8601/);
            }
        }
    });

    test('option validation exits 2', () => {
        for (const args of [
            ['name', '-c', '0'],
            ['name', '-c', '1001'],
            ['name', '--locale', 'fr'],
            ['name', '--min', '1'],
            ['name', '--length', '4'],
            ['name', 'extra'],
            ['name', '--from', '2020-01-01'],
            ['hex', '--length', '0'],
            ['name', '--seed', 'x'],
        ]) {
            const r = cli(['fake', ...args]);
            assert.equal(r.code, 2, args.join(' '));
            assert.equal(r.out, '');
        }
    });

    test('--json shape keeps native types', () => {
        const j = JSON.parse(cli(['fake', 'number', '-c', '3', '--seed', '1', '--json']).out);
        assert.deepEqual(Object.keys(j), ['type', 'locale', 'values']);
        assert.equal(j.type, 'number');
        assert.equal(j.locale, 'en');
        assert.equal(j.values.length, 3);
        assert.ok(j.values.every((v: unknown) => typeof v === 'number'));
        const b = JSON.parse(cli(['fake', 'bool', '-c', '2', '--locale', 'id', '--json']).out);
        assert.equal(b.locale, 'id');
        assert.ok(b.values.every((v: unknown) => typeof v === 'boolean'));
    });
});
