import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { describe, test } from 'node:test';
import { fileURLToPath } from 'node:url';

const CLI = fileURLToPath(new URL('../src/cli.js', import.meta.url));

/** run the CLI without a terminal; `input` is piped to stdin */
const cli = (args: string[], input = '') => {
    const r = spawnSync(process.execPath, [CLI, ...args], {
        encoding: 'utf8',
        input,
        env: { PATH: process.env.PATH },
    });
    return { code: r.status, out: r.stdout, err: r.stderr };
};

/** evaluates one expression and returns the trimmed stdout, asserting success */
const calc = (expression: string, ...options: string[]) => {
    const r = cli(['math', ...options, '--', expression]);
    assert.equal(r.code, 0, `${expression}: ${r.err}`);
    return r.out.trim();
};

describe('math: evaluation', () => {
    test('operator precedence and associativity', () => {
        assert.equal(calc('2 + 3 * 4'), '14');
        assert.equal(calc('(2 + 3) * 4'), '20');
        assert.equal(calc('10 - 4 - 3'), '3');
        assert.equal(calc('100 / 10 / 5'), '2');
        assert.equal(calc('2 ^ 3 ^ 2'), '512');
        assert.equal(calc('2 ** 3 ** 2'), '512');
        assert.equal(calc('-2^2'), '-4');
        assert.equal(calc('(-2)^2'), '4');
        assert.equal(calc('2^-1'), '0.5');
        assert.equal(calc('2^-3^2'), String(2 ** -9));
        assert.equal(calc('2 * 3 ^ 2'), '18');
        assert.equal(calc('--3'), '3');
        assert.equal(calc('7 % 4 * 2'), '6');
        assert.equal(calc('-7 % 3'), '-1');
    });

    test('number literals', () => {
        assert.equal(calc('1e-3'), '0.001');
        assert.equal(calc('.5 + 1.'), '1.5');
        assert.equal(calc('0xff'), '255');
        assert.equal(calc('0XFF_FF'), '65535');
        assert.equal(calc('1_000_000 + 1'), '1000001');
        assert.equal(calc('1.5E3'), '1500');
    });

    test('factorial', () => {
        assert.equal(calc('0!'), '1');
        assert.equal(calc('5!'), '120');
        assert.equal(calc('3!!'), '720');
        assert.equal(calc('2^3!'), '64');
        assert.equal(calc('-3!'), '-6');
        assert.equal(calc('20!'), '2432902008176640000');
        assert.equal(calc('170!'), '7.25741561531e+306');
        for (const bad of ['171!', '(-1)!', '2.5!']) {
            const r = cli(['math', '--', bad]);
            assert.equal(r.code, 1, bad);
            assert.match(r.err, /Faktorial/);
        }
    });

    test('constants and functions', () => {
        assert.equal(calc('pi'), '3.14159265359');
        assert.equal(calc('E'), '2.71828182846');
        assert.equal(calc('tau / 2 - pi'), '0');
        assert.equal(calc('inf'), 'inf');
        assert.equal(calc('-inf'), '-inf');
        assert.equal(calc('inf + 1'), 'inf');
        assert.equal(calc('sqrt(16) + cbrt(27)'), '7');
        assert.equal(calc('abs(-3) + sign(-9)'), '2');
        assert.equal(calc('round(2.5) + round(-2.5)'), '0');
        assert.equal(calc('floor(-1.5)'), '-2');
        assert.equal(calc('ceil(1.2)'), '2');
        assert.equal(calc('trunc(-1.9)'), '-1');
        assert.equal(calc('ln(e)'), '1');
        assert.equal(calc('log(1000) + log2(8) + log10(100)'), '8');
        assert.equal(calc('exp(0)'), '1');
        assert.equal(calc('min(3, 1, 2) + max(1, 5)'), '6');
        assert.equal(calc('pow(2, 10)'), '1024');
        assert.equal(calc('hypot(3, 4)'), '5');
        assert.equal(calc('mod(10, 4)'), '2');
        assert.equal(calc('gcd(12, 18) + lcm(4, 6)'), '18');
        assert.equal(calc('SQRT(4)'), '2');
        assert.equal(calc('deg(pi)'), '180');
        assert.equal(calc('rad(180) - pi'), '0');
        assert.equal(calc('sin(pi / 2)'), '1');
        assert.equal(calc('cos(0)'), '1');
        assert.equal(calc('atan2(1, 1) * 4'), '3.14159265359');
        assert.equal(calc('tanh(0) + sinh(0)'), '0');
    });

    test('--degrees', () => {
        assert.equal(calc('sin(30)', '--degrees'), '0.5');
        assert.equal(calc('sin(180)', '--degrees'), '0');
        assert.equal(calc('cos(90)', '--degrees'), '0');
        assert.equal(calc('cos(-360)', '--degrees'), '1');
        assert.equal(calc('tan(45)', '--degrees'), '1');
        assert.equal(calc('asin(1)', '--degrees'), '90');
        assert.equal(calc('acos(0.5)', '--degrees'), '60');
        assert.equal(calc('atan(1)', '--degrees'), '45');
        assert.equal(calc('atan2(1, -1)', '--degrees'), '135');
        const r = cli(['math', '--degrees', 'tan(90)']);
        assert.equal(r.code, 1);
        assert.match(r.err, /tan tidak terdefinisi/);
    });

    test('float noise is hidden; integers stay exact', () => {
        assert.equal(calc('0.1 + 0.2'), '0.3');
        assert.equal(calc('sqrt(2)^2'), '2');
        assert.equal(calc('2^64'), '18446744073709551616');
        assert.equal(calc('2^53 + 1'), '9007199254740992');
        assert.equal(calc('10^21'), '1e+21');
        assert.equal(calc('10^25'), '1e+25');
        assert.equal(calc('1/3'), '0.333333333333');
        assert.equal(calc('-0'), '0');
    });
});

describe('math: formatting', () => {
    test('--precision', () => {
        assert.equal(calc('1/3', '-p', '4'), '0.3333');
        assert.equal(calc('2', '--precision', '3'), '2.000');
        assert.equal(calc('2.5', '-p', '0'), '3');
        assert.equal(calc('-0.0001', '-p', '2'), '0.00');
        assert.equal(calc('pi', '-p', '15'), '3.141592653589793');
        for (const bad of ['16', '-1', 'x', '1.5']) {
            const r = cli(['math', '-p', bad, '1']);
            assert.equal(r.code, 2, bad);
        }
    });

    test('--scientific', () => {
        assert.equal(calc('12345.678', '--scientific'), '1.2345678e+4');
        assert.equal(calc('0.1 + 0.2', '--scientific'), '3e-1');
        assert.equal(calc('1500', '--scientific', '-p', '2'), '1.50e+3');
        assert.equal(calc('0', '--scientific'), '0e+0');
    });
});

describe('math: input', () => {
    test('arguments are joined with spaces', () => {
        const r = cli(['math', '1', '+', '2', '*', '3']);
        assert.equal(r.out, '7\n');
    });

    test('multiple lines from stdin with ans', () => {
        const r = cli(['math'], '1 + 1\r\n\n ans * 10\nans ^ 2\n');
        assert.equal(r.code, 0, r.err);
        assert.equal(r.out, '2\n20\n400\n');
    });

    test('ans without a previous result fails', () => {
        const r = cli(['math', 'ans + 1']);
        assert.equal(r.code, 1);
        assert.match(r.err, /'ans' belum ada/);
    });

    test('empty input is a usage error', () => {
        assert.equal(cli(['math'], '\n  \n').code, 2);
    });

    test('error on a later line reports its number; earlier results are printed', () => {
        const r = cli(['math'], '1+1\n2+\n');
        assert.equal(r.code, 2);
        assert.equal(r.out, '2\n');
        assert.match(r.err, /baris 2: .*kolom 3/);
    });
});

describe('math: errors', () => {
    test('syntax errors are exit 2 and name the column', () => {
        const cases: [string, RegExp][] = [
            ['1 + * 2', /kolom 5/],
            ['(1 + 2', /diharapkan '\)' \(kolom 7\)/],
            ['1 + 2)', /kolom 6/],
            ['1 $ 2', /Karakter tidak dikenal: "\$" \(kolom 3\)/],
            ['2 +', /berakhir.*\(kolom 4\)/],
            ['sqrt 4', /'\(' diharapkan setelah fungsi sqrt \(kolom 6\)/],
            ['sqrt()', /membutuhkan 1 argumen, diberikan 0/],
            ['atan2(1)', /membutuhkan 2 argumen, diberikan 1/],
            ['max()', /minimal 1 argumen/],
            ['pi(2)', /Perkalian implisit.*kolom 3/],
            ['2pi', /Perkalian implisit.*kolom 2/],
            ['2(3)', /Perkalian implisit.*kolom 2/],
            ['(1)(2)', /Perkalian implisit.*kolom 4/],
            ['1 2', /Perkalian implisit.*kolom 3/],
            ['0x', /heksadesimal.*kolom 3/],
            ['1__0', /Pemisah '_'/],
            ['1_', /Pemisah '_'.*kolom 2/],
            ['.', /Titik harus diikuti digit/],
            ['1e999', /Angka terlalu besar/],
            ['1,2', /Token tidak terduga ','/],
            ['()', /Token tidak terduga '\)'/],
        ];
        for (const [expression, pattern] of cases) {
            const r = cli(['math', '--', expression]);
            assert.equal(r.code, 2, `${expression}: ${r.err}`);
            assert.match(r.err, pattern, expression);
            assert.equal(r.out, '');
        }
    });

    test('domain errors are exit 1', () => {
        const cases: [string, RegExp][] = [
            ['1 / 0', /Pembagian dengan nol \(kolom 3\)/],
            ['0 / 0', /Pembagian dengan nol/],
            ['5 % 0', /Modulo dengan nol/],
            ['mod(5, 0)', /Modulo dengan nol/],
            ['0 ^ -1', /nol pangkat negatif/],
            ['sqrt(-1)', /sqrt dari bilangan negatif \(kolom 1\)/],
            ['ln(0)', /ln hanya terdefinisi/],
            ['log(-5)', /log hanya terdefinisi/],
            ['asin(2)', /asin hanya terdefinisi/],
            ['(-8)^0.5', /NaN/],
            ['inf - inf', /NaN/],
            ['10^400', /tak berhingga/],
            ['exp(1000)', /tak berhingga/],
            ['2^1024', /tak berhingga/],
            ['gcd(1.5, 2)', /bilangan bulat/],
            ['lcm(9007199254740991, 2)', /melebihi/],
        ];
        for (const [expression, pattern] of cases) {
            const r = cli(['math', '--', expression]);
            assert.equal(r.code, 1, `${expression}: ${r.err}`);
            assert.match(r.err, pattern, expression);
            assert.equal(r.out, '');
        }
    });

    test('injection attempts fail as unknown identifiers', () => {
        for (const [expression, name] of [
            ['process.exit()', 'process'],
            ['constructor', 'constructor'],
            ['__proto__', '__proto__'],
            ['toString(1)', 'toString'],
            ['Math.max(1)', 'Math'],
            ['globalThis', 'globalThis'],
            ['require("fs")', 'require'],
            ['this', 'this'],
        ] as const) {
            const r = cli(['math', '--', expression]);
            assert.equal(r.code, 2, expression);
            assert.match(r.err, new RegExp(`Identifier tidak dikenal: ${name} \\(kolom 1\\)`));
        }
        assert.equal(cli(['math', '1 + `x`']).code, 2);
        assert.equal(cli(['math', '[1]']).code, 2);
        assert.equal(cli(['math', '{}']).code, 2);
    });

    test('length and nesting limits', () => {
        const nest = (n: number) => '('.repeat(n) + '1' + ')'.repeat(n);
        assert.equal(calc(nest(199)), '1');
        const deep = cli(['math', nest(201)]);
        assert.equal(deep.code, 2);
        assert.match(deep.err, /terlalu dalam/);

        assert.equal(calc('-'.repeat(199) + '1'), '-1');
        assert.equal(cli(['math', '--', '-'.repeat(300) + '1']).code, 2);
        assert.equal(cli(['math', '2' + '^2'.repeat(300)]).code, 2);
        assert.equal(cli(['math', 'abs('.repeat(300) + '1' + ')'.repeat(300)]).code, 2);

        const long = '1' + '+1'.repeat(4999); // exactly 10000 characters
        assert.equal(calc(long), '5000');
        const tooLong = cli(['math', long + '+1']);
        assert.equal(tooLong.code, 2);
        assert.match(tooLong.err, /terlalu panjang/);
    });

    test('unknown option and --json conflicts are usage errors', () => {
        assert.equal(cli(['math', '--bogus', '1']).code, 2);
        assert.equal(cli(['math', '--json', '--scientific', '1']).code, 2);
        assert.equal(cli(['math', '--json', '-p', '2', '1']).code, 2);
    });
});

describe('math: --json', () => {
    test('shape', () => {
        const r = cli(['math', '--json'], '0.1+0.2\ninf\n-inf\n2^64\nans/2\n');
        assert.equal(r.code, 0, r.err);
        assert.deepEqual(JSON.parse(r.out), {
            results: [
                { expression: '0.1+0.2', result: 0.3 },
                { expression: 'inf', result: 'inf' },
                { expression: '-inf', result: '-inf' },
                // JSON numbers are doubles: text prints 18446744073709551616, JSON the shortest double repr.
                { expression: '2^64', result: 18446744073709552000 },
                { expression: 'ans/2', result: 9223372036854776000 },
            ],
        });
    });

    test('errors leave stdout empty', () => {
        const r = cli(['math', '--json'], '1+1\n1/0\n');
        assert.equal(r.code, 1);
        assert.equal(r.out, '');
        assert.match(r.err, /baris 2: Pembagian dengan nol/);
    });
});

describe('math: review fixes', () => {
    test('--degrees snaps exactly at multiples of 30 and tan 45', () => {
        assert.equal(calc('floor(tan(45))', '--degrees'), '1');
        assert.equal(calc('floor(sin(30)*2)', '--degrees'), '1');
        assert.equal(calc('cos(60)', '--degrees'), '0.5');
        assert.equal(calc('floor(cos(60)*2)', '--degrees'), '1');
        assert.equal(calc('sin(150)', '--degrees'), '0.5');
        assert.equal(calc('sin(210) * 2', '--degrees'), '-1');
        assert.equal(calc('tan(135)', '--degrees'), '-1');
        assert.equal(calc('tan(225)', '--degrees'), '1');
        assert.equal(calc('tan(-135)', '--degrees'), '1');
        assert.equal(calc('tan(-45)', '--degrees'), '-1');
        assert.equal(calc('floor(sin(390)*2)', '--degrees'), '1');
        assert.equal(calc('floor(cos(-60)*2)', '--degrees'), '1');
        assert.equal(cli(['math', '--degrees', 'tan(270)']).code, 1);
    });

    test('-p stays fixed-decimal for integers >= 1e21', () => {
        assert.equal(calc('1e21', '-p', '2'), '1000000000000000000000.00');
        assert.equal(calc('1e21', '-p', '0'), '1000000000000000000000');
        assert.equal(calc('-1e21', '-p', '2'), '-1000000000000000000000.00');
        assert.equal(calc('2^70', '-p', '2'), '1180591620717411303424.00');
        assert.equal(calc('1e21', '-p', '15'), '1000000000000000000000.000000000000000');
        assert.equal(calc('1e21', '--scientific', '-p', '2'), '1.00e+21');
    });

    test('syntax errors win over earlier domain errors', () => {
        assert.equal(cli(['math', '1/0 + (']).code, 2);
        assert.equal(cli(['math', 'ans + (']).code, 2);
        assert.equal(cli(['math', 'sqrt(-1) + unknown']).code, 2);
        assert.equal(cli(['math', '(-1)! + 1e5000']).code, 2);
        assert.equal(cli(['math', '1/0 + min()']).code, 2);
        assert.equal(cli(['math', '1/0 + 1']).code, 1);
        assert.equal(cli(['math', '1/0']).code, 1);
    });

    test('literal overflow is a usage error; underflow is 0', () => {
        assert.equal(cli(['math', '1e5000']).code, 2);
        assert.equal(calc('1e-5000'), '0');
        assert.equal(calc('1e20/3'), '33333333333333331968');
        assert.equal(calc('0x20000000000001'), '9007199254740992');
    });

    test('a multi-line argument is split into expressions', () => {
        const r = cli(['math', '1\n2']);
        assert.equal(r.code, 0, r.err);
        assert.equal(r.out, '1\n2\n');
    });

    test('caret stays aligned with tabs; control characters are escaped', () => {
        const r = cli(['math', '1\t+\t*']);
        assert.equal(r.code, 2);
        const lines = r.err.split('\n');
        const echo = lines.find((l) => l.includes('1\\t+\\t*'))!;
        assert.ok(echo, r.err);
        const caret = lines[lines.indexOf(echo) + 1]!;
        assert.equal(caret.indexOf('^'), echo.indexOf('*'));
        assert.ok(!r.err.includes('\t'));
        const control = cli(['math', '1+\u0001']);
        assert.equal(control.code, 2);
        assert.ok(control.err.includes('1+\\u{1}'), control.err);
        assert.ok(!control.err.includes('\u0001'));
        const unicode = cli(['math', '1+\u{1f600}']);
        assert.equal(unicode.code, 2);
        assert.match(unicode.err, /Karakter tidak dikenal: "😀"/);
        const unicodeLines = unicode.err.split('\n');
        const unicodeEcho = unicodeLines.findIndex((line) => line.includes('1+😀'));
        assert.ok(unicodeEcho >= 0, unicode.err);
        assert.equal(unicodeLines[unicodeEcho + 1]!.indexOf('^'), unicodeLines[unicodeEcho]!.indexOf('😀'));
    });
});
