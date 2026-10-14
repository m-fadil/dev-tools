import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { describe, test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { inspect } from 'node:util';
import { parseUrl } from '../src/tools/url/url.js';

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

const FULL = 'https://user:s3cret@example.com:8443/a/b?x=1&y=a%20b&x=2+3#frag';

describe('url', () => {
    test('prints components, masks the password, lists query rows in order', () => {
        const r = cli(['url', FULL]);
        assert.equal(r.code, 0);
        assert.match(r.out, /^protocol: +https:$/m);
        assert.match(r.out, /^username: +user$/m);
        assert.match(r.out, /^password: +\*\*\*$/m);
        assert.match(r.out, /^port: +8443$/m);
        assert.match(r.out, /^origin: +https:\/\/example\.com:8443$/m);
        assert.doesNotMatch(r.out, /s3cret/);
        const query = r.out.slice(r.out.indexOf('Query:'));
        assert.deepEqual(query.trim().split('\n'), ['Query:', '  x = 1', '  y = a b', '  x = 2 3']);
    });

    test('--show-password reveals it; empty fields and absent query are omitted', () => {
        assert.match(cli(['url', '--show-password', FULL]).out, /^password: +s3cret$/m);
        const r = cli(['url', 'http://example.com']);
        assert.match(r.out, /^port: +\(default 80\)$/m);
        assert.doesNotMatch(r.out, /username|password|search|hash|Query/);
    });

    test('--query prints every decoded value of a key; missing key exits 1', () => {
        assert.equal(cli(['url', '-q', 'x', FULL]).out, '1\n2 3\n');
        assert.equal(cli(['url', '--query', 'y', FULL]).out, 'a b\n');
        const r = cli(['url', '-q', 'nope', FULL]);
        assert.equal(r.code, 1);
        assert.match(r.err, /nope/);
    });

    test('--json structure, password masking, stdin input with trailing newline', () => {
        const result = cli(['url', '--json'], FULL + '\n');
        assert.equal(result.code, 0);
        const masked = JSON.parse(result.out);
        assert.equal(result.out, `${JSON.stringify(masked, null, 2)}\n`, 'uses shared pretty-printed JSON output');
        assert.equal(masked.password, '***');
        assert.equal(masked.hostname, 'example.com');
        assert.equal(masked.port, '8443');
        assert.equal(masked.pathname, '/a/b');
        assert.equal(masked.hash, '#frag');
        assert.deepEqual(masked.query, [
            ['x', '1'],
            ['y', 'a b'],
            ['x', '2 3'],
        ]);
        assert.equal(JSON.parse(cli(['url', '--json', '--show-password', FULL]).out).password, 's3cret');
    });

    test('--json with --query is a usage error', () => {
        assert.equal(cli(['url', '--json', '-q', 'x', FULL]).code, 2);
    });

    test('invalid URL exits 1; missing scheme hints at https://', () => {
        const bad = cli(['url', 'example.com/path']);
        assert.equal(bad.code, 1);
        assert.match(bad.err, /URL tidak valid: example\.com\/path/);
        assert.match(bad.err, /https:\/\//);
        const worse = cli(['url', 'http://']);
        assert.equal(worse.code, 1);
        assert.match(worse.err, /URL tidak valid: http:\/\//);
    });

    test('invalid URL diagnostics redact credentials even with --show-password or --json', () => {
        for (const url of [
            'https://user:SECRETPW@exa mple.com/',
            'https://user:SECRETPW@example.com:bad/',
            'https://user:SECRETPW@example.com:65536/',
            'https://user:SECRETPW/extra@exa mple.com/',
            'https://user:SECRETPW?extra@exa mple.com/',
            'https://user:SECRETPW#extra@exa mple.com/',
            '//user:SECRETPW@exa mple.com/',
        ]) {
            for (const options of [[], ['--show-password'], ['--json']]) {
                const r = cli(['url', ...options, url]);
                assert.equal(r.code, 1, url);
                assert.equal(r.out, '');
                assert.doesNotMatch(r.err, /SECRETPW|user:/);
                assert.match(r.err, /\*\*\*@/);
            }
        }
    });

    test('invalid URL errors and their causes never retain raw credentials', () => {
        const url = 'https://user:SECRETPW@exa mple.com/';
        assert.throws(
            () => parseUrl(url, true),
            (error: unknown) => {
                assert.ok(error instanceof Error);
                assert.match(error.message, /https:\/\/\*\*\*@exa mple\.com\//);
                assert.doesNotMatch(inspect(error, { depth: null }), /SECRETPW|user:/);
                return true;
            },
        );
    });

    test('opaque URLs keep their address and have no userinfo', () => {
        const r = cli(['url', '--json', 'mailto:user@example.com']);
        assert.equal(r.code, 0);
        const parsed = JSON.parse(r.out);
        assert.equal(parsed.pathname, 'user@example.com');
        assert.equal(parsed.username, '');
        assert.equal(parsed.password, '');
    });
});
