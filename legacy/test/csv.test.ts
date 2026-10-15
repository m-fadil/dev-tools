import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { describe, test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { detectDelimiter, parseCsv } from '../src/tools/csv/parse.js';
import { displayWidth, renderTable, sanitizeCell, truncate } from '../src/tools/csv/table.js';

const CLI = fileURLToPath(new URL('../src/cli.js', import.meta.url));

/** run the CLI without a terminal; `input` is piped to stdin */
const cli = (args: string[], input = '') => {
    const r = spawnSync(process.execPath, [CLI, ...args], {
        encoding: 'utf8',
        input,
        env: { PATH: process.env.PATH },
        maxBuffer: 64 * 1024 * 1024,
    });
    return { code: r.status, out: r.stdout, err: r.stderr };
};

const json = (args: string[], input: string) => JSON.parse(cli(['csv', '--json', ...args], input).out);

describe('csv parsing', () => {
    test('quotes, escaped quotes, embedded delimiters and newlines, CRLF, BOM, trailing newline', () => {
        const input = '\ufeffname,note\r\n"Smith, J","say ""hi""\nbye"\r\nplain,\r\n';
        assert.deepEqual(json([], input), [
            { name: 'Smith, J', note: 'say "hi"\nbye' },
            { name: 'plain', note: '' },
        ]);
    });

    test('strips exactly one leading BOM, including through the CLI with detection', () => {
        assert.deepEqual(parseCsv('\ufeff\ufeffa,b\n1,2', ','), [
            { row: 1, fields: ['\ufeffa', 'b'] },
            { row: 2, fields: ['1', '2'] },
        ]);
        assert.deepEqual(json([], '\ufeff\ufeffa,b\n1,2'), [{ '\ufeffa': '1', b: '2' }]);
        assert.deepEqual(json(['--detect'], '\ufeff\ufeffa,b\n1,2'), [{ '\ufeffa': '1', b: '2' }]);
    });

    test('trailing delimiter, lone empty field, blank lines skipped', () => {
        assert.deepEqual(json(['--no-header'], 'a,\n\n,\nb,c'), [
            ['a', ''],
            ['', ''],
            ['b', 'c'],
        ]);
        assert.deepEqual(json([], 'a\n""\nx\n'), [{ a: '' }, { a: 'x' }]);
    });

    test('unterminated quote reports the starting row', () => {
        const r = cli(['csv'], 'a,b\n1,2\n3,"oops\n4,5\n');
        assert.equal(r.code, 1);
        assert.match(r.err, /tanda kutip tidak ditutup \(mulai di baris 3\)/);
    });

    test('text after a closing quote is an error with row number', () => {
        const r = cli(['csv'], 'a,b\n"x"y,2\n');
        assert.equal(r.code, 1);
        assert.match(r.err, /baris 2/);
    });

    test('delimiters by name and character; --detect', () => {
        assert.deepEqual(json(['-d', 'semicolon'], 'a;b\n1;2'), [{ a: '1', b: '2' }]);
        assert.deepEqual(json(['-d', 'tab'], 'a\tb\n1\t2'), [{ a: '1', b: '2' }]);
        assert.deepEqual(json(['-d', '|'], 'a|b\n1|2'), [{ a: '1', b: '2' }]);
        const detected = cli(['csv', '--json', '--detect'], '"x,y,z";b\n1;2\n');
        assert.deepEqual(JSON.parse(detected.out), [{ 'x,y,z': '1', b: '2' }]);
        assert.match(detected.err, /Delimiter terdeteksi: ;/);
        assert.equal(cli(['csv', '-d', 'ab'], 'a').code, 2);
        assert.equal(cli(['csv', '-d', '"'], 'a').code, 2);
        assert.equal(cli(['csv', '--detect', '-d', ','], 'a').code, 2);
    });

    test('detection defaults to comma on ties or no candidates and treats unquoted quotes literally', () => {
        for (const input of ['a;b|c', 'a;b|c;d|e', 'plain', '']) {
            assert.equal(detectDelimiter(input), ',', input);
        }
        assert.equal(detectDelimiter('a;b|c;d'), ';');
        assert.equal(detectDelimiter('5" pipe,x'), ',');
        assert.equal(detectDelimiter('5" literal;x;y'), ';');
        assert.equal(detectDelimiter('"a;""b;c";d'), ';');
        assert.equal(detectDelimiter('"a\n,b";c\nx,y,z'), ';');
        const tie = cli(['csv', '--detect', '--json'], 'a;b|c\nvalue');
        assert.equal(tie.code, 0, tie.err);
        assert.deepEqual(JSON.parse(tie.out), [{ 'a;b|c': 'value' }]);
        assert.match(tie.err, /Delimiter terdeteksi: ,/);
        assert.deepEqual(json(['--detect'], '5" pipe,x\n1,2'), [{ '5" pipe': '1', x: '2' }]);
    });
});

describe('csv output', () => {
    test('table aligns by display width and truncates long cells', () => {
        const input = 'nama,kota\n日本,Tokyo\nab,"a very long cell value that must be cut"\n';
        const r = cli(['csv', '--max-width', '10'], input);
        assert.equal(r.code, 0, r.err);
        assert.equal(r.out, ['nama | kota', '-----+-----------', '日本 | Tokyo', 'ab   | a very lo…', ''].join('\n'));
    });

    test('emoji presentation and flags use two columns without widening text presentation', () => {
        for (const cell of ['🇮🇩', '⌚', '⌛', '✅', '☀️', '♟️', '🫠', '👨‍👩‍👧‍👦', '1️⃣']) {
            assert.equal(displayWidth(cell), 2, cell);
            assert.equal(truncate(`${cell}ab`, 3), `${cell}…`, cell);
        }
        for (const cell of ['☀', '♟', '⌚︎', 'x️', 'e\u0301']) {
            assert.equal(displayWidth(cell), 1, cell);
        }
        assert.equal(displayWidth('日本'), 4);
    });

    test('table replaces every C0/C1 control, including terminal escape sequences', () => {
        for (let cp = 0; cp <= 0x9f; cp++) {
            if (cp > 0x1f && cp < 0x7f) continue;
            const expected = cp === 9 ? ' ' : cp === 10 || cp === 13 ? '↵' : '\ufffd';
            assert.equal(sanitizeCell(String.fromCodePoint(cp)), expected, String(cp));
            assert.equal(displayWidth(expected), 1);
        }
        assert.equal(sanitizeCell('a\r\nb\tc'), 'a↵b c');
        const cell = '\x1b[2J\x1b]0;owned\x07\u009b31m\u007f\u0080';
        const input = `a,b\n${cell},x`;
        const table = cli(['csv', '--max-width', '100'], input);
        assert.equal(table.code, 0, table.err);
        assert.doesNotMatch(table.out, /[\u0000-\u0009\u000b-\u001f\u007f-\u009f]/);
        assert.ok(table.out.includes(sanitizeCell(cell)));
        assert.deepEqual(json([], input), [{ a: cell, b: 'x' }]);
        assert.equal(cli(['csv', '-F', 'csv'], input).out, `${input}\n`);
        assert.equal(cli(['csv', '-F', 'tsv'], input).out, `a\tb\n${cell}\tx\n`);
        assert.doesNotMatch(renderTable([cell], [['x']], 100), /[\u0000-\u0009\u000b-\u001f\u007f-\u009f]/);
    });

    test('--no-header treats the first row as data with names c1..cn', () => {
        const r = cli(['csv', '--no-header'], 'a,b\n1,2\n');
        assert.equal(r.out, ['c1 | c2', '---+---', 'a  | b', '1  | 2', ''].join('\n'));
        assert.equal(cli(['csv', '--no-header', '-F', 'csv'], 'a,b\n1,2\n').out, 'a,b\n1,2\n');
    });

    test('tsv escapes control characters; csv re-quotes', () => {
        const input = 'a,b\n"x\ty","l1\nl2"\n';
        assert.equal(cli(['csv', '-F', 'tsv'], input).out, 'a\tb\nx\\ty\tl1\\nl2\n');
        assert.equal(cli(['csv', '-F', 'csv', '-d', 'tab'], 'a\tb\n"1,2"\t"q""r"\n').out, 'a,b\n"1,2","q""r"\n');
    });

    test('--select by name and 1-based index, --limit', () => {
        const input = 'a,b,c\n1,2,3\n4,5,6\n';
        assert.deepEqual(json(['--select', 'c,1', '--limit', '1'], input), [{ c: '3', a: '1' }]);
        const missing = cli(['csv', '--select', 'zz'], input);
        assert.equal(missing.code, 1);
        assert.match(missing.err, /Kolom tidak ditemukan: zz/);
        assert.equal(cli(['csv', '--select', '4'], input).code, 1);
        assert.equal(cli(['csv', '--limit', '0'], input).code, 2);
    });

    test('--stats counts rows, columns and empty cells', () => {
        const input = 'a,b\n1,\n,\n3,4\n';
        assert.deepEqual(json(['--stats'], input), {
            rows: 3,
            columns: 2,
            ragged: [],
            columnStats: [
                { index: 1, name: 'a', empty: 1 },
                { index: 2, name: 'b', empty: 2 },
            ],
        });
        const text = cli(['csv', '--stats'], input).out;
        assert.match(text, /Baris data: 3/);
        assert.match(text, /2 "b": 2/);
    });

    test('300000 data rows complete in stats, table and JSON formats', () => {
        const count = 300_000;
        const input = `a,b\n${'1,2\n'.repeat(count)}`;
        const stats = cli(['csv', '--stats'], input);
        assert.equal(stats.code, 0, stats.err);
        assert.match(stats.out, /Baris data: 300000/);
        assert.match(stats.out, /Kolom: 2/);
        const jsonStats = cli(['csv', '--stats', '--json'], input);
        assert.equal(jsonStats.code, 0, jsonStats.err);
        assert.equal(JSON.parse(jsonStats.out).rows, count);
        const table = cli(['csv'], input);
        assert.equal(table.code, 0, table.err);
        assert.equal(table.out.split('\n').length, count + 3);
        assert.ok(table.out.endsWith('1 | 2\n'));
        const objects = cli(['csv', '--json'], input);
        assert.equal(objects.code, 0, objects.err);
        const data = JSON.parse(objects.out);
        assert.equal(data.length, count);
        assert.deepEqual(data[0], { a: '1', b: '2' });
        assert.deepEqual(data[count - 1], { a: '1', b: '2' });
    });

    test('stats does not spread one argument per input column', () => {
        const count = 150_000;
        const input = Array.from({ length: count }, (_, i) => `h${i}`).join(',');
        const stats = cli(['csv', '--stats'], input);
        assert.equal(stats.code, 0, stats.err);
        assert.match(stats.out, /Baris data: 0/);
        assert.match(stats.out, /Kolom: 150000/);
        assert.ok(stats.out.endsWith('  150000 "h149999": 0\n'));
    });

    test('--json conflicts with another --format; duplicate headers cannot become objects', () => {
        assert.equal(cli(['csv', '--json', '-F', 'tsv'], 'a\n1').code, 2);
        assert.equal(cli(['csv', '--json', '-F', 'json'], 'a\n1').code, 0);
        const dup = cli(['csv', '--json'], 'a,a\n1,2');
        assert.equal(dup.code, 1);
        assert.match(dup.err, /Header duplikat "a" \(kolom 1 dan 2\)/);
        assert.equal(cli(['csv', '-F', 'xml'], 'a').code, 2);
    });
});

describe('csv ragged rows', () => {
    const input = 'a,b,c\n1,2\n1,2,3,4\nx,y,z\n';

    test('warns on stderr with row numbers and pads', () => {
        const r = cli(['csv', '--json'], input);
        assert.equal(r.code, 0);
        assert.match(r.err, /baris 2 punya 2 kolom, diharapkan 3/);
        assert.match(r.err, /baris 3 punya 4 kolom, diharapkan 3/);
        assert.deepEqual(JSON.parse(r.out), [
            { a: '1', b: '2', c: '', c4: '' },
            { a: '1', b: '2', c: '3', c4: '4' },
            { a: 'x', b: 'y', c: 'z', c4: '' },
        ]);
    });

    test('generated names skip real headers and other generated names', () => {
        assert.deepEqual(json([], 'a,c3\n1,2,3,4'), [{ a: '1', c3: '2', c4: '3', c5: '4' }]);
        assert.deepEqual(json([], 'a,c3,c4\n1,2,3,4,5'), [{ a: '1', c3: '2', c4: '3', c5: '4', c6: '5' }]);
    });

    test('--strict fails on the first ragged row', () => {
        const r = cli(['csv', '--strict'], input);
        assert.equal(r.code, 1);
        assert.equal(r.out, '');
        assert.match(r.err, /Baris 2 punya 2 kolom, diharapkan 3/);
    });
});
