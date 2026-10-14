import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHmac } from 'node:crypto';
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

const b64 = (value: unknown) => Buffer.from(JSON.stringify(value)).toString('base64url');

function sign(payload: unknown, key: string, header: Record<string, unknown> = { alg: 'HS256', typ: 'JWT' }): string {
    const alg = String(header.alg);
    const input = `${b64(header)}.${b64(payload)}`;
    const hash = { HS256: 'sha256', HS384: 'sha384', HS512: 'sha512' }[alg] ?? 'sha256';
    return `${input}.${createHmac(hash, key).update(input).digest('base64url')}`;
}

// jwt.io sample token, secret "your-256-bit-secret"
const SAMPLE =
    'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c';
const NOW = '2026-01-01T00:00:00Z';
const NOW_EPOCH = 1767225600;

describe('jwt decode', () => {
    test('prints header, payload and iat; says signature is not verified', () => {
        const r = cli(['jwt', '--now', NOW, SAMPLE]);
        assert.equal(r.code, 0);
        assert.match(r.out, /Header:\n\{\n {2}"alg": "HS256",\n {2}"typ": "JWT"\n\}/);
        assert.match(
            r.out,
            /Payload:\n\{\n {2}"sub": "1234567890",\n {2}"name": "John Doe",\n {2}"iat": 1516239022\n\}/,
        );
        assert.match(r.out, /iat: 2018-01-18T01:30:22\.000Z \(\d+ hari lalu\)/);
        assert.match(r.out, /TIDAK diverifikasi/);
    });

    test('accepts Bearer prefix, surrounding whitespace, stdin', () => {
        const a = cli(['jwt', '--now', NOW], `  Bearer ${SAMPLE}\n`);
        assert.equal(a.code, 0);
        assert.match(a.out, /John Doe/);
        const b = cli(['jwt', '--now', NOW, 'Bearer', SAMPLE]);
        assert.equal(b.out, a.out);
    });

    test('exp / nbf status relative to --now, epoch or ISO', () => {
        const expired = sign({ exp: NOW_EPOCH - 3 * 3600 - 10 }, 'k');
        const r = cli(['jwt', '--now', String(NOW_EPOCH), expired]);
        assert.equal(r.code, 0, 'expired alone does not fail without --key');
        assert.match(r.out, /exp: 2025-12-31T20:59:50\.000Z \(kedaluwarsa 3 jam lalu\)/);

        const valid = sign({ exp: NOW_EPOCH + 2 * 86400 + 5, nbf: NOW_EPOCH + 60 }, 'k');
        const v = cli(['jwt', '--now', NOW, valid]);
        assert.match(v.out, /exp: .*\(berlaku 2 hari lagi\)/);
        assert.match(v.out, /nbf: 2026-01-01T00:01:00\.000Z \(belum berlaku, 1 menit lagi\)/);

        const started = sign({ nbf: NOW_EPOCH - 1 }, 'k');
        assert.match(cli(['jwt', '--now', NOW, started]).out, /nbf: .*\(sudah berlaku\)/);
    });

    test('mistyped exp/nbf warn on stderr without interpreting them, but decode succeeds', () => {
        for (const json of [false, true]) {
            const r = cli(['jwt', '--now', NOW, ...(json ? ['--json'] : []), sign({ exp: '1', nbf: null }, 'k')]);
            assert.equal(r.code, 0);
            assert.match(r.err, /Peringatan: Klaim exp, nbf harus berupa angka/);
            if (json) {
                const decoded = JSON.parse(r.out);
                assert.deepEqual(decoded.payload, { exp: '1', nbf: null });
                assert.deepEqual(decoded.times, []);
                assert.equal(decoded.valid, null);
            } else {
                assert.doesNotMatch(r.out, /exp:|nbf:/);
                assert.match(r.out, /TIDAK diverifikasi/);
            }
        }
    });

    test('malformed tokens fail with exit 1', () => {
        const cases: [string, RegExp][] = [
            ['abc.def', /3 segmen/],
            ['a.b.c.d', /3 segmen/],
            [`${b64({ alg: 'HS256' })}.!!!.x`, /Payload bukan base64url/],
            [`${b64({ alg: 'HS256' })}.${b64({})}=.x`, /Payload bukan base64url/],
            [`${b64({ alg: 'HS256' })}.${b64({})}.a+b`, /Signature bukan base64url/],
            [`${Buffer.from('not json').toString('base64url')}.${b64({})}.`, /Header bukan JSON/],
            [`${b64([1])}.${b64({})}.`, /Header harus berupa objek/],
            [`${b64({})}.${b64('str')}.`, /Payload harus berupa objek/],
        ];
        for (const [token, message] of cases) {
            const r = cli(['jwt', token]);
            assert.equal(r.code, 1, token);
            assert.match(r.err, message);
            assert.equal(r.out, '');
        }
        assert.equal(cli(['jwt'], '  \n').code, 1);
    });

    test('invalid --now is a usage error', () => {
        const r = cli(['jwt', '--now', 'kemarin', SAMPLE]);
        assert.equal(r.code, 2);
        assert.match(r.err, /--now tidak valid/);
    });

    test('zone-less --now uses UTC with either a T or space, regardless of TZ', () => {
        const token = sign({ exp: NOW_EPOCH + 1, nbf: NOW_EPOCH }, 'k');
        const expected = cli(['jwt', '-k', 'k', '--json', '--now', NOW, token]);
        assert.equal(expected.code, 0);
        for (const now of ['2026-01-01', '2026-01-01T00:00:00', '2026-01-01 00:00:00']) {
            const actual = cli(['jwt', '-k', 'k', '--json', '--now', now, token], '', { TZ: 'Asia/Jakarta' });
            assert.deepEqual(actual, expected, now);
        }
    });

    test('--now rejects calendar rollover and invalid clock or zone values', () => {
        for (const now of [
            '2026-02-30',
            '2026-02-29 00:00:00',
            '2026-02-30T00:00:00Z',
            '2026-04-31T00:00:00+07:00',
            '2026-00-01',
            '2026-13-01',
            '2026-01-00',
            '2026-01-01T24:00:00',
            '2026-01-01T00:60:00',
            '2026-01-01T00:00:60',
            '2026-01-01T00:00:00+24:00',
        ]) {
            const r = cli(['jwt', '--now', now, SAMPLE]);
            assert.equal(r.code, 2, now);
            assert.match(r.err, /--now tidak valid/);
            assert.equal(r.out, '');
        }
    });

    test('--now accepts leap dates, fractional seconds, and explicit offsets', () => {
        for (const now of ['2024-02-29T00:00:00Z', '2024-02-29 00:00:00', '2024-02-29T07:00:00+07:00']) {
            const r = cli(['jwt', '-s', '-k', 'k', '--now', now, '--iat', '--json', '{}']);
            assert.equal(r.code, 0, now);
            assert.equal(JSON.parse(r.out).payload.iat, 1709164800);
        }
        const token = sign({ exp: NOW_EPOCH + 0.5 }, 'k');
        assert.equal(cli(['jwt', '-k', 'k', '--now', '2026-01-01 00:00:00.25', token]).code, 0);
        assert.equal(cli(['jwt', '-k', 'k', '--now', '2026-01-01T00:00:00.5Z', token]).code, 1);
    });
});

describe('jwt verify', () => {
    test('valid HMAC signature (jwt.io vector)', () => {
        const r = cli(['jwt', '-k', 'your-256-bit-secret', '--now', NOW, SAMPLE]);
        assert.equal(r.code, 0);
        assert.match(r.out, /Signature: valid/);
        assert.doesNotMatch(r.out, /TIDAK/);
    });

    test('HS384 and HS512', () => {
        for (const alg of ['HS384', 'HS512']) {
            const token = sign({ a: 1 }, 'rahasia', { alg });
            assert.match(cli(['jwt', '-k', 'rahasia', token]).out, /Signature: valid/);
            assert.equal(cli(['jwt', '-k', 'salah', token]).code, 1);
        }
    });

    test('wrong key, tampered payload, empty or truncated signature exit 1 after printing', () => {
        const r = cli(['jwt', '-k', 'salah', SAMPLE]);
        assert.equal(r.code, 1);
        assert.match(r.err, /Signature tidak valid/);
        assert.match(r.out, /John Doe/);

        const [h, , s] = SAMPLE.split('.');
        const tampered = `${h}.${b64({ sub: '1234567890', name: 'Admin', iat: 1516239022 })}.${s}`;
        assert.equal(cli(['jwt', '-k', 'your-256-bit-secret', tampered]).code, 1);
        assert.equal(cli(['jwt', '-k', 'your-256-bit-secret', `${SAMPLE.split('.').slice(0, 2).join('.')}.`]).code, 1);
        assert.equal(cli(['jwt', '-k', 'your-256-bit-secret', SAMPLE.slice(0, -4)]).code, 1);
    });

    test('alg none and non-HMAC algorithms are rejected with --key', () => {
        for (const alg of ['none', 'None', 'RS256', 'ES256']) {
            const token = `${b64({ alg })}.${b64({ sub: 'x' })}.`;
            const r = cli(['jwt', '-k', 'k', token]);
            assert.equal(r.code, 1, alg);
            assert.match(r.err, /Algoritma tidak didukung/);
        }
        const noAlg = `${b64({ typ: 'JWT' })}.${b64({})}.`;
        assert.equal(cli(['jwt', '-k', 'k', noAlg]).code, 1);
        // inherited property names must not count as algorithms
        assert.equal(cli(['jwt', '-k', 'k', `${b64({ alg: 'constructor' })}.${b64({})}.`]).code, 1);
    });

    test('expired or not-yet-valid token exits 1 with --key but prints everything', () => {
        const expired = sign({ exp: NOW_EPOCH - 10 }, 'k');
        const r = cli(['jwt', '-k', 'k', '--now', NOW, expired]);
        assert.equal(r.code, 1);
        assert.match(r.out, /Signature: valid/);
        assert.match(r.err, /kedaluwarsa/);

        const early = sign({ nbf: NOW_EPOCH + 10 }, 'k');
        const e = cli(['jwt', '-k', 'k', '--now', NOW, early]);
        assert.equal(e.code, 1);
        assert.match(e.err, /belum berlaku/);

        // exp exactly equal to now is already expired
        assert.equal(cli(['jwt', '-k', 'k', '--now', String(NOW_EPOCH), sign({ exp: NOW_EPOCH }, 'k')]).code, 1);
        assert.equal(cli(['jwt', '-k', 'k', '--now', NOW, sign({ exp: NOW_EPOCH + 1 }, 'k')]).code, 0);
    });

    test('present exp/nbf must be numbers with --key, including in JSON output', () => {
        for (const name of ['exp', 'nbf']) {
            for (const value of ['1', null, true, [], {}]) {
                const token = sign({ [name]: value }, 'k');
                for (const json of [false, true]) {
                    const r = cli(['jwt', '-k', 'k', '--now', NOW, ...(json ? ['--json'] : []), token]);
                    assert.equal(r.code, 1, `${name}=${JSON.stringify(value)}`);
                    assert.match(r.err, new RegExp(`Klaim ${name} harus berupa angka`));
                    if (json) {
                        const decoded = JSON.parse(r.out);
                        assert.equal(decoded.signature.verified, true);
                        assert.equal(decoded.valid, false);
                    } else {
                        assert.match(r.out, /Payload:/);
                        assert.match(r.out, /Signature: valid/);
                    }
                }
            }
        }
        assert.equal(cli(['jwt', '-k', 'k', sign({}, 'k')]).code, 0, 'absent claims remain optional');
    });

    test('infinite NumericDate claims compare numerically and have null ISO values', () => {
        const cases: [string, boolean, RegExp][] = [
            ['{"exp":-1e999}', false, /kedaluwarsa/],
            ['{"exp":1e999}', true, /tidak pernah kedaluwarsa/],
            ['{"nbf":1e999}', false, /belum berlaku/],
            ['{"nbf":-1e999}', true, /sudah berlaku/],
            ['{"exp":1e999,"nbf":-1e999,"iat":1e999}', true, /di masa depan/],
        ];
        for (const [payload, valid, status] of cases) {
            // JSON.stringify(Infinity) becomes null; preserve the JSON exponent on the signed wire.
            const input = `${b64({ alg: 'HS256' })}.${Buffer.from(payload).toString('base64url')}`;
            const token = `${input}.${createHmac('sha256', 'k').update(input).digest('base64url')}`;
            const r = cli(['jwt', '-k', 'k', '--now', NOW, '--json', token]);
            assert.equal(r.code, valid ? 0 : 1, payload);
            const decoded = JSON.parse(r.out);
            assert.equal(decoded.signature.verified, true);
            assert.equal(decoded.valid, valid);
            assert.ok(decoded.times.every((time: { iso: unknown }) => time.iso === null));
            assert.match(decoded.times.map((time: { status: string }) => time.status).join('\n'), status);
            if (!valid) assert.match(r.err, status);
            const text = cli(['jwt', '-k', 'k', '--now', NOW, token]);
            assert.equal(text.code, valid ? 0 : 1);
            assert.match(text.out, status);
            const unverified = cli(['jwt', '--now', NOW, '--json', token]);
            assert.equal(unverified.code, 0);
            assert.equal(JSON.parse(unverified.out).valid, null);
        }
    });

    test('invalid signature wins over expiry in the error message', () => {
        const r = cli(['jwt', '-k', 'salah', '--now', NOW, sign({ exp: 1 }, 'k')]);
        assert.match(r.err, /Signature tidak valid/);
    });

    test('empty --key is a usage error', () => {
        assert.equal(cli(['jwt', '-k', '', SAMPLE]).code, 2);
    });
});

describe('jwt --sign', () => {
    test('matches the jwt.io vector byte for byte', () => {
        const payload = '{"sub":"1234567890","name":"John Doe","iat":1516239022}';
        const r = cli(['jwt', '--sign', '-k', 'your-256-bit-secret', payload]);
        assert.equal(r.code, 0);
        assert.equal(r.out, `${SAMPLE}\n`);
    });

    test('round trip verifies with the same key only, for every alg', () => {
        for (const alg of ['HS256', 'HS384', 'HS512']) {
            const token = cli(['jwt', '-s', '-k', 'rahasia', '--alg', alg, '{"a":1}']).out.trim();
            assert.match(token, /^[\w-]+\.[\w-]+\.[\w-]+$/);
            const ok = cli(['jwt', '-k', 'rahasia', token]);
            assert.equal(ok.code, 0);
            assert.match(ok.out, new RegExp(`"alg": "${alg}"`));
            assert.match(ok.out, /Signature: valid/);
            assert.equal(cli(['jwt', '-k', 'lain', token]).code, 1);
        }
    });

    test('--exp duration and --iat are relative to --now; --exp overrides payload', () => {
        const r = cli([
            'jwt',
            '-s',
            '-k',
            'k',
            '--now',
            NOW,
            '--exp',
            '2w',
            '--iat',
            '--json',
            '{"exp":5,"iat":6,"x":"y"}',
        ]);
        const j = JSON.parse(r.out);
        assert.equal(j.payload.iat, NOW_EPOCH);
        assert.equal(j.payload.exp, NOW_EPOCH + 14 * 86400);
        assert.equal(j.payload.x, 'y');
        assert.deepEqual(j.header, { alg: 'HS256', typ: 'JWT' });
        assert.equal(cli(['jwt', '-k', 'k', '--now', NOW, j.token]).code, 0);
    });

    test('--exp units and absolute epoch', () => {
        const exp = (value: string) =>
            JSON.parse(cli(['jwt', '-s', '-k', 'k', '--now', NOW, '--exp', value, '--json', '{}']).out).payload.exp;
        assert.equal(exp('30s'), NOW_EPOCH + 30);
        assert.equal(exp('15m'), NOW_EPOCH + 900);
        assert.equal(exp('1h'), NOW_EPOCH + 3600);
        assert.equal(exp('7d'), NOW_EPOCH + 7 * 86400);
        assert.equal(exp('1767229200'), 1767229200);
    });

    test('reads payload from stdin; expired token fails verification', () => {
        const token = cli(['jwt', '-s', '-k', 'k', '--now', NOW, '--exp', '1m'], '{"sub":"1"}\n').out.trim();
        assert.equal(cli(['jwt', '-k', 'k', '--now', '2026-01-01T00:00:30Z', token]).code, 0);
        assert.equal(cli(['jwt', '-k', 'k', '--now', '2026-01-01T00:02:00Z', token]).code, 1);
    });

    test('usage errors exit 2', () => {
        for (const args of [
            ['-s', '{}'],
            ['-s', '-k', 'k', '[1]'],
            ['-s', '-k', 'k', '"str"'],
            ['-s', '-k', 'k', 'bukan json'],
            ['-s', '-k', 'k', '--alg', 'RS256', '{}'],
            ['-s', '-k', 'k', '--exp', '1x', '{}'],
            ['-s', '-k', 'k', '--exp', '-5m', '{}'],
            ['-s', '-k', 'k', '--exp', '1.5h', '{}'],
            ['-k', 'k', '--exp', '1h', SAMPLE],
        ]) {
            const r = cli(['jwt', ...args]);
            assert.equal(r.code, 2, args.join(' '));
            assert.equal(r.out, '');
        }
    });
});

describe('jwt --json', () => {
    test('decode without key', () => {
        const r = cli(['jwt', '--json', '--now', NOW, SAMPLE]);
        assert.equal(r.code, 0);
        const j = JSON.parse(r.out);
        assert.deepEqual(j.header, { alg: 'HS256', typ: 'JWT' });
        assert.equal(j.payload.name, 'John Doe');
        assert.equal(j.times.length, 1);
        assert.equal(j.times[0].claim, 'iat');
        assert.equal(j.times[0].iso, '2018-01-18T01:30:22.000Z');
        assert.equal(typeof j.times[0].status, 'string');
        assert.deepEqual(j.signature, { verified: null });
        assert.equal(j.valid, null);
    });

    test('verified token', () => {
        const j = JSON.parse(cli(['jwt', '--json', '-k', 'your-256-bit-secret', '--now', NOW, SAMPLE]).out);
        assert.deepEqual(j.signature, { verified: true, algorithm: 'HS256' });
        assert.equal(j.valid, true);
    });

    test('failures still print JSON and exit 1', () => {
        const bad = cli(['jwt', '--json', '-k', 'salah', SAMPLE]);
        assert.equal(bad.code, 1);
        assert.match(bad.err, /Signature tidak valid/);
        const j = JSON.parse(bad.out);
        assert.equal(j.signature.verified, false);
        assert.equal(j.valid, false);

        const expired = cli(['jwt', '--json', '-k', 'k', '--now', NOW, sign({ exp: NOW_EPOCH - 10 }, 'k')]);
        assert.equal(expired.code, 1);
        const e = JSON.parse(expired.out);
        assert.equal(e.signature.verified, true);
        assert.equal(e.valid, false);
        assert.equal(e.times[0].claim, 'exp');

        const none = cli(['jwt', '--json', '-k', 'k', `${b64({ alg: 'none' })}.${b64({})}.`]);
        assert.equal(none.code, 1);
        assert.equal(JSON.parse(none.out).valid, false);
    });

    test('malformed token prints no JSON', () => {
        const r = cli(['jwt', '--json', 'abc']);
        assert.equal(r.code, 1);
        assert.equal(r.out, '');
    });
});
