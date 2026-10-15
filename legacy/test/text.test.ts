import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { describe, test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { processLines } from '../src/tools/text/analyze.js';

const CLI = fileURLToPath(new URL('../src/cli.js', import.meta.url));

/** run the CLI without a terminal; `input` is piped to stdin */
const cli = (args: string[], input = '') => {
    const r = spawnSync(process.execPath, [CLI, ...args], { encoding: 'utf8', input, env: { PATH: process.env.PATH } });
    return { code: r.status, out: r.stdout, err: r.stderr };
};

const text = (args: string[], input = '') => cli(['text', ...args], input);

describe('text stats', () => {
    test('counts', () => {
        const r = text(['stats', '--json'], 'Hello, wörld! 👨‍👩‍👧 Ok.\nsecond line here\n');
        assert.equal(r.code, 0);
        assert.deepEqual(JSON.parse(r.out), {
            characters: 40,
            graphemes: 36,
            bytes: 54,
            words: 6,
            lines: 2,
            sentences: 3,
            longestLine: { number: 1, length: 19 },
            averageWordLength: 4.33,
        });
    });

    test('empty input and plain output', () => {
        assert.deepEqual(JSON.parse(text(['stats', '--json'], '').out), {
            characters: 0,
            graphemes: 0,
            bytes: 0,
            words: 0,
            lines: 0,
            sentences: 0,
            longestLine: { number: 0, length: 0 },
            averageWordLength: 0,
        });
        assert.match(text(['stats', 'a b']).out, /^kata\s+2$/m);
        // decimals do not end a sentence
        assert.equal(JSON.parse(text(['stats', '--json', 'Pi is 3.14. Yes']).out).sentences, 2);
    });
});

describe('text reverse', () => {
    test('by grapheme, per line; --lines reverses order', () => {
        assert.equal(text(['reverse', 'abc'], '').out, 'cba\n');
        assert.equal(text(['reverse'], 'ab\ncd\n').out, 'ba\ndc\n');
        assert.equal(text(['reverse', 'e\u0301a👍🏽']).out, '👍🏽ae\u0301\n');
        assert.equal(text(['reverse', '--lines'], 'ab\ncd\nef\n').out, 'ef\ncd\nab\n');
        assert.deepEqual(JSON.parse(text(['reverse', '--json'], 'ab\ncd').out), { lines: ['ba', 'dc'] });
    });
});

describe('text slug', () => {
    test('diacritics, collapse, separator', () => {
        assert.equal(text(['slug', '  Héllo,   Wörld!! Ça va? ']).out, 'hello-world-ca-va\n');
        assert.equal(text(['slug', '--sep', '_', "It's A Test--2"]).out, 'its_a_test_2\n');
        assert.deepEqual(JSON.parse(text(['slug', '--json', 'A B']).out), { slug: 'a-b' });
    });

    test('empty slug and bad separator', () => {
        const empty = text(['slug', '!!! ---']);
        assert.equal(empty.code, 1);
        assert.match(empty.err, /Slug kosong/);
        assert.equal(text(['slug', '--sep', 'ab', 'x']).code, 2);
        assert.equal(text(['slug', '--sep', 'a', 'x']).code, 2);
    });
});

describe('text freq', () => {
    const input = 'the cat The dog the Cat a bird';
    test('ranking, case folding, limits', () => {
        assert.equal(text(['freq'], input).out, '3  the\n2  cat\n1  a\n1  bird\n1  dog\n');
        assert.equal(text(['freq', '--top', '1'], input).out, '3  the\n');
        assert.equal(text(['freq', '--case-sensitive', '--top', '2'], input).out, '2  the\n1  Cat\n');
        assert.equal(text(['freq', '--min-length', '4'], input).out, '1  bird\n');
    });

    test('--json shape', () => {
        assert.deepEqual(JSON.parse(text(['freq', '--json', '--top', '2'], input).out), {
            total: 8,
            unique: 5,
            top: [
                { word: 'the', count: 3 },
                { word: 'cat', count: 2 },
            ],
        });
    });

    test('invalid numbers', () => {
        assert.equal(text(['freq', '--top', '0', 'x']).code, 2);
        assert.equal(text(['freq', '--min-length', 'abc', 'x']).code, 2);
    });
});

describe('text lines', () => {
    const input = '  b \na\n\nb\n a\n';
    test('processing', () => {
        assert.equal(text(['lines', '--sort'], input).out, '\n  b \n a\na\nb\n');
        assert.equal(text(['lines', '--trim', '--unique'], input).out, 'b\na\n\n');
        assert.equal(text(['lines', '--trim', '--unique', '--sort', '--reverse'], input).out, 'b\na\n\n');
        assert.equal(text(['lines', '--reverse', '--numbered'], 'x\ny\n').out, '1  y\n2  x\n');
        assert.deepEqual(JSON.parse(text(['lines', '--unique', '--json'], 'a\nb\na\n').out), {
            count: 2,
            lines: ['a', 'b'],
        });
        assert.equal(text(['lines'], '').out, '');
    });
});

describe('text errors', () => {
    test('usage errors', () => {
        assert.equal(text([]).code, 2);
        assert.equal(text(['nope', 'x']).code, 2);
        const r = text(['stats', '--sort', 'x']);
        assert.equal(r.code, 2);
        assert.match(r.err, /--sort tidak berlaku untuk "stats"/);
        assert.equal(text(['lines', '--numbered', '--json'], 'a').code, 2);
    });
});

describe('text regressions', () => {
    test('stats sentence counting is linear on long punctuation runs', () => {
        const r = spawnSync(process.execPath, [CLI, 'text', 'stats', '--json'], {
            encoding: 'utf8',
            input: '.'.repeat(200000) + 'x',
            timeout: 10000,
            env: { PATH: process.env.PATH },
        });
        assert.equal(r.error, undefined, 'punctuation-run analysis must finish before the timeout');
        assert.equal(r.status, 0);
        assert.equal(JSON.parse(r.stdout).sentences, 1);
        assert.equal(JSON.parse(text(['stats', '--json'], 'Hi... Yes! No?\n').out).sentences, 3);
    });

    test('sentence boundaries preserve punctuation, whitespace, decimals, and Unicode content', () => {
        const cases: [string, number][] = [
            ['... !? …', 0],
            ['Hi... Yes! No?\n', 3],
            ['Hello...world! Next', 2],
            ['Pi is 3.14. Yes', 2],
            ['One?!\tTwo…\u00a0Three.', 3],
            ['\u{10400}... １２３!', 2],
            ['Only words', 1],
            ['🙂... 🙂!', 0],
        ];
        for (const [input, expected] of cases) {
            const r = text(['stats', '--json'], input);
            assert.equal(r.code, 0);
            assert.equal(JSON.parse(r.out).sentences, expected, JSON.stringify(input));
        }
    });

    test('slug keeps kana dakuten and Devanagari, strips Latin accents', () => {
        assert.equal(text(['slug', 'Héllo Wörld'], '').out.trim(), 'hello-world');
        assert.equal(text(['slug', 'ガパ'], '').out.trim(), 'ガパ');
        assert.equal(text(['slug', 'カ\u3099 ハ\u309a'], '').out.trim(), 'ガ-パ');
        assert.equal(text(['slug', 'हिन्दी भाषा'], '').out.trim(), 'हिन्दी-भाषा');
        assert.equal(text(['slug', 'a\u1ab0'], '').out.trim(), 'a\u1ab0');
    });

    test('stats bytes come from the raw input', () => {
        const cases: [number[], number][] = [
            [[0xff, 0xfe], 2],
            [[0xff, 0xfe, 0x0a], 2],
            [[0xff, 0xfe, 0x0d, 0x0a], 2],
            [[0xff, 0xfe, 0x0a, 0x0a], 3],
            [[0xff, 0xfe, 0x0d], 3],
        ];
        for (const [input, expected] of cases) {
            const r = spawnSync(process.execPath, [CLI, 'text', 'stats', '--json'], {
                encoding: 'utf8',
                input: Buffer.from(input),
                env: { PATH: process.env.PATH },
            });
            assert.equal(r.status, 0);
            assert.equal(JSON.parse(r.stdout).bytes, expected);
        }
        assert.equal(JSON.parse(text(['stats', '--json'], 'ab\n').out).bytes, 2);
        assert.equal(JSON.parse(text(['stats', '--json'], 'é\r\n').out).bytes, 2);
    });

    test('lines --sort orders by code point', () => {
        const lines = [
            '\u{1f600}',
            '\uffff',
            '\uff21',
            'z',
            '\u{10000}',
            '\u{10ffff}',
            'ab',
            'a',
            'a\u{1f600}',
            'a\uffff',
        ];
        const r = text(['lines', '--sort', '--json'], lines.join('\n') + '\n');
        assert.equal(r.code, 0);
        assert.deepEqual(JSON.parse(r.out).lines, [
            'a',
            'ab',
            'a\uffff',
            'a\u{1f600}',
            'z',
            '\uff21',
            '\uffff',
            '\u{10000}',
            '\u{1f600}',
            '\u{10ffff}',
        ]);
    });

    test('code point ordering keeps unpaired surrogates in their BMP positions', () => {
        assert.deepEqual(
            processLines('\u{10000}\n\udc00\n\ue000\n\ud800\n', {
                trim: false,
                unique: false,
                sort: true,
                reverse: false,
            }),
            ['\ud800', '\udc00', '\ue000', '\u{10000}'],
        );
    });
});
