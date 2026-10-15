import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { after, describe, test } from 'node:test';
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

const dir = mkdtempSync(join(tmpdir(), 'env-test-'));
after(() => rmSync(dir, { recursive: true, force: true }));

let counter = 0;
const file = (content: string) => {
    const path = join(dir, `f${counter++}.env`);
    writeFileSync(path, content);
    return path;
};

describe('env', () => {
    test('identical files exit 0', () => {
        const a = file('A=1\nB=2\n');
        const r = cli(['env', a, file('B=2\nA=1')]);
        assert.equal(r.code, 0);
        assert.equal(r.out.trim(), '2 sama, 0 beda, 0 hanya a, 0 hanya b');
    });

    test('reports sorted one-sided keys and masks values by default', () => {
        const a = file('Z=1\nA=1\nSHARED=rahasia-a\nSAME=x\n');
        const b = file('Y=1\nB=1\nSHARED=rahasia-b\nSAME=x\n');
        const r = cli(['env', a, b]);
        assert.equal(r.code, 1);
        assert.equal(
            r.out,
            'Hanya di a:\n  A\n  Z\nHanya di b:\n  B\n  Y\nBeda nilai:\n  SHARED\n1 sama, 1 beda, 2 hanya a, 2 hanya b\n',
        );
        assert.doesNotMatch(r.out + r.err, /rahasia/);
    });

    test('--show-values prints both values', () => {
        const r = cli(['env', '--show-values', file('K=satu'), file('K=dua')]);
        assert.equal(r.code, 1);
        assert.match(r.out, /K: satu -> dua/);
    });

    test('--keys-only ignores value differences', () => {
        const r = cli(['env', '-k', file('K=1\nL=1'), file('K=2\nL=1')]);
        assert.equal(r.code, 0);
        assert.equal(r.out.trim(), '2 sama, 0 beda, 0 hanya a, 0 hanya b');
        assert.equal(cli(['env', '-k', file('K=1'), file('M=1')]).code, 1);
    });

    test('parser: export, comments, quotes, escapes, no interpolation', () => {
        const a = file(
            [
                '# komentar',
                '',
                'export A=1',
                'B="x # y"',
                "C='lit\\n#z'",
                'D=plain # trailing',
                'E="baris1\\nbaris2 \\"q\\""',
                'F=${A}',
                'G=#fff',
                'H=',
            ].join('\n'),
        );
        const b = file(
            [
                'A=1',
                'B=x # y',
                'C=lit\\n#z',
                'D=plain',
                'E="baris1\\nbaris2 \\"q\\"" # c',
                'F=${A}',
                'G=#fff',
                'H=',
            ].join('\n'),
        );
        const r = cli(['env', '-s', a, b]);
        // B differs only in quoting: unquoted `x # y` is `x`; everything else parses equal
        assert.equal(r.out, 'Beda nilai:\n  B: x # y -> x\n7 sama, 1 beda, 0 hanya a, 0 hanya b\n');
    });

    test('duplicate keys: last wins, warning on stderr', () => {
        const r = cli(['env', '-s', file('K=1\nK=2\n'), file('K=2\n')]);
        assert.equal(r.code, 0);
        assert.match(r.err, /^a:2: .*K duplikat \(baris 1\)/);
        assert.equal(r.out.trim(), '1 sama, 0 beda, 0 hanya a, 0 hanya b');
    });

    test('invalid lines are reported with line numbers and cause exit 1', () => {
        const r = cli(['env', file('A=1\nbukan valid\nB="belum ditutup\n'), file('A=1\n')]);
        assert.equal(r.code, 1);
        assert.match(r.err, /a:2: baris tidak valid/);
        assert.match(r.err, /a:3: baris tidak valid/);
        assert.match(r.err, /2 baris tidak valid/);
    });

    test('CRLF files parse like LF files', () => {
        assert.equal(cli(['env', file('A=1\r\nB=2\r\n'), file('A=1\nB=2\n')]).code, 0);
    });

    test('wrong positional count is a usage error', () => {
        assert.equal(cli(['env', file('A=1')]).code, 2);
        assert.equal(cli(['env']).code, 2);
    });

    test('missing file fails with exit 1', () => {
        const r = cli(['env', join(dir, 'tidak-ada.env'), file('A=1')]);
        assert.equal(r.code, 1);
        assert.match(r.err, /Tidak bisa membaca/);
    });
});

describe('env --json', () => {
    test('masks values by default', () => {
        const r = cli(['env', '--json', file('A=1\nB=2\nC=3\nS=s\n'), file('A=1\nB=9\nD=4\nS=s\n')]);
        assert.equal(r.code, 1);
        assert.deepEqual(JSON.parse(r.out), {
            onlyA: ['C'],
            onlyB: ['D'],
            different: [{ key: 'B' }],
            same: 2,
            invalidLines: [],
        });
    });

    test('--show-values adds values; invalid lines list file and line', () => {
        const a = file('A=1\nbad line\n');
        const b = file('A=2\n');
        const r = cli(['env', '--json', '-s', a, b]);
        assert.equal(r.code, 1);
        const doc = JSON.parse(r.out);
        assert.deepEqual(doc.different, [{ key: 'A', a: '1', b: '2' }]);
        assert.deepEqual(doc.invalidLines, [{ file: a, line: 2 }]);
    });

    test('no differences exits 0', () => {
        const r = cli(['env', '--json', file('A=1\n'), file('A=1\n')]);
        assert.equal(r.code, 0);
        assert.equal(JSON.parse(r.out).same, 1);
    });
});
