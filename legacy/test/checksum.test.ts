import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
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

const dir = mkdtempSync(join(tmpdir(), 'checksum-test-'));
after(() => rmSync(dir, { recursive: true, force: true }));

const ABC_SHA256 = 'ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad';
const ABC_MD5 = '900150983cd24fb0d6963f7d28e17f72';
const EMPTY_SHA256 = 'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855';

mkdirSync(join(dir, 'sub'));
writeFileSync(join(dir, 'abc.txt'), 'abc');
writeFileSync(join(dir, 'empty.txt'), '');
writeFileSync(join(dir, 'sub', 'spasi nama.txt'), 'abc');

describe('checksum', () => {
    test('prints sha256sum format for known vectors, in argument order', () => {
        const a = join(dir, 'abc.txt');
        const e = join(dir, 'empty.txt');
        const r = cli(['checksum', a, e]);
        assert.equal(r.code, 0);
        assert.equal(r.out, `${ABC_SHA256}  ${a}\n${EMPTY_SHA256}  ${e}\n`);
        assert.equal(cli(['checksum', '-a', 'md5', a]).out, `${ABC_MD5}  ${a}\n`);
    });

    test('errors: no files, directory, missing file, bad algorithm', () => {
        assert.equal(cli(['checksum']).code, 2);
        assert.equal(cli(['checksum', '-a', 'rot13', join(dir, 'abc.txt')]).code, 2);
        const d = cli(['checksum', join(dir, 'sub')]);
        assert.equal(d.code, 1);
        assert.ok(d.err.includes(join(dir, 'sub')));
        const m = cli(['checksum', join(dir, 'nope.txt')]);
        assert.equal(m.code, 1);
        assert.ok(m.err.includes(join(dir, 'nope.txt')));
    });
});

describe('checksum --check', () => {
    test('all OK: relative paths, binary marker, comments, uppercase hex, spaces in names', () => {
        const sums = join(dir, 'ok.sums');
        writeFileSync(
            sums,
            `# komentar\n\n${ABC_SHA256}  abc.txt\n${ABC_SHA256.toUpperCase()} *abc.txt\n${EMPTY_SHA256}  empty.txt\r\n${ABC_SHA256}  sub/spasi nama.txt\n`,
        );
        const r = cli(['checksum', '-c', sums]);
        assert.equal(r.code, 0);
        assert.equal(r.out, 'abc.txt: OK\nabc.txt: OK\nempty.txt: OK\nsub/spasi nama.txt: OK\n');
        assert.match(r.err, /4 OK, 0 GAGAL, 0 TIDAK ADA, 0 ERROR, 0 baris tidak valid/);
    });

    test('mismatch, missing file and malformed lines exit 1 with reports', () => {
        const sums = join(dir, 'bad.sums');
        const wrong = ABC_SHA256.replace(/^./, '0');
        writeFileSync(
            sums,
            `${wrong}  abc.txt\n${ABC_SHA256}  hilang.txt\nbukan-hex  abc.txt\n${ABC_MD5}  abc.txt\n${ABC_SHA256}  empty.txt\n`,
        );
        const r = cli(['checksum', '-c', sums]);
        assert.equal(r.code, 1);
        assert.equal(r.out, 'abc.txt: GAGAL\nhilang.txt: TIDAK ADA\nempty.txt: GAGAL\n');
        assert.match(r.err, /baris 3 tidak valid/);
        assert.match(r.err, /baris 4 tidak valid/);
        assert.match(r.err, /2 GAGAL/);
        assert.match(r.err, /1 TIDAK ADA, 0 ERROR, 2 baris tidak valid/);
    });

    test('digest length must match --algorithm', () => {
        const sums = join(dir, 'md5.sums');
        writeFileSync(sums, `${ABC_MD5}  abc.txt\n`);
        assert.equal(cli(['checksum', '-c', sums]).code, 1);
        const r = cli(['checksum', '-a', 'md5', '-c', sums]);
        assert.equal(r.code, 0);
        assert.equal(r.out, 'abc.txt: OK\n');
    });

    test('file arguments with --check are a usage error; missing/empty sums file fails', () => {
        const sums = join(dir, 'ok.sums');
        assert.equal(cli(['checksum', '-c', sums, 'abc.txt']).code, 2);
        assert.equal(cli(['checksum', '-c', join(dir, 'tidak-ada.sums')]).code, 1);
        const empty = join(dir, 'kosong.sums');
        writeFileSync(empty, '# saja\n');
        assert.equal(cli(['checksum', '-c', empty]).code, 1);
    });
});

describe('checksum --json', () => {
    test('print mode', () => {
        const r = cli(['checksum', '--json', '-a', 'md5', join(dir, 'abc.txt')]);
        assert.equal(r.code, 0);
        assert.deepEqual(JSON.parse(r.out), {
            algorithm: 'md5',
            files: [{ path: join(dir, 'abc.txt'), digest: ABC_MD5 }],
            errors: [],
        });
    });

    test('check mode reports statuses and exits 1 on failure', () => {
        const sums = join(dir, 'json.sums');
        const wrong = ABC_SHA256.replace(/^./, '0');
        writeFileSync(sums, `${ABC_SHA256}  abc.txt\n${wrong}  empty.txt\n${ABC_SHA256}  hilang.txt\nxx  abc.txt\n`);
        const r = cli(['checksum', '--json', '-c', sums]);
        assert.equal(r.code, 1);
        assert.deepEqual(JSON.parse(r.out), {
            algorithm: 'sha256',
            results: [
                { path: 'abc.txt', status: 'ok' },
                { path: 'empty.txt', status: 'failed' },
                { path: 'hilang.txt', status: 'missing' },
            ],
            malformed: [4],
            ok: 1,
            failed: 1,
            missing: 1,
            errors: 0,
            passed: false,
        });
    });

    test('check mode passed', () => {
        const sums = join(dir, 'json-ok.sums');
        writeFileSync(sums, `${ABC_SHA256}  abc.txt\n`);
        const r = cli(['checksum', '--json', '-c', sums]);
        assert.equal(r.code, 0);
        assert.equal(JSON.parse(r.out).passed, true);
    });
});

describe('checksum error handling', () => {
    const wrong = ABC_SHA256.replace(/^./, '0');
    const mixedSums = () => {
        const sums = join(dir, 'mixed.sums');
        writeFileSync(
            sums,
            `${ABC_SHA256}  sub\n${ABC_SHA256}  abc.txt/x\n${ABC_SHA256}  abc.txt\n${wrong}  empty.txt\n`,
        );
        return sums;
    };

    test('--check: directory and ENOTDIR entries are errors, the rest is still verified', () => {
        const r = cli(['checksum', '-c', mixedSums()]);
        assert.equal(r.code, 1);
        assert.match(r.out, /^sub: ERROR \(.+\)$/m);
        assert.match(r.out, /^abc\.txt\/x: ERROR \(.+\)$/m);
        assert.match(r.out, /^abc\.txt: OK$/m);
        assert.match(r.out, /^empty\.txt: GAGAL$/m);
        assert.match(r.err, /2 ERROR/);
    });

    test('--check --json: error entries carry a reason and are counted', () => {
        const r = cli(['checksum', '--json', '-c', mixedSums()]);
        assert.equal(r.code, 1);
        const doc = JSON.parse(r.out);
        assert.equal(doc.errors, 2);
        assert.equal(doc.ok, 1);
        assert.equal(doc.failed, 1);
        assert.equal(doc.passed, false);
        const byPath = Object.fromEntries(doc.results.map((x: { path: string }) => [x.path, x]));
        assert.equal(byPath['sub'].status, 'error');
        assert.equal(typeof byPath['sub'].error, 'string');
        assert.equal(byPath['abc.txt/x'].status, 'error');
        assert.equal(byPath['abc.txt'].status, 'ok');
    });

    test('missing sums file: Indonesian message, exit 1', () => {
        const r = cli(['checksum', '-c', join(dir, 'tidak-ada.sums')]);
        assert.equal(r.code, 1);
        assert.match(r.err, /tidak-ada\.sums: tidak dapat dibaca \(/);
    });

    test('unreadable sums path: wrapped message and no raw Node error', () => {
        const sums = join(dir, 'sub');
        const r = cli(['checksum', '--json', '-c', sums]);
        assert.equal(r.code, 1);
        assert.equal(r.out, '');
        assert.ok(r.err.includes(`${sums}: tidak dapat dibaca (`));
        assert.doesNotMatch(r.err, /illegal operation|read '/);
    });

    test('sums file without trailing newline and empty sums file', () => {
        const sums = join(dir, 'nonl.sums');
        writeFileSync(sums, `${ABC_SHA256}  abc.txt`);
        const r = cli(['checksum', '-c', sums]);
        assert.equal(r.code, 0);
        assert.equal(r.out, 'abc.txt: OK\n');
        const empty = join(dir, 'zero.sums');
        writeFileSync(empty, '');
        assert.equal(cli(['checksum', '-c', empty]).code, 1);
    });

    test('print mode: failure on one file does not hide other digests', () => {
        const r = cli(['checksum', join(dir, 'abc.txt'), join(dir, 'sub'), join(dir, 'empty.txt')]);
        assert.equal(r.code, 1);
        assert.equal(r.out, `${ABC_SHA256}  ${join(dir, 'abc.txt')}\n${EMPTY_SHA256}  ${join(dir, 'empty.txt')}\n`);
        assert.ok(r.err.includes(join(dir, 'sub')));
        const j = cli(['checksum', '--json', join(dir, 'abc.txt'), join(dir, 'sub'), join(dir, 'empty.txt')]);
        assert.equal(j.code, 1);
        assert.deepEqual(JSON.parse(j.out), {
            algorithm: 'sha256',
            files: [
                { path: join(dir, 'abc.txt'), digest: ABC_SHA256 },
                { path: join(dir, 'empty.txt'), digest: EMPTY_SHA256 },
            ],
            errors: [{ path: join(dir, 'sub'), message: 'direktori' }],
        });
    });

    test('non-regular files are rejected instead of read forever', { skip: !existsSync('/dev/zero') }, () => {
        const r = spawnSync(process.execPath, [CLI, 'checksum', '/dev/zero'], { encoding: 'utf8', timeout: 10000 });
        assert.equal(r.error, undefined, 'command must finish before the timeout');
        assert.equal(r.status, 1);
        assert.ok(r.stderr.includes('/dev/zero'));
        assert.match(r.stderr, /bukan file biasa/);
    });
});
