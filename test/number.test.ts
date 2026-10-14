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

describe('number', () => {
    test('prefix detection and all bases', () => {
        assert.equal(cli(['number', '255']).out, 'Desimal: 255\nHex:     0xff\nBiner:   0b11111111\nOktal:   0o377\n');
        assert.equal(cli(['number', '0xFF', '-t', '10']).out.trim(), '255');
        assert.equal(cli(['number', '0b1010', '-t', '16']).out.trim(), 'a');
        assert.equal(cli(['number', '0o17', '-t', '2']).out.trim(), '1111');
        assert.equal(cli(['number', '0xFF_FF', '-t', '10']).out.trim(), '65535');
    });

    test('--from forces base; matching prefix allowed', () => {
        assert.equal(cli(['number', '-f', '2', '1010', '-t', '10']).out.trim(), '10');
        assert.equal(cli(['number', '-f', '16', 'ff', '-t', '10']).out.trim(), '255');
        assert.equal(cli(['number', '-f', '16', '0xff', '-t', '10']).out.trim(), '255');
        // 0b1 is plain hex digits under --from 16
        assert.equal(cli(['number', '-f', '16', '0b1', '-t', '10']).out.trim(), '177');
        assert.equal(cli(['number', '-f', '10', '0b1']).code, 2);
        assert.equal(cli(['number', '-f', '2', '102']).code, 2);
        assert.equal(cli(['number', '-f', '8', '0x7']).code, 2);
    });

    test('arbitrary size', () => {
        const big = '123456789012345678901234567890';
        assert.equal(cli(['number', big, '-t', '16']).out.trim(), '18ee90ff6c373e0ee4e3f0ad2');
        assert.equal(cli(['number', '0x18ee90ff6c373e0ee4e3f0ad2', '-t', '10']).out.trim(), big);
    });

    test('--to prefix and grouping', () => {
        assert.equal(cli(['number', '255', '-t', '16', '--prefix']).out.trim(), '0xff');
        assert.equal(cli(['number', '255', '-t', '2', '--prefix', '-g']).out.trim(), '0b1111_1111');
        assert.equal(cli(['number', '5', '-t', '2', '-g']).out.trim(), '0101');
        assert.equal(cli(['number', '0xabcdef', '-t', '16', '-g']).out.trim(), 'ab_cd_ef');
        assert.equal(cli(['number', '0xabc', '-t', '16', '-g']).out.trim(), '0a_bc');
        assert.match(cli(['number', '5', '-g']).out, /Biner:   0b0101\n/);
    });

    test("negative values and two's complement", () => {
        assert.equal(cli(['number', '-255', '-t', '16', '--prefix']).out.trim(), '-0xff');
        assert.equal(cli(['number', '-0xff', '-t', '10']).out.trim(), '-255');
        assert.match(cli(['number', '-255']).out, /^Hex:     -0xff$/m);
        assert.equal(cli(['number', '-1', '-w', '16', '-t', '16']).out.trim(), 'ffff');
        assert.equal(cli(['number', '-2', '-w', '8', '-t', '2']).out.trim(), '11111110');
        assert.equal(cli(['number', '-128', '-w', '8', '-t', '10']).out.trim(), '128');
        assert.equal(cli(['number', '255', '-w', '8', '-t', '16']).out.trim(), 'ff');
        assert.equal(cli(['number', '1', '-w', '8', '-t', '2']).out.trim(), '00000001');
        const full = cli(['number', '-1', '-w', '8']).out;
        assert.match(
            full,
            /Komplemen dua \(8 bit\):\nDesimal: 255\nHex:     0xff\nBiner:   0b11111111\nOktal:   0o377/,
        );
        assert.equal(cli(['number', '-129', '-w', '8']).code, 2);
        assert.equal(cli(['number', '256', '-w', '8']).code, 2);
        assert.equal(cli(['number', '1', '-w', '12']).code, 2);
    });

    test('--json shape', () => {
        const doc = JSON.parse(cli(['number', '-0b101', '--json']).out);
        assert.deepEqual(doc, {
            input: '-0b101',
            base: 2,
            negative: true,
            decimal: '-5',
            hex: '-0x5',
            binary: '-0b101',
            octal: '-0o5',
        });
        const w = JSON.parse(cli(['number', '-1', '-w', '8', '--json']).out);
        assert.deepEqual(w.twosComplement, {
            width: 8,
            decimal: '255',
            hex: '0xff',
            binary: '0b11111111',
            octal: '0o377',
        });
        assert.equal(cli(['number', '1', '--json', '-t', '2']).code, 2);
    });

    test('stdin and invalid input', () => {
        assert.equal(cli(['number', '-t', '16'], '255\n').out.trim(), 'ff');
        assert.equal(cli(['number', 'xyz']).code, 2);
        assert.equal(cli(['number', '0x']).code, 2);
        assert.equal(cli(['number', '1__0']).code, 2);
        assert.equal(cli(['number', '_10']).code, 2);
        assert.equal(cli(['number', '1.5']).code, 2);
        assert.equal(cli(['number', '1', '2']).code, 2);
        assert.equal(cli(['number', '1', '--prefix']).code, 2);
        assert.equal(cli(['number', '1', '-t', '3']).code, 2);
    });

    test('negative-looking option value stays with its option', () => {
        for (const option of ['-w', '--width']) {
            const r = cli(['number', option, '-8', '5']);
            assert.equal(r.code, 2);
            assert.match(r.err, /Nilai --width harus bilangan bulat 1-64: -8/);
        }
        for (const [option, name] of [
            ['-f', 'from'],
            ['--from', 'from'],
            ['-t', 'to'],
            ['--to', 'to'],
        ] as const) {
            const r = cli(['number', option, '-2', '5']);
            assert.equal(r.code, 2);
            assert.match(r.err, new RegExp(`Nilai --${name} tidak valid: -2`));
        }
        const negative = cli(['number', '-5', '-t', '10']);
        assert.equal(negative.code, 0);
        assert.equal(negative.out.trim(), '-5');
    });
});
