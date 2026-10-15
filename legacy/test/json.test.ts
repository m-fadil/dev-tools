import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { after, describe, test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { JsonTooDeepError, MAX_DEPTH, stringifyJson } from '../src/tools/json/json.js';

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

const dir = await mkdtemp(join(tmpdir(), 'dt-json-'));
after(() => rm(dir, { recursive: true, force: true }));

describe('json', () => {
    test('pretty prints with 2 spaces by default, honours --indent and --minify', () => {
        assert.equal(cli(['json', '{"a":[1,2]}']).out, '{\n  "a": [\n    1,\n    2\n  ]\n}\n');
        assert.equal(cli(['json', '--indent', '4', '[1]']).out, '[\n    1\n]\n');
        assert.equal(cli(['json', '--indent', '0', '[1, 2]']).out, '[1,2]\n');
        assert.equal(cli(['json', '-m'], '{ "a": 1,\n "b": [ true, null ] }\n').out, '{"a":1,"b":[true,null]}\n');
    });

    test('reads from --file and strips a BOM', async () => {
        const file = join(dir, 'a.json');
        await writeFile(file, '\uFEFF{"a":1}');
        assert.equal(cli(['json', '-m', '-f', file]).out, '{"a":1}\n');
    });

    test('--sort-keys is recursive and keeps array order; __proto__ stays data', () => {
        assert.equal(cli(['json', '-m', '-s', '{"b":1,"a":[{"d":1,"c":2},0]}']).out, '{"a":[{"c":2,"d":1},0],"b":1}\n');
        assert.equal(cli(['json', '-m', '-s', '{"b":1,"__proto__":2}']).out, '{"__proto__":2,"b":1}\n');
    });

    test('--check reports valid input and line:column of the error', () => {
        const ok = cli(['json', '--check'], '{"a":[1,2]}\n');
        assert.deepEqual([ok.code, ok.out], [0, 'valid\n']);
        const unexpected = cli(['json', '--check'], '[1,\n2,\nx]');
        assert.equal(unexpected.code, 1);
        assert.match(unexpected.err, /invalid: 3:1 /);
        const truncated = cli(['json', '--check', '{"a":1']);
        assert.equal(truncated.code, 1);
        assert.match(truncated.err, /invalid: 1:7 /);
        const trailing = cli(['json', '--check', '{"a":1} x']);
        assert.match(trailing.err, /invalid: 1:9 /);
        assert.match(cli(['json', '--check'], '').err, /invalid: 1:1 /);
    });

    test('invalid JSON without --check fails with exit 1 and no stdout', () => {
        const r = cli(['json', '{"a":}']);
        assert.equal(r.code, 1);
        assert.equal(r.out, '');
        assert.match(r.err, /invalid: 1:6 /);
    });

    test('--query paths: dots, indexes, quoted keys, root prefixes', () => {
        const doc = '{"a":{"b":[10,{"c":"x"}]},"k.d":{"e f":true},"n":null}';
        const q = (path: string, ...extra: string[]) => cli(['json', '-q', path, ...extra, doc]);
        assert.equal(q('a.b[0]').out, '10\n');
        assert.equal(q('$.a.b[1].c').out, '"x"\n');
        assert.equal(q('.a.b[1].c', '-r').out, 'x\n');
        assert.equal(q('["k.d"]["e f"]').out, 'true\n');
        assert.equal(q('n').out, 'null\n');
        assert.equal(q('$').out, cli(['json', doc]).out);
        assert.equal(q('a.b', '-m').out, '[10,{"c":"x"}]\n');
        assert.equal(q('a.b[0]', '-r').out, '10\n');
    });

    test('missing query path exits 1; malformed path is a usage error', () => {
        for (const path of ['a.zzz', 'a.b[5]', 'a.b.c', 'a[0]', 'toString']) {
            const r = cli(['json', '-q', path, '{"a":{"b":[1]}}']);
            assert.equal(r.code, 1, path);
            assert.equal(r.err.trim(), `path tidak ditemukan: ${path}`);
        }
        for (const path of ['a[', 'a[x]', 'a[-1]', 'a..b', 'a[0]b', '["x]']) {
            assert.equal(cli(['json', '-q', path, '{}']).code, 2, path);
        }
    });

    test('conflicting or out-of-range options are usage errors', () => {
        assert.equal(cli(['json', '--check', '--query', 'a', '{}']).code, 2);
        assert.equal(cli(['json', '--check', '-m', '{}']).code, 2);
        assert.equal(cli(['json', '-m', '--indent', '4', '{}']).code, 2);
        assert.equal(cli(['json', '--indent', '9', '{}']).code, 2);
        assert.equal(cli(['json', '--indent', '-1', '{}']).code, 2);
        assert.equal(cli(['json', '-r', '{}']).code, 2);
    });
});

describe('json hardening', () => {
    test('error position ignores text quoted from the input', () => {
        for (const input of ['a at position 99999', 'x end of JSON input', '\nx at position 99999']) {
            const r = cli(['json', '--check', input]);
            assert.equal(r.code, 1, input);
            assert.equal(r.out, '');
            assert.match(r.err, input.startsWith('\n') ? /invalid: 2:1 / : /invalid: 1:1 /);
        }
    });

    test('deep nesting gives a clear error, not a stack overflow', () => {
        const deep = '['.repeat(200000);
        for (const [args, input] of [
            [['json', '--check'], deep],
            [['json', '--check'], deep + ']'.repeat(200000)],
            [['json', '-s'], deep + ']'.repeat(200000)],
            [['json'], deep + ']'.repeat(200000)],
            [['json', '-m'], deep + ']'.repeat(200000)],
            [['json', '-s', '-m'], '{"a":'.repeat(200000) + '0' + '}'.repeat(200000)],
        ] as const) {
            const r = cli([...args], input);
            assert.equal(r.code, 1, args.join(' '));
            assert.equal(r.out, '');
            assert.match(r.err, new RegExp(`JSON terlalu dalam \\(maks ${MAX_DEPTH} level\\)`));
            assert.doesNotMatch(r.err, /RangeError|call stack/);
        }
    });

    test('nesting limit counts containers consistently, including empty containers', () => {
        for (const [open, close] of [
            ['[', ']'],
            ['{"a":', '}'],
        ] as const) {
            for (const value of ['0', open === '[' ? '[]' : '{}']) {
                const depth = value === '0' ? MAX_DEPTH : MAX_DEPTH - 1;
                const input = open.repeat(depth) + value + close.repeat(depth);
                const checked = cli(['json', '--check'], input);
                assert.deepEqual([checked.code, checked.out, checked.err], [0, 'valid\n', '']);
                const sorted = cli(['json', '-s', '-m'], input);
                assert.deepEqual([sorted.code, sorted.out, sorted.err], [0, input + '\n', '']);
            }
            const tooDeep = open.repeat(MAX_DEPTH + 1) + '0' + close.repeat(MAX_DEPTH + 1);
            for (const args of [
                ['json', '--check'],
                ['json', '-s', '-m'],
                ['json', '-m'],
            ]) {
                const r = cli(args, tooDeep);
                assert.equal(r.code, 1);
                assert.equal(r.err.trim(), `JSON terlalu dalam (maks ${MAX_DEPTH} level)`);
            }
        }
        const malformed = cli(['json', '--check'], '['.repeat(MAX_DEPTH) + 'x');
        assert.equal(malformed.code, 1);
        assert.match(malformed.err, new RegExp(`invalid: 1:${MAX_DEPTH + 1} `));
        const bracketsInString = JSON.stringify('['.repeat(MAX_DEPTH + 1));
        assert.equal(cli(['json', '--check'], bracketsInString).out, 'valid\n');
    });

    test('stringify converts native stack exhaustion to the depth error', () => {
        const data: unknown = JSON.parse('['.repeat(200000) + ']'.repeat(200000));
        assert.throws(() => stringifyJson(data, undefined), JsonTooDeepError);
    });

    test('query indexes reject leading zeros and unsafe integers', () => {
        for (const path of ['[01]', '[00]', '[000]', '[9007199254740992]', '[99999999999999999999]', 'a[01]']) {
            const r = cli(['json', '-q', path, '[1,2]']);
            assert.equal(r.code, 2, path);
            assert.equal(r.out, '');
            assert.match(r.err, /Path query tidak valid/);
        }
        assert.equal(cli(['json', '-q', '[9007199254740991]', '[1,2]']).code, 1);
        assert.equal(cli(['json', '-m', '-q', '[0]', '[1,2]']).out, '1\n');
        assert.equal(cli(['json', '-q', '["01"]', '{"01":"key"}']).out, '"key"\n');
    });
});
