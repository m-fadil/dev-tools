import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { describe, test } from 'node:test';
import { fileURLToPath } from 'node:url';

const CLI = fileURLToPath(new URL('../src/cli.js', import.meta.url));

/** run the CLI without a terminal; `input` is piped to stdin */
const cli = (args: string[], input = '', timeout?: number) => {
    const r = spawnSync(process.execPath, [CLI, ...args], {
        encoding: 'utf8',
        input,
        env: { PATH: process.env.PATH },
        timeout,
    });
    return { code: r.status, out: r.stdout, err: r.stderr };
};

describe('regex', () => {
    test('lists matches with numbered and named groups, unmatched groups as -', () => {
        const r = cli(['regex', '(\\d+)-(?<word>[a-z]+)?', '12-ab 7-']);
        assert.equal(r.code, 0, r.err);
        assert.equal(
            r.out,
            [
                '#1 index=0 "12-ab"',
                '  $1 "12"',
                '  $2 "ab"',
                '  <word> "ab"',
                '#2 index=6 "7-"',
                '  $1 "7"',
                '  $2 -',
                '  <word> -',
                '',
            ].join('\n'),
        );
    });

    test('text from stdin is used verbatim; --first stops at the first match', () => {
        const r = cli(['regex', '-f', 'i', '--first', 'A'], 'bab\nbAb\n');
        assert.equal(r.out, '#1 index=1 "a"\n');
        assert.equal(cli(['regex', '$'], 'x\n').out, '#1 index=2 ""\n');
    });

    test('zero-length matches terminate and advance, also over surrogate pairs with u', () => {
        assert.equal(cli(['regex', '--json', 'x*', 'ab']).code, 0);
        const plain = JSON.parse(cli(['regex', '--json', 'x*', 'ab']).out);
        assert.deepEqual(
            plain.matches.map((m: { index: number }) => m.index),
            [0, 1, 2],
        );
        const unicode = JSON.parse(cli(['regex', '--json', '-f', 'u', '', '😀a']).out);
        assert.deepEqual(
            unicode.matches.map((m: { index: number }) => m.index),
            [0, 2, 3],
        );
    });

    test('--replace uses JS replacement syntax and honours --first', () => {
        const r = cli(['regex', '-r', '$<b>-$1', '(?<b>\\d)(x)', '1x 2x']);
        assert.match(r.out, /Hasil ganti:\n1-1 2-2\n$/);
        const first = JSON.parse(cli(['regex', '--json', '--first', '-r', '[$&]', '\\d', '1 2 3']).out);
        assert.equal(first.replaced, '[1] 2 3');
    });

    test('--json shape', () => {
        const r = cli(['regex', '--json', '-f', 'gi', '(a)|(?<n>b)', 'Ab']);
        assert.deepEqual(JSON.parse(r.out), {
            pattern: '(a)|(?<n>b)',
            flags: 'gi',
            matches: [
                { index: 0, match: 'A', groups: ['A', null], named: { n: null } },
                { index: 1, match: 'b', groups: [null, 'b'], named: { n: 'b' } },
            ],
        });
        assert.equal('replaced' in JSON.parse(cli(['regex', '--json', 'z', 'a']).out), false);
    });

    test('--test exits 0 or 1 silently', () => {
        const yes = cli(['regex', '--test', 'b', 'abc']);
        assert.deepEqual([yes.code, yes.out, yes.err], [0, '', '']);
        const no = cli(['regex', '--test', 'z', 'abc']);
        assert.deepEqual([no.code, no.out], [1, '']);
    });

    test('invalid pattern, flags and option combinations are usage errors', () => {
        const bad = cli(['regex', '(', 'x']);
        assert.equal(bad.code, 2);
        assert.match(bad.err, /Regex tidak valid: .*Unterminated group/);
        assert.equal(cli(['regex', '-f', 'q', 'a', 'a']).code, 2);
        assert.equal(cli(['regex', '-f', 'gg', 'a', 'a']).code, 2);
        assert.equal(cli(['regex', '--timeout', '0', 'a', 'a']).code, 2);
        assert.equal(cli(['regex', '--test', '--json', 'a', 'a']).code, 2);
        assert.equal(cli(['regex']).code, 2);
    });

    test('catastrophic backtracking is cut off by the timeout', () => {
        const started = Date.now();
        const r = cli(['regex', '--timeout', '300', '^(a+)+$', `${'a'.repeat(40)}!`]);
        assert.equal(r.code, 1);
        assert.match(r.err, /regex timeout \(kemungkinan catastrophic backtracking\)/);
        assert.ok(Date.now() - started < 10000);
    });
});

describe('regex memory limit', () => {
    test('exploding --replace output fails fast with a memory message', () => {
        const started = Date.now();
        // One record per character exceeds the worker heap limit well before all
        // replacements finish. Pipe the input: multi-MB argv would fail with E2BIG.
        const r = cli(['regex', '-r', '$&$&$&$&', '--timeout', '30000', 'x'], 'x'.repeat(4 * 1024 * 1024), 15000);
        assert.equal(r.code, 1);
        assert.match(r.err, /regex melebihi batas memori/);
        assert.ok(Date.now() - started < 15000);
    });
});
