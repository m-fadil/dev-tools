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

/** fire times only (the lines starting with a year) */
const times = (out: string) => out.split('\n').filter((l) => /^\d{4}-/.test(l));

const utc = (expr: string[], from: string, n = 5) => cli(['cron', '--utc', '--from', from, '-n', String(n), ...expr]);

describe('cron', () => {
    test('steps, ranges and weekday names; explanation per field', () => {
        const r = utc(['*/15 9-10 * * mon-fri'], '2025-01-03T10:30:00Z', 4); // Friday
        assert.equal(r.code, 0);
        assert.match(r.out, /^menit: 0, 15, 30, 45$/m);
        assert.match(r.out, /^jam: 9, 10$/m);
        assert.match(r.out, /^hari: Senin, Selasa, Rabu, Kamis, Jumat$/m);
        assert.deepEqual(times(r.out), [
            '2025-01-03T10:45:00Z',
            '2025-01-06T09:00:00Z',
            '2025-01-06T09:15:00Z',
            '2025-01-06T09:30:00Z',
        ]);
    });

    test('five separate positionals equal one quoted expression; macros and names are case-insensitive', () => {
        const a = utc(['0', '12', '1', 'JAN', '*'], '2025-01-01T00:00:00Z', 2);
        const b = utc(['0 12 1 jan *'], '2025-01-01T00:00:00Z', 2);
        assert.equal(a.out, b.out);
        assert.deepEqual(times(a.out), ['2025-01-01T12:00:00Z', '2026-01-01T12:00:00Z']);
        assert.deepEqual(times(utc(['@HOURLY'], '2025-01-01T00:00:00Z', 2).out), [
            '2025-01-01T01:00:00Z',
            '2025-01-01T02:00:00Z',
        ]);
        assert.deepEqual(times(utc(['@monthly'], '2025-01-31T23:59:59Z', 2).out), [
            '2025-02-01T00:00:00Z',
            '2025-03-01T00:00:00Z',
        ]);
    });

    test('--from is exclusive: a matching start time is not repeated', () => {
        assert.deepEqual(times(utc(['*/20 10 * * *'], '2025-01-01T10:00:30Z', 2).out), [
            '2025-01-01T10:20:00Z',
            '2025-01-01T10:40:00Z',
        ]);
        assert.deepEqual(times(utc(['0 10 * * *'], '2025-01-01T10:00:00Z', 1).out), ['2025-01-02T10:00:00Z']);
    });

    test('weekday 0 and 7 are both Sunday; "a/n" runs to the maximum', () => {
        const sun = times(utc(['0 0 * * 0'], '2025-01-01T00:00:00Z', 2).out);
        assert.deepEqual(sun, ['2025-01-05T00:00:00Z', '2025-01-12T00:00:00Z']);
        assert.deepEqual(times(utc(['0 0 * * 7'], '2025-01-01T00:00:00Z', 2).out), sun);
        assert.deepEqual(times(utc(['50/5 0 1 1 *'], '2025-01-01T00:00:00Z', 5).out), [
            '2025-01-01T00:50:00Z',
            '2025-01-01T00:55:00Z',
            '2026-01-01T00:50:00Z',
            '2026-01-01T00:55:00Z',
            '2027-01-01T00:50:00Z',
        ]);
    });

    test('day-of-month and day-of-week both restricted: either matches', () => {
        // 2025-06-13 is a Friday; Fridays and the 13th
        assert.deepEqual(times(utc(['0 0 13 * fri'], '2025-06-01T00:00:00Z', 4).out), [
            '2025-06-06T00:00:00Z',
            '2025-06-13T00:00:00Z',
            '2025-06-20T00:00:00Z',
            '2025-06-27T00:00:00Z',
        ]);
        // with `*` in one field the other one alone decides (AND with an always-true field)
        assert.deepEqual(times(utc(['0 0 13 * *'], '2025-06-01T00:00:00Z', 2).out), [
            '2025-06-13T00:00:00Z',
            '2025-07-13T00:00:00Z',
        ]);
    });

    test('leap day schedule crosses non-leap years, including the 2100 gap', () => {
        assert.deepEqual(times(utc(['0 0 29 2 *'], '2096-01-01T00:00:00Z', 3).out), [
            '2096-02-29T00:00:00Z',
            '2104-02-29T00:00:00Z',
            '2108-02-29T00:00:00Z',
        ]);
    });

    test('impossible date exits 1 with a message', () => {
        const r = utc(['0 0 31 2 *'], '2025-01-01T00:00:00Z');
        assert.equal(r.code, 1);
        assert.match(r.err, /Tidak ada waktu yang cocok dalam 8 tahun/);
        assert.deepEqual(times(r.out), []);
    });

    test('search starting inside the 2100 leap-day gap still finds the next one', () => {
        assert.deepEqual(times(utc(['0 0 29 2 *'], '2100-01-01T00:00:00Z', 1).out), ['2104-02-29T00:00:00Z']);
        assert.deepEqual(times(utc(['0 0 29 2 *'], '2096-03-01T00:00:00Z', 2).out), [
            '2104-02-29T00:00:00Z',
            '2108-02-29T00:00:00Z',
        ]);
    });

    test('invalid expressions are usage errors naming the field', () => {
        const cases: [string[], RegExp][] = [
            [['60 * * * *'], /Field menit tidak valid: "60"/],
            [['* 24 * * *'], /Field jam tidak valid: "24"/],
            [['* * 0 * *'], /Field tanggal tidak valid: "0"/],
            [['* * * 13 *'], /Field bulan tidak valid: "13"/],
            [['* * * * 8'], /Field hari tidak valid: "8"/],
            [['* * * foo *'], /Field bulan tidak valid.*foo/],
            [['5-1 * * * *'], /Field menit tidak valid.*terbalik/],
            [['*/0 * * * *'], /Field menit tidak valid.*step/],
            [['1,,2 * * * *'], /Field menit tidak valid.*kosong/],
            [['1-2-3 * * * *'], /Field menit tidak valid/],
            [['* * * * fri-sun'], /Field hari tidak valid.*terbalik/],
            [['* * * *'], /harus 5 field/],
            [['* * * * * *'], /harus 5 field/],
            [['@reboot'], /Makro tidak dikenal/],
        ];
        for (const [expr, message] of cases) {
            const r = cli(['cron', ...expr]);
            assert.equal(r.code, 2, expr.join(' '));
            assert.match(r.err, message, expr.join(' '));
        }
        assert.equal(cli(['cron']).code, 2);
    });

    test('invalid options are usage errors', () => {
        assert.equal(cli(['cron', '-n', '0', '* * * * *']).code, 2);
        assert.equal(cli(['cron', '-n', '51', '* * * * *']).code, 2);
        assert.match(cli(['cron', '--from', '2025-02-30', '* * * * *']).err, /--from/);
        assert.equal(cli(['cron', '--from', 'besok', '* * * * *']).code, 2);
        assert.equal(cli(['cron', '--from', '2025-01-01T24:00:00Z', '* * * * *']).code, 2);
    });

    test('--from with an explicit offset is honored; default count is 5', () => {
        const r = cli(['cron', '--utc', '--from', '2025-01-01T07:00:00+07:00', '0 * * * *']);
        assert.deepEqual(times(r.out), [
            '2025-01-01T01:00:00Z',
            '2025-01-01T02:00:00Z',
            '2025-01-01T03:00:00Z',
            '2025-01-01T04:00:00Z',
            '2025-01-01T05:00:00Z',
        ]);
    });

    test('local time zone: offset in output; DST gap skipped, overlap fires once', () => {
        const berlin = { TZ: 'Europe/Berlin' };
        const run = (from: string, expr: string) => cli(['cron', '--from', from, '-n', '3', expr], '', berlin);
        // spring forward 2025-03-30: 02:30 does not exist
        assert.deepEqual(times(run('2025-03-28T00:00', '30 2 * * *').out), [
            '2025-03-28T02:30:00+01:00',
            '2025-03-29T02:30:00+01:00',
            '2025-03-31T02:30:00+02:00',
        ]);
        // fall back 2025-10-26: 02:30 happens twice, fires once
        assert.deepEqual(times(run('2025-10-25T00:00', '30 2 * * *').out), [
            '2025-10-25T02:30:00+02:00',
            '2025-10-26T02:30:00+02:00',
            '2025-10-27T02:30:00+01:00',
        ]);
        // starting inside the repeated hour never goes backwards
        assert.deepEqual(times(run('2025-10-26T02:10:00+01:00', '30 2 * * *').out), [
            '2025-10-27T02:30:00+01:00',
            '2025-10-28T02:30:00+01:00',
            '2025-10-29T02:30:00+01:00',
        ]);
    });
});

describe('cron --json', () => {
    test('document shape', () => {
        const r = utc(['--json', '30 9 * * mon'], '2025-01-01T00:00:00Z', 2);
        assert.equal(r.code, 0);
        assert.deepEqual(JSON.parse(r.out), {
            expression: '30 9 * * mon',
            fields: {
                minute: [30],
                hour: [9],
                dayOfMonth: Array.from({ length: 31 }, (_, i) => i + 1),
                month: Array.from({ length: 12 }, (_, i) => i + 1),
                dayOfWeek: [1],
            },
            from: '2025-01-01T00:00:00Z',
            timezone: 'utc',
            next: ['2025-01-06T09:30:00Z', '2025-01-13T09:30:00Z'],
        });
    });

    test('exhausted search keeps exit 1 and still prints JSON', () => {
        const r = utc(['--json', '0 0 31 2 *'], '2025-01-01T00:00:00Z');
        assert.equal(r.code, 1);
        assert.deepEqual(JSON.parse(r.out).next, []);
    });

    test('--from accepts years 0000-0099 without mapping them to 19xx', () => {
        for (const [from, expected] of [
            ['0000-02-28', ['0000-02-29T00:00:00Z', '0000-03-01T00:00:00Z']],
            ['0050-01-01', ['0050-01-02T00:00:00Z', '0050-01-03T00:00:00Z']],
            ['0050-01-01T00:00:00Z', ['0050-01-02T00:00:00Z', '0050-01-03T00:00:00Z']],
            ['0050-01-01T07:00:00+07:00', ['0050-01-02T00:00:00Z', '0050-01-03T00:00:00Z']],
            ['0099-12-31', ['0100-01-01T00:00:00Z', '0100-01-02T00:00:00Z']],
        ] as const) {
            const r = utc(['@daily'], from, 2);
            assert.equal(r.code, 0, r.err);
            assert.deepEqual(times(r.out), [...expected]);
        }
        const monthly = utc(['@monthly'], '0050-01-01', 2);
        assert.equal(monthly.code, 0, monthly.err);
        assert.deepEqual(times(monthly.out), ['0050-02-01T00:00:00Z', '0050-03-01T00:00:00Z']);
        const march = utc(['0 0 1 3 *'], '0050-01-01', 1);
        assert.equal(march.code, 0, march.err);
        assert.deepEqual(times(march.out), ['0050-03-01T00:00:00Z']);
        const local = cli(['cron', '--from', '0050-01-01', '-n', '2', '@daily'], '', { TZ: 'UTC' });
        assert.equal(local.code, 0, local.err);
        assert.deepEqual(times(local.out), ['0050-01-02T00:00:00+00:00', '0050-01-03T00:00:00+00:00']);
        const json = utc(['--json', '@daily'], '0050-01-01', 1);
        assert.equal(json.code, 0, json.err);
        assert.equal(JSON.parse(json.out).from, '0050-01-01T00:00:00Z');
        assert.deepEqual(JSON.parse(json.out).next, ['0050-01-02T00:00:00Z']);
        assert.equal(utc(['@daily'], '0050-02-29', 1).code, 2);
    });
});
