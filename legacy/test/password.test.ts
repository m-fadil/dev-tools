import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { describe, test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { UsageError } from '../src/errors.js';
import { characterClasses, generatePassword } from '../src/tools/password/generate.js';

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

const lines = (out: string) => out.trimEnd().split('\n');

describe('password', () => {
    test('default: 20 chars of lower+upper+digits, one per line for --count', () => {
        const r = cli(['password', '-c', '50']);
        assert.equal(r.code, 0);
        const passwords = lines(r.out);
        assert.equal(passwords.length, 50);
        for (const p of passwords) assert.match(p, /^[A-Za-z0-9]{20}$/);
        assert.equal(new Set(passwords).size, 50);
    });

    test('every active class is present even at the minimum length', () => {
        for (const p of lines(cli(['password', '-l', '4', '-s', '-c', '300']).out)) {
            assert.equal(p.length, 4);
            assert.match(p, /[a-z]/);
            assert.match(p, /[A-Z]/);
            assert.match(p, /[0-9]/);
            assert.match(p, /[!@#$%^&*()\-_=+[\]{};:,.?]/);
        }
        for (const p of lines(cli(['password', '-l', '4', '-c', '300']).out)) {
            assert.match(p, /^(?=.*[a-z])(?=.*[A-Z])(?=.*[0-9])[A-Za-z0-9]{4}$/);
        }
    });

    test('length boundaries 4 and 1024', () => {
        assert.equal(lines(cli(['password', '-l', '1024']).out)[0]?.length, 1024);
        for (const bad of ['3', '1025', '0', '-5', '1.5', 'abc', '']) {
            const r = cli(['password', '-l', bad]);
            assert.equal(r.code, 2, `length ${bad}`);
        }
        assert.equal(cli(['password', '-c', '1000']).out.trimEnd().split('\n').length, 1000);
        assert.equal(cli(['password', '-c', '1001']).code, 2);
        assert.equal(cli(['password', '-c', '0']).code, 2);
    });

    test('--only restricts the alphabet', () => {
        const only = (set: string, c = '100') => lines(cli(['password', '--only', set, '-l', '40', '-c', c]).out);
        for (const p of only('lower')) assert.match(p, /^[a-z]{40}$/);
        for (const p of only('upper')) assert.match(p, /^[A-Z]{40}$/);
        for (const p of only('digits')) assert.match(p, /^[0-9]{40}$/);
        for (const p of only('hex')) assert.match(p, /^[0-9a-f]{40}$/);
        for (const p of only('alnum')) assert.match(p, /^(?=.*[a-z])(?=.*[A-Z])(?=.*[0-9])[A-Za-z0-9]{40}$/);
    });

    test('--only with --symbols, unknown --only, and positionals are usage errors', () => {
        const mixed = cli(['password', '--only', 'hex', '-s']);
        assert.equal(mixed.code, 2);
        assert.match(mixed.err, /--only/);
        assert.equal(mixed.out, '');
        const unknown = cli(['password', '--only', 'emoji']);
        assert.equal(unknown.code, 2);
        assert.match(unknown.err, /--only tidak valid: emoji/);
        assert.equal(cli(['password', 'extra']).code, 2);
    });

    test('--unambiguous never emits 0 O 1 l I |', () => {
        const all = lines(cli(['password', '-u', '-s', '-l', '200', '-c', '50']).out);
        for (const p of all) assert.doesNotMatch(p, /[0O1lI|]/);
        for (const p of lines(cli(['password', '-u', '--only', 'hex', '-l', '100']).out)) {
            assert.match(p, /^[2-9a-f]{100}$/);
        }
        for (const p of lines(cli(['password', '-u', '--only', 'digits', '-l', '100']).out)) {
            assert.match(p, /^[2-9]{100}$/);
        }
    });

    test('--entropy prints an estimate to stderr only', () => {
        const base = cli(['password', '-e']);
        assert.equal(base.code, 0);
        assert.equal(base.err.trim(), 'Entropi: ~119 bit'); // 20 * log2(62)
        assert.match(base.out, /^[A-Za-z0-9]{20}\n$/);
        assert.equal(cli(['password', '-e', '-s']).err.trim(), 'Entropi: ~128 bit'); // 20 * log2(85)
        assert.equal(cli(['password', '-e', '-u']).err.trim(), 'Entropi: ~117 bit'); // 20 * log2(57)
        assert.equal(cli(['password', '-e', '--only', 'hex', '-l', '32']).err.trim(), 'Entropi: ~128 bit');
        assert.equal(cli(['password']).err, '');
    });
});

describe('password generator module', () => {
    test('placement and shuffle are deterministic with an injected randomInt', () => {
        // randomInt always 0: required 'a','A', fill 'a'; the shuffle then yields 'Aaa'
        assert.equal(
            generatePassword(['ab', 'AB'], 3, () => 0),
            'Aaa',
        );
        assert.equal(characterClasses({ symbols: true, only: undefined, unambiguous: false }).length, 4);
    });

    test('randomInt is only asked for in-range values', () => {
        const seen: number[] = [];
        const spy = (max: number) => {
            assert.ok(Number.isSafeInteger(max) && max >= 1);
            seen.push(max);
            return max - 1;
        };
        const p = generatePassword(['xy', '12'], 6, spy);
        assert.equal(p.length, 6);
        assert.ok(seen.every((m) => m <= 6));
    });

    test('length below the number of classes is a UsageError', () => {
        assert.throws(() => generatePassword(['a', 'B', '1', '!'], 3), UsageError);
    });
});

describe('password --json', () => {
    test('values only, without entropy', () => {
        const r = cli(['password', '--json', '-c', '3', '-l', '12']);
        assert.equal(r.code, 0);
        const doc = JSON.parse(r.out);
        assert.deepEqual(Object.keys(doc), ['values']);
        assert.equal(doc.values.length, 3);
        for (const v of doc.values) assert.equal(v.length, 12);
    });

    test('--entropy goes into the document, not stderr', () => {
        const r = cli(['password', '--json', '-e', '--only', 'hex', '-l', '16']);
        assert.equal(r.err, '');
        assert.equal(JSON.parse(r.out).entropyBits, 64);
    });
});
