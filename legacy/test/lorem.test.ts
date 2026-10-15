import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { describe, test } from 'node:test';
import { fileURLToPath } from 'node:url';

const CLI = fileURLToPath(new URL('../src/cli.js', import.meta.url));

const cli = (args: string[], input = '') => {
    const r = spawnSync(process.execPath, [CLI, ...args], { encoding: 'utf8', input, env: { PATH: process.env.PATH } });
    return { code: r.status, out: r.stdout, err: r.stderr };
};

describe('lorem', () => {
    test('default is 3 paragraphs separated by blank lines', () => {
        const r = cli(['lorem', '--seed', '1']);
        assert.equal(r.code, 0);
        const paragraphs = r.out.trimEnd().split('\n\n');
        assert.equal(paragraphs.length, 3);
        for (const p of paragraphs) {
            const sentences = p.match(/[^.]+\./g) ?? [];
            assert.ok(sentences.length >= 3 && sentences.length <= 7);
        }
    });

    test('seed makes output reproducible', () => {
        const a = cli(['lorem', '-p', '2', '--seed', '42']).out;
        assert.equal(cli(['lorem', '-p', '2', '--seed', '42']).out, a);
        assert.notEqual(cli(['lorem', '-p', '2', '--seed', '43']).out, a);
    });

    test('words count is exact, sentences are capitalised and end with a period', () => {
        assert.equal(cli(['lorem', '-w', '37', '--seed', '3']).out.trim().split(' ').length, 37);
        const s = cli(['lorem', '-s', '4', '--seed', '3']).out.trim();
        assert.match(s, /^[A-Z][^.]*\.( [A-Z][^.]*\.){3}$/);
        for (const sentence of s.match(/[^.]+\./g) ?? []) {
            const n = sentence.trim().split(' ').length;
            assert.ok(n >= 5 && n <= 15, sentence);
        }
    });

    test('--classic starts with the classic opening', () => {
        assert.ok(
            cli(['lorem', '-s', '2', '--classic']).out.startsWith(
                'Lorem ipsum dolor sit amet, consectetur adipiscing elit',
            ),
        );
        assert.ok(
            cli(['lorem', '-p', '1', '--classic']).out.startsWith(
                'Lorem ipsum dolor sit amet, consectetur adipiscing elit',
            ),
        );
        assert.equal(cli(['lorem', '-w', '3', '--classic']).out.trim(), 'lorem ipsum dolor');
    });

    test('maximum word count works with and without the classic opening', () => {
        for (const extra of [[], ['--classic']]) {
            const r = cli(['lorem', '-w', '5000', '--seed', '1', ...extra]);
            assert.equal(r.code, 0, r.err);
            assert.equal(r.out.trimEnd().split(' ').length, 5000);
        }
    });

    test('limits are enforced with exit 2', () => {
        assert.equal(cli(['lorem', '-w', '5000', '--seed', '1']).code, 0);
        for (const args of [
            ['-w', '0'],
            ['-w', '5001'],
            ['-s', '501'],
            ['-p', '101'],
            ['-p', 'x'],
            ['-w', '1', '-p', '1'],
            ['extra'],
        ]) {
            const r = cli(['lorem', ...args]);
            assert.equal(r.code, 2, args.join(' '));
            assert.equal(r.out, '');
        }
        assert.equal(cli(['lorem', '--seed', '-1']).code, 2);
    });

    test('--json shape', () => {
        const j = JSON.parse(cli(['lorem', '-s', '2', '--seed', '5', '--json']).out);
        assert.deepEqual(Object.keys(j), ['unit', 'count', 'text']);
        assert.equal(j.unit, 'sentences');
        assert.equal(j.count, 2);
        assert.equal(typeof j.text, 'string');
        assert.equal(JSON.parse(cli(['lorem', '--json']).out).unit, 'paragraphs');
    });
});
