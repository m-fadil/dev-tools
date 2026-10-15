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

describe('bytes', () => {
    test('decimal and binary units differ, case-insensitive', () => {
        assert.equal(cli(['bytes', '1kb', '-t', 'B']).out.trim(), '1000');
        assert.equal(cli(['bytes', '1KiB', '-t', 'b']).out.trim(), '1024');
        assert.equal(cli(['bytes', '1kib', '-t', 'B']).out.trim(), '1024');
        assert.equal(cli(['bytes', '1536']).out.split('\n')[0], 'Byte: 1536');
        assert.equal(cli(['bytes', '2', 'GiB', '-t', 'MiB']).out.trim(), '2048');
        assert.equal(cli(['bytes', '2 GiB', '-t', 'MB']).out.trim(), '2147.48');
    });

    test('exact arithmetic without float drift', () => {
        const r = cli(['bytes', '0.1 KB', '--json']);
        const doc = JSON.parse(r.out);
        assert.equal(doc.bytes, '100');
        assert.equal(doc.rounded, false);
        assert.equal(cli(['bytes', '9007199254740993', '-t', 'B']).out.trim(), '9007199254740993');
        assert.equal(cli(['bytes', '1.5GB', '-t', 'B']).out.trim(), '1500000000');
    });

    test('non-integer bytes are rounded half up and flagged', () => {
        const r = cli(['bytes', '1.5', '--json']);
        assert.equal(r.code, 0);
        const doc = JSON.parse(r.out);
        assert.equal(doc.bytes, '2');
        assert.equal(doc.rounded, true);
        assert.match(r.err, /dibulatkan/);
        assert.equal(JSON.parse(cli(['bytes', '1.4', '--json']).out).bytes, '1');
        assert.match(cli(['bytes', '0.5']).out, /Byte: 1 \(dibulatkan\)/);
    });

    test('precision and readable line', () => {
        assert.match(cli(['bytes', '1.5GB']).out, /^Terbaca: 1\.5 GB \| 1\.4 GiB$/m);
        assert.equal(cli(['bytes', '1536', '-t', 'KiB', '-p', '0']).out.trim(), '2');
        assert.equal(cli(['bytes', '1536', '-t', 'KiB', '-p', '10']).out.trim(), '1.5');
        assert.equal(cli(['bytes', '1', '-t', 'PB', '-p', '10']).out.trim(), '0');
        assert.equal(cli(['bytes', '1', '-t', 'KB', '-p', '3']).out.trim(), '0.001');
        assert.match(cli(['bytes', '512']).out, /^Terbaca: 512 B \| 512 B$/m);
        assert.match(cli(['bytes', '5000PB']).out, /^Terbaca: 5000 PB \| /m);
    });

    test('--json shape', () => {
        const doc = JSON.parse(cli(['bytes', '2GiB', '--json']).out);
        assert.deepEqual(Object.keys(doc), ['input', 'bytes', 'rounded', 'precision', 'decimal', 'binary', 'readable']);
        assert.deepEqual(Object.keys(doc.decimal), ['KB', 'MB', 'GB', 'TB', 'PB']);
        assert.deepEqual(Object.keys(doc.binary), ['KiB', 'MiB', 'GiB', 'TiB', 'PiB']);
        assert.equal(doc.binary.GiB, '2');
        assert.deepEqual(doc.readable, { decimal: '2.15 GB', binary: '2 GiB' });
        const to = JSON.parse(cli(['bytes', '2GiB', '--to', 'GiB', '--json']).out);
        assert.deepEqual(to, { input: '2GiB', bytes: '2147483648', rounded: false, unit: 'GiB', value: '2' });
    });

    test('invalid input exits 2', () => {
        assert.equal(cli(['bytes', '-5']).code, 2);
        assert.match(cli(['bytes', '-5']).err, /negatif/);
        assert.equal(cli(['bytes', '1XB']).code, 2);
        assert.equal(cli(['bytes', 'abc']).code, 2);
        assert.equal(cli(['bytes']).code, 2);
        assert.equal(cli(['bytes', '1', '-t', 'zb']).code, 2);
        assert.equal(cli(['bytes', '1', '-p', '11']).code, 2);
    });

    test('readable promotes the unit when rounding reaches the base', () => {
        const decimal = cli(['bytes', '999999']);
        assert.equal(decimal.code, 0);
        assert.match(decimal.out, /^Terbaca: 1 MB \| 976\.56 KiB$/m);
        const binary = cli(['bytes', '1048575']);
        assert.equal(binary.code, 0);
        assert.match(binary.out, /^Terbaca: 1\.05 MB \| 1 MiB$/m);
        assert.equal(JSON.parse(cli(['bytes', '999999', '--json']).out).readable.decimal, '1 MB');
        assert.equal(JSON.parse(cli(['bytes', '1048575', '--json']).out).readable.binary, '1 MiB');
        assert.equal(cli(['bytes', '999999', '-t', 'KB']).out.trim(), '1000');
        assert.equal(cli(['bytes', '1048575', '-t', 'KiB']).out.trim(), '1024');
    });

    test('readable promotion respects precision and the largest unit', () => {
        assert.equal(JSON.parse(cli(['bytes', '999500', '-p', '0', '--json']).out).readable.decimal, '1 MB');
        assert.equal(JSON.parse(cli(['bytes', '1048064', '-p', '0', '--json']).out).readable.binary, '1 MiB');
        assert.equal(JSON.parse(cli(['bytes', '999999', '-p', '3', '--json']).out).readable.decimal, '999.999 KB');
        assert.equal(JSON.parse(cli(['bytes', '1048575', '-p', '3', '--json']).out).readable.binary, '1023.999 KiB');
        assert.equal(JSON.parse(cli(['bytes', '999999999999999999', '--json']).out).readable.decimal, '1000 PB');
        const largestBinary = (1024n ** 6n - 1n).toString();
        assert.equal(JSON.parse(cli(['bytes', largestBinary, '--json']).out).readable.binary, '1024 PiB');
    });

    test('negative-looking option value is not taken as the size', () => {
        for (const option of ['-p', '--precision']) {
            const r = cli(['bytes', '1', option, '-1']);
            assert.equal(r.code, 2);
            assert.match(r.err, /Nilai --precision harus bilangan bulat 0-10: -1/);
            assert.doesNotMatch(r.err, /Ukuran tidak boleh negatif/);
        }
        for (const option of ['-t', '--to']) {
            const r = cli(['bytes', '1', option, '-1']);
            assert.equal(r.code, 2);
            assert.match(r.err, /Nilai --to tidak valid: -1/);
        }
        const negative = cli(['bytes', '-5']);
        assert.equal(negative.code, 2);
        assert.match(negative.err, /Ukuran tidak boleh negatif: -5/);
    });
});
