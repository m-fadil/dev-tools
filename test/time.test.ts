import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { describe, test } from 'node:test';
import { fileURLToPath } from 'node:url';

const CLI = fileURLToPath(new URL('../src/cli.js', import.meta.url));

/** run the CLI without a terminal; `input` is piped to stdin */
const cli = (args: string[], input = '', env: Record<string, string> = {}) => {
    const r = spawnSync(process.execPath, [CLI, ...args], {
        encoding: 'utf8',
        input,
        env: { PATH: process.env.PATH, ...env },
    });
    return { code: r.status, out: r.stdout, err: r.stderr };
};

const NOW = '2026-01-02T03:04:05Z';

describe('time', () => {
    test('epoch seconds in UTC and Asia/Jakarta with relative output', () => {
        const r = cli(['time', '-z', 'Asia/Jakarta', '--now', NOW, '1767323045']);
        assert.equal(r.code, 0);
        const lines = r.out.trimEnd().split('\n');
        assert.deepEqual(
            lines.map((l) => l.replace(/\s+/g, ' ')),
            [
                'UTC: 2026-01-02T03:04:05.000Z',
                'Asia/Jakarta: 2026-01-02T10:04:05.000+07:00',
                'Epoch (s): 1767323045',
                'Epoch (ms): 1767323045000',
                'Relatif: sekarang',
            ],
        );
    });

    test('epoch unit detection, override, and boundary at 1e11', () => {
        const ms = cli(['time', '-z', 'UTC', '--now', NOW, '1767323045123']).out;
        assert.match(ms, /2026-01-02T03:04:05\.123Z/);
        // 99999999999 is still seconds (year 5138); 100000000000 is ms (1973)
        assert.match(cli(['time', '-z', 'UTC', '99999999999']).out, /^UTC:\s+5138-11-16T09:46:39\.000Z/);
        assert.match(cli(['time', '-z', 'UTC', '100000000000']).out, /^UTC:\s+1973-03-03T09:46:40\.000Z/);
        assert.match(cli(['time', '-z', 'UTC', '-u', 'ms', '1000']).out, /^UTC:\s+1970-01-01T00:00:01\.000Z/);
        assert.match(cli(['time', '-z', 'UTC', '-u', 's', '1767323045']).out, /^UTC:\s+2026-01-02T03:04:05\.000Z/);
    });

    test('negative epoch needs -- and gives pre-1970 dates', () => {
        const r = cli(['time', '-z', 'UTC', '--', '-86400']);
        assert.equal(r.code, 0);
        assert.match(r.out, /^UTC:\s+1969-12-31T00:00:00\.000Z/);
        assert.match(r.out, /Epoch \(s\):\s+-86400/);
    });

    test('ISO without offset is read in the chosen zone, with offset it is absolute', () => {
        const jkt = cli(['time', '-z', 'Asia/Jakarta', '2026-01-02T10:04:05']).out;
        assert.match(jkt, /^UTC:\s+2026-01-02T03:04:05\.000Z/);
        const ny = cli(['time', '-z', 'America/New_York', '2026-07-01T12:00:00']).out;
        assert.match(ny, /^UTC:\s+2026-07-01T16:00:00\.000Z/);
        assert.match(ny, /America\/New_York:\s+2026-07-01T12:00:00\.000-04:00/);
        const abs = cli(['time', '-z', 'Asia/Tokyo', '2026-01-02T10:04:05.5+07:00']).out;
        assert.match(abs, /^UTC:\s+2026-01-02T03:04:05\.500Z/);
        assert.match(abs, /Asia\/Tokyo:\s+2026-01-02T12:04:05\.500\+09:00/);
        // date only = midnight in the zone
        assert.match(cli(['time', '-z', 'Asia/Jakarta', '2026-01-02']).out, /^UTC:\s+2026-01-01T17:00:00\.000Z/);
    });

    test('DST: winter and summer offsets, half-hour zone', () => {
        assert.match(
            cli(['time', '-z', 'Europe/Berlin', '2026-01-15T12:00:00']).out,
            /2026-01-15T12:00:00\.000\+01:00/,
        );
        assert.match(
            cli(['time', '-z', 'Europe/Berlin', '2026-07-15T12:00:00']).out,
            /2026-07-15T12:00:00\.000\+02:00/,
        );
        assert.match(cli(['time', '-z', 'Asia/Kolkata', NOW]).out, /2026-01-02T08:34:05\.000\+05:30/);
    });

    test('relative output picks the largest sensible unit, past and future', () => {
        const rel = (v: string) => cli(['time', '-z', 'UTC', '--now', NOW, v]).out.match(/Relatif:\s+(.*)/)![1];
        assert.equal(rel('2026-01-02T00:04:05Z'), '3 jam lalu');
        assert.equal(rel('2026-01-04T03:04:05Z'), '2 hari lagi');
        assert.equal(rel('2026-01-02T03:03:05Z'), '1 menit lalu');
        assert.equal(rel('2026-01-02T03:04:35Z'), '30 detik lagi');
        assert.equal(rel('2026-03-04T03:04:05Z'), '2 bulan lagi');
        assert.equal(rel('2023-01-02T03:04:05Z'), '3 tahun lalu');
    });

    test('--now accepts epoch', () => {
        const r = cli(['time', '-z', 'UTC', '--now', '1767323045', '1767326645']);
        assert.match(r.out, /Relatif:\s+1 jam lagi/);
    });

    test('no value prints the current time', () => {
        const r = cli(['time', '-z', 'UTC']);
        assert.equal(r.code, 0);
        assert.match(r.out, /Relatif:\s+sekarang/);
    });

    test('invalid input is a usage error', () => {
        for (const bad of [
            ['time', 'kemarin'],
            ['time', '2026-02-30'],
            ['time', '2026-13-01'],
            ['time', '2026-01-02T24:00:00'],
            ['time', '2026-01-02T10:00:00+25:00'],
            ['time', '1.5'],
            ['time', '9999999999999999'],
            ['time', '-u', 's', '1767323045000000000'],
            ['time', '-u', 'us', '1'],
            ['time', '-z', 'Mars/Olympus', '1'],
            ['time', '--now', 'bukan-waktu', '1'],
            ['time', '1', '2'],
        ]) {
            const r = cli(bad);
            assert.equal(r.code, 2, bad.join(' '));
            assert.equal(r.out, '');
            assert.notEqual(r.err, '');
        }
    });

    test('range limits of Date', () => {
        const max = cli(['time', '-z', 'UTC', '8640000000000000']);
        assert.equal(max.code, 0);
        assert.match(max.out, /^UTC:\s+\+275760-09-13T00:00:00\.000Z/);
        assert.equal(cli(['time', '-z', 'UTC', '8640000000000001']).code, 2);
        assert.equal(cli(['time', '-z', 'UTC', '--', '-8640000000000001']).code, 2);
    });
});

describe('time --json', () => {
    test('emits one document with the documented fields', () => {
        const r = cli(['time', '--json', '-z', 'Asia/Jakarta', '--now', NOW, '1767323045']);
        assert.equal(r.code, 0);
        assert.deepEqual(JSON.parse(r.out), {
            input: '1767323045',
            utc: '2026-01-02T03:04:05.000Z',
            zone: { name: 'Asia/Jakarta', iso: '2026-01-02T10:04:05.000+07:00' },
            epochSeconds: 1767323045,
            epochMillis: 1767323045000,
            relative: JSON.parse(r.out).relative,
        });
        assert.equal(typeof JSON.parse(r.out).relative, 'string');
    });

    test('input is null without a value; invalid input still exits 2', () => {
        const r = cli(['time', '--json', '-z', 'UTC', '--now', NOW]);
        assert.equal(r.code, 0);
        assert.equal(JSON.parse(r.out).input, null);
        assert.equal(cli(['time', '--json', 'bukan-waktu']).code, 2);
    });

    test('renders at the Date range limits with a valid offset', () => {
        for (const [zone, value, utc] of [
            ['Asia/Jakarta', '+275760-09-13T00:00:00Z', '+275760-09-13T00:00:00.000Z'],
            ['America/New_York', '-8640000000000000', '-271821-04-20T00:00:00.000Z'],
        ] as const) {
            const r = cli(['time', '-z', zone, '--now', NOW, '--json', '--', value]);
            assert.equal(r.code, 0, r.err);
            const doc = JSON.parse(r.out);
            const iso: string = doc.zone.iso;
            assert.equal(doc.utc, utc);
            assert.doesNotMatch(iso, /NaN/);
            assert.match(iso, /[+-]\d{2}:\d{2}(?::\d{2})?$/);
            if (zone === 'Asia/Jakarta') assert.equal(iso, '+275760-09-13T07:00:00.000+07:00');
            const text = cli(['time', '-z', zone, '--now', NOW, '--', value]);
            assert.equal(text.code, 0, text.err);
            assert.ok(text.out.includes(iso));
            assert.ok(text.out.includes(utc));
        }
    });

    test('DST gap shifts forward by the gap length; overlap picks the earlier occurrence', () => {
        for (const [zone, value, utc, iso] of [
            ['America/New_York', '2026-03-08T02:30', '2026-03-08T07:30:00.000Z', '2026-03-08T03:30:00.000-04:00'],
            ['Europe/Berlin', '2026-03-29T02:30', '2026-03-29T01:30:00.000Z', '2026-03-29T03:30:00.000+02:00'],
            ['America/New_York', '2026-11-01T01:30', '2026-11-01T05:30:00.000Z', '2026-11-01T01:30:00.000-04:00'],
            ['Europe/Berlin', '2026-10-25T02:30', '2026-10-25T00:30:00.000Z', '2026-10-25T02:30:00.000+02:00'],
        ] as const) {
            const r = cli(['time', '-z', zone, '--json', value]);
            assert.equal(r.code, 0, r.err);
            const doc = JSON.parse(r.out);
            assert.equal(doc.utc, utc);
            assert.equal(doc.zone.iso, iso);
        }
    });

    test('--unit is rejected for ISO input and applies to --now epoch values', () => {
        for (const args of [
            ['-u', 'ms', '2026-01-02'],
            ['-u', 'ms', '--now', NOW],
            ['-u', 'ms', '--now', NOW, '1000'],
            ['-u', 'ms', '--now', '5000', NOW],
            ['-u', 'ms'],
        ]) {
            const r = cli(['time', ...args]);
            assert.equal(r.code, 2, args.join(' '));
            assert.equal(r.out, '');
            assert.match(r.err, /--unit/);
        }
        for (const [unit, now, value, millis] of [
            ['ms', '5000', '1000', 1000],
            ['s', '5', '1', 1000],
        ] as const) {
            const r = cli(['time', '-z', 'UTC', '-u', unit, '--now', now, value, '--json']);
            assert.equal(r.code, 0, r.err);
            const doc = JSON.parse(r.out);
            assert.equal(doc.relative, '4 detik lalu');
            assert.equal(doc.epochMillis, millis);
        }
        const nowOnly = cli(['time', '-z', 'UTC', '-u', 'ms', '--now', '5000', '--json']);
        assert.equal(nowOnly.code, 0, nowOnly.err);
        assert.equal(JSON.parse(nowOnly.out).epochMillis, 5000);
        assert.equal(JSON.parse(nowOnly.out).relative, 'sekarang');
    });
});
