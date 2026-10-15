import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { describe, test } from 'node:test';
import { fileURLToPath } from 'node:url';

const CLI = fileURLToPath(new URL('../src/cli.js', import.meta.url));

/** run the CLI without a terminal; `input` is piped to stdin */
const cli = (args: string[], input = '') => {
    const r = spawnSync(process.execPath, [CLI, ...args], { encoding: 'utf8', input, env: { PATH: process.env.PATH } });
    return { code: r.status, out: r.stdout, err: r.stderr };
};

const ascii = (args: string[]) => cli(['ascii', ...args]);

describe('ascii', () => {
    test('lookup by char, decimal, hex', () => {
        for (const v of ['A', '65', '0x41']) {
            assert.deepEqual(JSON.parse(ascii(['--json', v]).out), {
                decimal: 65,
                hex: '0x41',
                octal: '101',
                binary: '01000001',
                unicode: 'U+0041',
                utf8: ['41'],
                ascii: true,
                character: 'A',
                name: null,
                description: null,
            });
        }
        assert.match(ascii(['65']).out, /^desimal\s+65\nhex\s+0x41\noktal\s+101\nbiner\s+01000001\nkarakter\s+A\n$/);
    });

    test('control characters', () => {
        const lf = JSON.parse(ascii(['--json', '0x0a']).out);
        assert.equal(lf.name, 'LF');
        assert.equal(lf.character, null);
        assert.match(lf.description, /Line Feed/);
        assert.equal(JSON.parse(ascii(['--json', '0x00']).out).name, 'NUL');
        assert.equal(JSON.parse(ascii(['--json', '127']).out).name, 'DEL');
        assert.equal(JSON.parse(ascii(['--json', ' ']).out).name, 'SP');
        assert.match(ascii(['10']).out, /nama\s+LF/);
    });

    test('non-ASCII', () => {
        const e = JSON.parse(ascii(['--json', 'é']).out);
        assert.equal(e.decimal, 233);
        assert.equal(e.unicode, 'U+00E9');
        assert.deepEqual(e.utf8, ['C3', 'A9']);
        assert.equal(e.ascii, false);
        assert.deepEqual(JSON.parse(ascii(['--json', '😀']).out).utf8, ['F0', '9F', '98', '80']);
        assert.match(ascii(['0x20AC']).out, /unicode\s+U\+20AC\nutf-8\s+E2 82 AC\nkarakter\s+€/);
        assert.equal(JSON.parse(ascii(['--json', '0x85']).out).character, null);
    });

    test('table', () => {
        const r = ascii([]);
        assert.equal(r.code, 0);
        const rows = r.out.trimEnd().split('\n');
        assert.equal(rows.length, 34);
        assert.match(rows[2]!, /^ {2}0 00 000 NUL \| {2}32 20 040 SP {2}\| {2}64 40 100 @ {3}\| {2}96 60 140 `$/);
        const table = JSON.parse(ascii(['--json']).out).table;
        assert.equal(table.length, 128);
        assert.equal(table[127].name, 'DEL');
        assert.equal(ascii(['--table']).out, r.out);
    });

    test('errors', () => {
        for (const args of [
            ['abc'],
            ['0x110000'],
            ['0xD800'],
            ['1', '2'],
            ['--table', 'A'],
            ['99999999999999999999'],
        ]) {
            assert.notEqual(ascii(args).code, 0, args.join(' '));
        }
        assert.equal(ascii(['1', '2']).code, 2);
        assert.equal(ascii(['--table', 'A']).code, 2);
        assert.equal(ascii(['abc']).code, 1);
    });
});
