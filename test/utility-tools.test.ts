import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createCipheriv, createDecipheriv, scryptSync } from 'node:crypto';
import { describe, test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { createV1Generator, createV7Generator } from '../src/tools/uuid/version.js';

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

/** like `cli` but with raw bytes in and out, for binary payloads */
const cliBytes = (args: string[], input: Buffer) => {
    const r = spawnSync(process.execPath, [CLI, ...args], { input, env: { PATH: process.env.PATH } });
    return { code: r.status, out: r.stdout, err: r.stderr.toString('utf8') };
};

describe('hash', () => {
    test('known SHA-256 and HMAC vectors, stdin used verbatim', () => {
        assert.equal(
            cli(['hash', 'abc']).out.trim(),
            'ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad',
        );
        assert.equal(cli(['hash', '-a', 'md5'], 'abc').out.trim(), '900150983cd24fb0d6963f7d28e17f72');
        // RFC 4231 case 2
        assert.equal(
            cli(['hash', '-k', 'Jefe', 'what do ya want for nothing?']).out.trim(),
            '5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843',
        );
        // stdin keeps its trailing newline: not the hash of "abc"
        assert.notEqual(cli(['hash'], 'abc\n').out.trim(), cli(['hash', 'abc']).out.trim());
    });

    test('crc32 vector, encodings, no HMAC', () => {
        assert.equal(cli(['hash', '-a', 'crc32', '123456789']).out, 'cbf43926\n');
        assert.equal(cli(['hash', '-a', 'crc32', '']).out, '00000000\n');
        assert.equal(cli(['hash', '-a', 'crc32', '-e', 'base64', '123456789']).out, 'y/Q5Jg==\n');
        assert.equal(cli(['hash', '-a', 'crc32', '-e', 'base64url', '123456789']).out, 'y_Q5Jg==\n'.replace('==', ''));
        const r = cli(['hash', '-a', 'crc32', '-k', 'x', 'abc']);
        assert.equal(r.code, 2);
        assert.equal(r.out, '');
    });

    test('crc32 with --key is rejected before any input is read', () => {
        // A missing file would exit 1 if the input were read first.
        const r = cli(['hash', '-a', 'crc32', '-k', 'x', '-f', '/nonexistent/dt-input']);
        assert.equal(r.code, 2);
        assert.equal(r.out, '');
    });

    test('--json shape', () => {
        assert.deepEqual(JSON.parse(cli(['hash', '--json', '-a', 'crc32', '123456789']).out), {
            algorithm: 'crc32',
            encoding: 'hex',
            hmac: false,
            digest: 'cbf43926',
        });
        const h = JSON.parse(cli(['hash', '--json', '-k', 'Jefe', 'what do ya want for nothing?']).out);
        assert.equal(h.hmac, true);
        assert.equal(h.algorithm, 'sha256');
        assert.equal(h.digest, '5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843');
    });

    test('unknown algorithm is a usage error', () => {
        const r = cli(['hash', '-a', 'rot13', 'x']);
        assert.equal(r.code, 2);
        assert.match(r.err, /--algorithm tidak valid: rot13/);
    });
});

describe('encode / decode', () => {
    test('round trip for every type, including non-ASCII', () => {
        const text = 'halo dunia & café ✓?';
        for (const type of ['base64', 'base64url', 'hex', 'url']) {
            const enc = cli(['encode', '-t', type, text]);
            assert.equal(enc.code, 0, enc.err);
            const dec = cli(['decode', '-t', type], enc.out);
            assert.equal(dec.out, text, type);
        }
        assert.equal(cli(['encode', 'halo']).out, 'aGFsbw==\n');
        assert.equal(cli(['encode', '-t', 'base64url', '??>>']).out, 'Pz8-Pg\n');
    });

    test('--json shape', () => {
        assert.deepEqual(JSON.parse(cli(['encode', '--json', 'halo']).out), { type: 'base64', output: 'aGFsbw==' });
        assert.deepEqual(JSON.parse(cli(['encode', '--json', '-t', 'hex', 'hi']).out), { type: 'hex', output: '6869' });
    });

    test('malformed input fails with exit 1 instead of decoding garbage', () => {
        assert.equal(cli(['decode', '-t', 'hex', 'abc']).code, 1);
        assert.equal(cli(['decode', '-t', 'base64', 'a$b=']).code, 1);
        assert.equal(cli(['decode', '-t', 'url', '%E0%A4%A']).code, 1);
    });

    test('url encoding works on raw bytes, so binary round-trips', () => {
        const enc = cliBytes(['encode', '-t', 'url'], Buffer.from([0xff, 0xfe]));
        assert.equal(enc.out.toString(), '%FF%FE\n');
        const all = Buffer.from(Array.from({ length: 256 }, (_, i) => i));
        const round = cliBytes(['encode', '-t', 'url'], all);
        assert.equal(round.code, 0, round.err);
        const dec = cliBytes(['decode', '-t', 'url'], round.out);
        assert.deepEqual(dec.out, all);
        assert.deepEqual(cliBytes(['decode', '-t', 'url'], Buffer.from('%ff%FE')).out, Buffer.from([0xff, 0xfe]));
        // unreserved characters stay literal; '+' is not a space
        assert.equal(cli(['encode', '-t', 'url', "a-_.!~*'()Z9 +"]).out, "a-_.!~*'()Z9%20%2B\n");
        assert.equal(cli(['decode', '-t', 'url', 'a+b']).out, 'a+b');
    });

    test('url decode requires two hex digits after every %', () => {
        for (const bad of ['%', '%4', '%zz', 'a%4g', '100%'])
            assert.equal(cli(['decode', '-t', 'url', bad]).code, 1, bad);
    });

    test('base64 padding and trailing bits are strict', () => {
        for (const type of ['base64', 'base64url']) {
            assert.equal(cli(['decode', '-t', type, 'YQ==']).out, 'a', type);
            assert.equal(cli(['decode', '-t', type, 'YQ']).out, 'a', type);
            assert.equal(cli(['decode', '-t', type, 'YWI=']).out, 'ab', type);
            assert.equal(cli(['decode', '-t', type, 'YWI']).out, 'ab', type);
            for (const bad of ['YQ=', 'YWJj=', 'YWJj==', 'YQ===', 'Y', '=', 'YR==', 'YR', 'YWJ=='])
                assert.equal(cli(['decode', '-t', type, bad]).code, 1, `${type} ${bad}`);
        }
        assert.equal(cli(['decode', '-t', 'base64url', 'Pz8-Pg==']).out, '??>>');
        assert.equal(cli(['decode', '-t', 'base64url', 'Pz8-Pg']).out, '??>>');
    });

    test('whitespace inside base64 and hex input is ignored', () => {
        const data = Buffer.alloc(100, 7).toString('hex');
        const wrapped = Buffer.from(data, 'hex')
            .toString('base64')
            .replace(/(.{76})/g, '$1\r\n');
        assert.match(wrapped, /\r\n/);
        assert.equal(cli(['decode', '-t', 'base64'], wrapped).out, Buffer.from(data, 'hex').toString());
        assert.equal(cli(['decode', '-t', 'base64'], ' YW Jj\n ZA = =\n').out, 'abcd');
        assert.equal(cli(['decode', '-t', 'base64url'], ' Pz 8-\n Pg = =\n').out, '??>>');
        assert.equal(cli(['decode', '-t', 'hex'], '68 61\n6c 6f\n').out, 'halo');
        assert.equal(cli(['decode', '-t', 'hex'], '686 1').code, 0);
        assert.equal(cli(['decode', '-t', 'hex'], '6 8 6').code, 1);
    });
});

describe('encrypt', () => {
    test('round trip, random per run, wrong password and tampering rejected', () => {
        const a = cli(['encrypt', '-p', 'rahasia', 'halo']);
        assert.equal(a.code, 0, a.err);
        assert.notEqual(a.out, cli(['encrypt', '-p', 'rahasia', 'halo']).out);
        assert.equal(cli(['encrypt', '-d', '-p', 'rahasia'], a.out).out, 'halo');

        const wrong = cli(['encrypt', '-d', '-p', 'salah'], a.out);
        assert.equal(wrong.code, 1);
        assert.match(wrong.err, /password salah atau data rusak/);

        // Flip one byte of the decoded token (version | salt 16 | iv 12 | tag 16 | ciphertext), then re-encode.
        const raw = Buffer.from(a.out.trim(), 'base64url');
        const decryptWith = (bytes: Buffer) => cli(['encrypt', '-d', '-p', 'rahasia'], bytes.toString('base64url'));
        for (const [name, offset] of [
            ['salt', 1],
            ['iv', 17],
            ['tag', 29],
            ['ciphertext', raw.length - 1],
        ] as const) {
            const tampered = Buffer.from(raw);
            tampered[offset] = tampered[offset]! ^ 0x01;
            const r = decryptWith(tampered);
            assert.equal(r.code, 1, name);
            assert.match(r.err, /password salah atau data rusak/, name);
        }
    });

    test('truncated token and unknown version byte are rejected', () => {
        const raw = Buffer.from(cli(['encrypt', '-p', 'rahasia', 'halo']).out.trim(), 'base64url');
        const decryptWith = (bytes: Buffer) => cli(['encrypt', '-d', '-p', 'rahasia'], bytes.toString('base64url'));
        assert.equal(decryptWith(raw.subarray(0, 40)).code, 1);
        assert.equal(decryptWith(raw.subarray(0, 1)).code, 1);
        assert.equal(decryptWith(Buffer.alloc(0)).code, 1);
        // dropping the last ciphertext byte breaks the auth tag
        assert.equal(decryptWith(raw.subarray(0, -1)).code, 1);
        for (const version of [0, 2, 255]) {
            const wrong = Buffer.from(raw);
            wrong[0] = version;
            assert.equal(decryptWith(wrong).code, 1, `version ${version}`);
        }
    });

    test('token format: scrypt N=16384 r=8 p=1, AES-256-GCM with the version byte as AAD', () => {
        const salt = Buffer.alloc(16, 1);
        const iv = Buffer.alloc(12, 2);
        const seal = (aad: Buffer) => {
            const key = scryptSync('rahasia', salt, 32, { N: 16384, r: 8, p: 1 });
            const cipher = createCipheriv('aes-256-gcm', key, iv);
            cipher.setAAD(aad);
            const body = Buffer.concat([cipher.update('halo'), cipher.final()]);
            return Buffer.concat([Buffer.from([1]), salt, iv, cipher.getAuthTag(), body]).toString('base64url');
        };
        assert.equal(cli(['encrypt', '-d', '-p', 'rahasia'], seal(Buffer.from([1]))).out, 'halo');
        assert.equal(cli(['encrypt', '-d', '-p', 'rahasia'], seal(Buffer.alloc(0))).code, 1);
        const emitted = Buffer.from(cli(['encrypt', '-p', 'rahasia', 'halo']).out.trim(), 'base64url');
        const key = scryptSync('rahasia', emitted.subarray(1, 17), 32, { N: 16384, r: 8, p: 1 });
        const decipher = createDecipheriv('aes-256-gcm', key, emitted.subarray(17, 29));
        decipher.setAAD(emitted.subarray(0, 1));
        decipher.setAuthTag(emitted.subarray(29, 45));
        assert.equal(Buffer.concat([decipher.update(emitted.subarray(45)), decipher.final()]).toString(), 'halo');
    });

    test('explicit empty --password is a usage error even when the env has one', () => {
        const r = cli(['encrypt', '-p', '', 'halo'], '', { DEV_TOOL_PASSWORD: 'pw' });
        assert.equal(r.code, 2);
        assert.equal(r.out, '');
        assert.equal(cli(['encrypt', '-d', '-p', ''], 'x', { DEV_TOOL_PASSWORD: 'pw' }).code, 2);
    });

    test('password from env; missing password without a terminal is a usage error', () => {
        const a = cli(['encrypt', 'halo'], '', { DEV_TOOL_PASSWORD: 'pw' });
        assert.equal(cli(['encrypt', '-d'], a.out, { DEV_TOOL_PASSWORD: 'pw' }).out, 'halo');
        const missing = cli(['encrypt', 'halo']);
        assert.equal(missing.code, 2);
        assert.match(missing.err, /Password diperlukan/);
    });
});

describe('uuid', () => {
    test('UUID v4 by default, count honored', () => {
        const lines = cli(['uuid', '-c', '3']).out.trim().split('\n');
        assert.equal(lines.length, 3);
        for (const line of lines)
            assert.match(line, /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/);
        assert.equal(new Set(lines).size, 3);
    });

    test('token size by bytes, bits, or exact characters in every format', () => {
        const tok = (...a: string[]) => cli(['uuid', '-t', ...a]).out.trim();
        assert.match(tok(), /^[0-9a-f]{64}$/);
        assert.equal(tok('-b', '24', '-f', 'base64url').length, 32);
        assert.match(tok('--bits', '128'), /^[0-9a-f]{32}$/);
        assert.match(tok('-l', '7'), /^[0-9a-f]{7}$/);
        assert.match(tok('-l', '50', '-f', 'base64'), /^[A-Za-z0-9+/]{50}$/);
        assert.match(tok('-l', '50', '-f', 'base64url'), /^[A-Za-z0-9_-]{50}$/);
        assert.match(tok('-f', 'alnum'), /^[A-Za-z0-9]{32}$/);
        assert.match(tok('-l', '5', '-f', 'alnum'), /^[A-Za-z0-9]{5}$/);
        assert.notEqual(tok('-l', '40', '-f', 'alnum'), tok('-l', '40', '-f', 'alnum'));
    });

    const bitsOf = (u: string) => BigInt('0x' + u.replaceAll('-', ''));

    test('v7 layout and strict ordering within one invocation', () => {
        const before = Date.now();
        const lines = cli(['uuid', '-v', '7', '-c', '1000']).out.trim().split('\n');
        const after = Date.now();
        assert.equal(lines.length, 1000);
        for (const line of lines)
            assert.match(line, /^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/);
        // lexicographic order of the canonical form is chronological order
        for (let i = 1; i < lines.length; i++)
            assert.ok((lines[i - 1] ?? '') < (lines[i] ?? ''), `${lines[i - 1]} < ${lines[i]}`);
        const ts = Number(bitsOf(lines[0] ?? '') >> 80n);
        assert.ok(ts >= before - 1000 && ts <= after + 1000, String(ts)); // process start is outside our window
        assert.equal(new Set(lines).size, 1000);
    });

    test('v7 stays strictly increasing past counter overflow with a frozen clock', () => {
        const frozen = 1700000000000;
        const generate = createV7Generator(() => frozen);
        let previous = generate();
        assert.equal(Number(bitsOf(previous) >> 80n), frozen);
        let overflows = 0;
        for (let i = 1; i < 10000; i++) {
            const current = generate();
            assert.ok(current > previous, `${previous} < ${current}`);
            const previousMs = Number(bitsOf(previous) >> 80n);
            const currentMs = Number(bitsOf(current) >> 80n);
            if (currentMs !== previousMs) {
                assert.equal(currentMs, previousMs + 1);
                assert.equal(Number((bitsOf(previous) >> 64n) & 0xfffn), 4095);
                overflows++;
            }
            previous = current;
        }
        assert.ok(overflows >= 2);
        assert.ok(Number(bitsOf(previous) >> 80n) > frozen);
    });

    test('v7 stays strictly increasing with a decreasing clock', () => {
        let clock = 1700000000000;
        const generate = createV7Generator(() => clock--);
        let previous = generate();
        for (let i = 1; i < 10000; i++) {
            const current = generate();
            assert.ok(current > previous, `${previous} < ${current}`);
            assert.ok(bitsOf(current) >> 80n >= bitsOf(previous) >> 80n);
            previous = current;
        }
    });

    test('v1 advances its timestamp with frozen and decreasing clocks', () => {
        for (const decrement of [0, 1]) {
            let clock = 1700000000000;
            const generate = createV1Generator(() => {
                const value = clock;
                clock -= decrement;
                return value;
            });
            let previousTicks = -1n;
            let suffix = '';
            for (let i = 0; i < 10000; i++) {
                const current = generate();
                const [lo = '', mid = '', hi = ''] = current.split('-');
                const ticks = BigInt('0x' + hi.slice(1) + mid + lo);
                if (i === 0) {
                    assert.equal(ticks, (1700000000000n + 12219292800000n) * 10000n);
                    suffix = current.slice(19);
                } else {
                    assert.equal(ticks, previousTicks + 1n);
                    assert.equal(current.slice(19), suffix);
                }
                previousTicks = ticks;
            }
        }
    });

    test('v1 layout, multicast random node, uniqueness', () => {
        const lines = cli(['uuid', '-v', '1', '-c', '500']).out.trim().split('\n');
        assert.equal(lines.length, 500);
        assert.equal(new Set(lines).size, 500);
        for (const line of lines)
            assert.match(line, /^[0-9a-f]{8}-[0-9a-f]{4}-1[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/);
        // same node and clock sequence throughout, multicast bit (lowest bit of first node octet) set
        assert.equal(new Set(lines.map((l) => l.slice(19))).size, 1);
        assert.equal(parseInt((lines[0] ?? '').slice(24, 26), 16) & 1, 1);
        // timestamp: 100ns ticks since 1582-10-15, strictly increasing
        const ticks = (u: string) => {
            const [lo = '', mid = '', hi = ''] = u.split('-');
            return BigInt('0x' + hi.slice(1) + mid + lo);
        };
        for (let i = 1; i < lines.length; i++) assert.ok(ticks(lines[i] ?? '') > ticks(lines[i - 1] ?? ''));
        const ms = Number(ticks(lines[0] ?? '') / 10000n - 12219292800000n);
        assert.ok(Math.abs(ms - Date.now()) < 60_000, String(ms));
    });

    test('--json shape', () => {
        const u = JSON.parse(cli(['uuid', '--json', '-v', '7', '-c', '2']).out);
        assert.equal(u.type, 'uuid');
        assert.equal(u.version, 7);
        assert.equal(u.values.length, 2);
        assert.equal(JSON.parse(cli(['uuid', '--json']).out).version, 4);
        const t = JSON.parse(cli(['uuid', '--json', '-t', '-b', '4', '-f', 'hex']).out);
        assert.equal(t.type, 'token');
        assert.equal(t.format, 'hex');
        assert.match(t.values[0], /^[0-9a-f]{8}$/);
        assert.equal('version' in t, false);
    });

    test('invalid size options are usage errors', () => {
        for (const args of [
            ['-b', '8'], // size without --token
            ['-t', '-b', '0'],
            ['-t', '--bits', '12'],
            ['-t', '-l', '5', '-b', '5'],
            ['-t', '-f', 'alnum', '-b', '5'],
            ['-t', '-f', 'nope'],
            ['-c', '1001'],
            ['-v', '5'],
            ['-t', '-v', '4'],
        ]) {
            assert.equal(cli(['uuid', ...args]).code, 2, args.join(' '));
        }
    });
});
