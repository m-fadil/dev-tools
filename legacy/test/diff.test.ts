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

const dir = mkdtempSync(join(tmpdir(), 'dt-diff-'));
after(() => rmSync(dir, { recursive: true, force: true }));

const file = (name: string, content: string | Buffer): string => {
    const path = join(dir, name);
    writeFileSync(path, content);
    return path;
};

/** hunk lines only (drops the summary and ---/+++ header) */
const hunks = (out: string): string => out.slice(out.indexOf('@@'));

describe('diff', () => {
    test('identical files exit 0 with the known sha256', () => {
        const a = file('same-a.txt', 'abc');
        const b = file('same-b.txt', 'abc');
        const r = cli(['diff', a, b]);
        assert.equal(r.code, 0);
        assert.equal(
            r.out.trim(),
            'Identik (sha256 ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad, 3 byte)',
        );
    });

    test('empty files are identical', () => {
        const r = cli(['diff', file('e1', ''), file('e2', '')]);
        assert.equal(r.code, 0);
        assert.match(r.out, /^Identik \(sha256 e3b0c44298fc1c14.*, 0 byte\)/);
    });

    test('different text: exit 1, sizes, hashes and a minimal unified diff', () => {
        const a = file('t1-a.txt', 'satu\ndua\ntiga\n');
        const b = file('t1-b.txt', 'satu\nDUA\ntiga\n');
        const r = cli(['diff', a, b]);
        assert.equal(r.code, 1);
        assert.equal(r.err, '');
        assert.match(r.out, /a: .*t1-a\.txt {2}14 byte {2}sha256 [0-9a-f]{64}/);
        assert.match(r.out, /--- .*t1-a\.txt\n\+\+\+ .*t1-b\.txt\n/);
        assert.equal(hunks(r.out), '@@ -1,3 +1,3 @@\n satu\n-dua\n+DUA\n tiga\n');
    });

    test('insertions and deletions use LCS, not positional comparison', () => {
        const a = file('lcs-a', 'a\nb\nc\nd\ne\n');
        const b = file('lcs-b', 'b\nc\nx\nd\ne\nf\n');
        const r = cli(['diff', '-U', '0', a, b]);
        assert.equal(hunks(r.out), '@@ -1,1 +0,0 @@\n-a\n@@ -3,0 +3,1 @@\n+x\n@@ -5,0 +6,1 @@\n+f\n');
    });

    test('context option splits and merges hunks', () => {
        const lines = Array.from({ length: 20 }, (_, i) => `l${i + 1}`);
        const changed = [...lines];
        changed[1] = 'X2';
        changed[17] = 'X18';
        const a = file('ctx-a', lines.join('\n') + '\n');
        const b = file('ctx-b', changed.join('\n') + '\n');
        const small = hunks(cli(['diff', '-U', '1', a, b]).out);
        assert.equal(small.match(/^@@/gm)?.length, 2);
        assert.match(small, /^@@ -1,3 \+1,3 @@$/m);
        assert.match(small, /^@@ -17,3 \+17,3 @@$/m);
        const wide = hunks(cli(['diff', '-U', '20', a, b]).out);
        assert.equal(wide.match(/^@@/gm)?.length, 1);
        assert.match(wide, /^@@ -1,20 \+1,20 @@$/m);
    });

    test('missing trailing newline is a real difference and is marked', () => {
        const a = file('nl-a', 'x\ny');
        const b = file('nl-b', 'x\ny\n');
        const r = cli(['diff', a, b]);
        assert.equal(r.code, 1);
        assert.equal(hunks(r.out), '@@ -1,2 +1,2 @@\n x\n-y\n\\ Tidak ada newline di akhir file\n+y\n');
    });

    test('adding to an empty file', () => {
        const r = cli(['diff', file('em-a', ''), file('em-b', 'baru\n')]);
        assert.equal(r.code, 1);
        assert.equal(hunks(r.out), '@@ -0,0 +1,1 @@\n+baru\n');
    });

    test('binary files report the first differing byte offset', () => {
        const a = file('bin-a', Buffer.from([1, 2, 0, 4, 5]));
        const b = file('bin-b', Buffer.from([1, 2, 0, 9, 5]));
        const r = cli(['diff', a, b]);
        assert.equal(r.code, 1);
        assert.match(r.out, /^Biner, berbeda di byte offset 3$/m);
        assert.doesNotMatch(r.out, /@@/);
    });

    test('binary offset when one file is a prefix of the other', () => {
        const a = file('pre-a', Buffer.from([0, 1, 2]));
        const b = file('pre-b', Buffer.from([0, 1, 2, 3]));
        assert.match(cli(['diff', a, b]).out, /byte offset 3$/m);
    });

    test('binary offset beyond the first read block', () => {
        const base = Buffer.alloc(200_000, 7);
        base[0] = 0;
        const other = Buffer.from(base);
        other[150_000] = 8;
        assert.match(cli(['diff', file('big-a', base), file('big-b', other)]).out, /byte offset 150000$/m);
    });

    test('files over 5 MiB get a summary and a git hint only', () => {
        const a = file('huge-a', Buffer.alloc(5 * 1024 * 1024 + 1, 0x61));
        const b = file('huge-b', Buffer.alloc(5 * 1024 * 1024 + 1, 0x62));
        const r = cli(['diff', a, b]);
        assert.equal(r.code, 1);
        assert.match(r.out, /git diff --no-index/);
        assert.doesNotMatch(r.out, /@@/);
    });

    test('too many lines for the quadratic guard gives a summary and a git hint', () => {
        // 6000 x 6000 lines > 25,000,000
        const a = file('many-a', 'a\n'.repeat(6000));
        const b = file('many-b', 'b\n'.repeat(6000));
        const r = cli(['diff', a, b]);
        assert.equal(r.code, 1);
        assert.match(r.out, /git diff --no-index/);
        assert.doesNotMatch(r.out, /@@/);
    });

    test('missing file is an error on stderr naming the path', () => {
        const a = file('ok.txt', 'x');
        const missing = join(dir, 'tidak-ada.txt');
        const r = cli(['diff', a, missing]);
        assert.equal(r.code, 1);
        assert.equal(r.out, '');
        assert.ok(r.err.includes(missing));
    });

    test('usage errors: wrong operand count and bad --context', () => {
        const a = file('u.txt', 'x');
        assert.equal(cli(['diff', a]).code, 2);
        assert.equal(cli(['diff', a, a, a]).code, 2);
        const r = cli(['diff', '-U', '21', a, a]);
        assert.equal(r.code, 2);
        assert.match(r.err, /--context/);
        assert.equal(cli(['diff', '-U', 'x', a, a]).code, 2);
    });
});

describe('diff --json', () => {
    test('identical', () => {
        const a = file('j1', 'x\n');
        const r = cli(['diff', '--json', a, file('j2', 'x\n')]);
        assert.equal(r.code, 0);
        const doc = JSON.parse(r.out);
        assert.equal(doc.identical, true);
        assert.equal(doc.kind, 'identical');
        assert.equal(doc.a.size, 2);
        assert.equal(doc.a.sha256, doc.b.sha256);
        assert.equal(doc.hunks, undefined);
    });

    test('text difference has hunks and exit 1', () => {
        const r = cli(['diff', '--json', '-U', '0', file('j3', 'a\nb\nc\n'), file('j4', 'a\nB\nc\n')]);
        assert.equal(r.code, 1);
        const doc = JSON.parse(r.out);
        assert.equal(doc.kind, 'text');
        assert.equal(doc.identical, false);
        assert.deepEqual(doc.hunks, [
            {
                aStart: 2,
                aLines: 1,
                bStart: 2,
                bLines: 1,
                lines: [
                    { op: '-', text: 'b' },
                    { op: '+', text: 'B' },
                ],
            },
        ]);
    });

    test('binary difference reports the first differing byte', () => {
        const pa = join(dir, 'jb1');
        const pb = join(dir, 'jb2');
        writeFileSync(pa, Buffer.from([1, 0, 2]));
        writeFileSync(pb, Buffer.from([1, 0, 3]));
        const r = cli(['diff', '--json', pa, pb]);
        assert.equal(r.code, 1);
        const doc = JSON.parse(r.out);
        assert.equal(doc.kind, 'binary');
        assert.equal(doc.firstDifferingByte, 2);
    });

    test('missing file: exit 1, no stdout', () => {
        const r = cli(['diff', '--json', join(dir, 'nope'), join(dir, 'nope2')]);
        assert.equal(r.code, 1);
        assert.equal(r.out, '');
    });
});
