import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { describe, test } from 'node:test';
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

const sv = (...args: string[]) => cli(['semver', ...args]);
const satisfies = (version: string, range: string) => sv('satisfies', version, range).code === 0;

const CHAIN = [
    '1.0.0-alpha',
    '1.0.0-alpha.1',
    '1.0.0-alpha.beta',
    '1.0.0-beta',
    '1.0.0-beta.2',
    '1.0.0-beta.11',
    '1.0.0-rc.1',
    '1.0.0',
];

describe('semver parse / valid', () => {
    test('parse splits all parts and strips one leading v', () => {
        const r = sv('parse', '--json', 'v1.2.3-alpha.1.x-y+build.5');
        assert.equal(r.code, 0);
        assert.deepEqual(JSON.parse(r.out), {
            input: 'v1.2.3-alpha.1.x-y+build.5',
            version: '1.2.3-alpha.1.x-y+build.5',
            major: 1,
            minor: 2,
            patch: 3,
            prerelease: ['alpha', 1, 'x-y'],
            build: ['build', '5'],
        });
        assert.match(sv('parse', '1.2.3').out, /^major:\s+1\nminor:\s+2\npatch:\s+3\nprerelease: -\nbuild:\s+-\n$/);
    });

    test('strict grammar: valid exits 0, invalid exits 1', () => {
        for (const ok of ['0.0.0', '1.2.3', 'v1.2.3', '1.0.0-0', '1.0.0-0A.is.legal', '1.0.0+001', '1.0.0-a+b']) {
            const r = sv('valid', ok);
            assert.deepEqual([r.code, r.out.trim()], [0, 'valid'], ok);
        }
        for (const bad of [
            '',
            '1',
            '1.2',
            '01.2.3',
            '1.02.3',
            '1.2.03',
            '1.2.3-01',
            '1.2.3-',
            '1.2.3+',
            '1.2.3-a..b',
            '1.2.3-a_b',
            'vv1.2.3',
            '=1.2.3',
            ' 1.2.3',
            '1.2.3.4',
            '99999999999999999999.0.0',
        ]) {
            const r = sv('valid', bad);
            assert.deepEqual([r.code, r.out.trim()], [1, 'invalid'], JSON.stringify(bad));
        }
    });

    test('valid --json reports the error and keeps exit 1', () => {
        const ok = sv('valid', '--json', 'v1.0.0+x');
        assert.deepEqual(JSON.parse(ok.out), { input: 'v1.0.0+x', valid: true, version: '1.0.0+x', error: null });
        const bad = sv('valid', '--json', '1.0');
        assert.equal(bad.code, 1);
        const doc = JSON.parse(bad.out);
        assert.deepEqual([doc.valid, doc.version], [false, null]);
        assert.equal(typeof doc.error, 'string');
    });

    test('invalid version in other commands is a usage error', () => {
        for (const args of [
            ['parse', '1.0'],
            ['compare', '1.0.0', 'x'],
            ['sort', '1.0.0', '01.0.0'],
            ['bump', 'patch', '1.0'],
            ['satisfies', '1.x', '*'],
        ]) {
            const r = sv(...args);
            assert.equal(r.code, 2, args.join(' '));
            assert.equal(r.out, '');
            assert.match(r.err, /tidak valid/);
        }
    });
});

describe('semver compare / sort', () => {
    test('precedence chain from the SemVer spec', () => {
        for (let i = 0; i < CHAIN.length; i++) {
            for (let j = 0; j < CHAIN.length; j++) {
                const r = sv('compare', CHAIN[i]!, CHAIN[j]!);
                assert.equal(r.code, 0);
                assert.equal(r.out.trim(), String(Math.sign(i - j)), `${CHAIN[i]} vs ${CHAIN[j]}`);
            }
        }
    });

    test('core precedence, build ignored, numeric ids below alphanumeric', () => {
        assert.equal(sv('compare', '1.0.0', '2.0.0').out.trim(), '-1');
        assert.equal(sv('compare', '2.0.0', '2.1.0').out.trim(), '-1');
        assert.equal(sv('compare', '2.1.0', '2.1.1').out.trim(), '-1');
        assert.equal(sv('compare', '1.0.0+a', '1.0.0+b').out.trim(), '0');
        assert.equal(sv('compare', 'v1.0.0', '1.0.0').out.trim(), '0');
        assert.equal(sv('compare', '1.0.0-1', '1.0.0-a').out.trim(), '-1');
        assert.equal(sv('compare', '1.0.0-2', '1.0.0-10').out.trim(), '-1');
        // beyond Number.MAX_SAFE_INTEGER, still compared numerically
        assert.equal(sv('compare', '1.0.0-9007199254740993', '1.0.0-9007199254740992').out.trim(), '1');
        assert.equal(sv('compare', '1.0.0-a', '1.0.0-a.0').out.trim(), '-1');
        assert.equal(sv('compare', '1.0.0-B', '1.0.0-a').out.trim(), '-1');
    });

    test('compare --json', () => {
        assert.deepEqual(JSON.parse(sv('compare', '--json', '1.0.0', '1.0.0-rc.1').out), {
            a: '1.0.0',
            b: '1.0.0-rc.1',
            result: 1,
        });
    });

    test('sort ascending / --reverse, stable for build-only ties', () => {
        const shuffled = [CHAIN[5]!, CHAIN[7]!, CHAIN[0]!, CHAIN[3]!, CHAIN[1]!, CHAIN[6]!, CHAIN[2]!, CHAIN[4]!];
        assert.deepEqual(
            sv('sort', ...shuffled)
                .out.trim()
                .split('\n'),
            CHAIN,
        );
        assert.deepEqual(
            sv('sort', '--reverse', ...shuffled)
                .out.trim()
                .split('\n'),
            [...CHAIN].reverse(),
        );
        assert.deepEqual(sv('sort', 'v2.0.0', '1.0.0+b', '1.0.0+a').out.trim().split('\n'), [
            '1.0.0+b',
            '1.0.0+a',
            '2.0.0',
        ]);
        assert.deepEqual(JSON.parse(sv('sort', '--json', '--reverse', '1.0.0', '1.1.0').out), {
            reverse: true,
            versions: ['1.1.0', '1.0.0'],
        });
    });
});

describe('semver bump', () => {
    const bump = (release: string, version: string, ...extra: string[]) =>
        sv('bump', ...extra, release, version).out.trim();

    test('plain releases follow npm', () => {
        assert.equal(bump('major', '1.2.3'), '2.0.0');
        assert.equal(bump('minor', '1.2.3'), '1.3.0');
        assert.equal(bump('patch', '1.2.3'), '1.2.4');
        assert.equal(bump('patch', 'v1.2.3+meta'), '1.2.4');
        // releasing a prerelease drops the tag without skipping a number
        assert.equal(bump('patch', '1.2.3-beta.1'), '1.2.3');
        assert.equal(bump('minor', '1.3.0-beta.1'), '1.3.0');
        assert.equal(bump('major', '2.0.0-rc.1'), '2.0.0');
        assert.equal(bump('minor', '1.2.3-beta.1'), '1.3.0');
        assert.equal(bump('major', '1.2.3-beta.1'), '2.0.0');
    });

    test('pre* releases and --preid', () => {
        assert.equal(bump('premajor', '1.2.3'), '2.0.0-0');
        assert.equal(bump('preminor', '1.2.3'), '1.3.0-0');
        assert.equal(bump('prepatch', '1.2.3'), '1.2.4-0');
        assert.equal(bump('premajor', '1.2.3', '--preid', 'beta'), '2.0.0-beta.0');
        assert.equal(bump('preminor', '1.2.3', '--preid', 'beta'), '1.3.0-beta.0');
        assert.equal(bump('prepatch', '1.2.3', '--preid', 'beta'), '1.2.4-beta.0');
        assert.equal(bump('prepatch', '1.2.3-beta.4', '--preid', 'beta'), '1.2.4-beta.0');
    });

    test('prerelease bumps patch first, then increments or appends', () => {
        assert.equal(bump('prerelease', '1.2.3'), '1.2.4-0');
        assert.equal(bump('prerelease', '1.2.3', '--preid', 'rc'), '1.2.4-rc.0');
        assert.equal(bump('prerelease', '1.2.3-0'), '1.2.3-1');
        assert.equal(bump('prerelease', '1.2.3-beta.1'), '1.2.3-beta.2');
        assert.equal(bump('prerelease', '1.2.3-beta.9+x'), '1.2.3-beta.10');
        assert.equal(bump('prerelease', '1.2.3-beta'), '1.2.3-beta.0');
        assert.equal(bump('prerelease', '1.2.3-a.1.b'), '1.2.3-a.2.b');
        assert.equal(bump('prerelease', '1.2.3-1.b.2'), '1.2.3-1.b.3');
        assert.equal(bump('prerelease', '1.2.3-beta.1', '--preid', 'beta'), '1.2.3-beta.2');
        assert.equal(bump('prerelease', '1.2.3-beta.1', '--preid', 'rc'), '1.2.3-rc.0');
        assert.equal(bump('prerelease', '1.2.3-beta', '--preid', 'beta'), '1.2.3-beta.0');
        assert.equal(bump('prerelease', '1.2.3-9007199254740993'), '1.2.3-9007199254740994');
    });

    test('--json shape', () => {
        assert.deepEqual(JSON.parse(sv('bump', '--json', '--preid', 'beta', 'preminor', 'v1.2.3').out), {
            release: 'preminor',
            preid: 'beta',
            from: '1.2.3',
            version: '1.3.0-beta.0',
        });
        assert.equal(JSON.parse(sv('bump', '--json', 'patch', '1.0.0').out).preid, null);
    });

    test('usage errors', () => {
        assert.equal(sv('bump', 'huge', '1.0.0').code, 2);
        assert.equal(sv('bump', 'patch').code, 2);
        assert.equal(sv('bump', '--preid', 'beta', 'patch', '1.0.0').code, 2);
        for (const preid of ['1', 'a.b', '01', 'a b', '']) {
            assert.equal(sv('bump', '--preid', preid, 'prepatch', '1.0.0').code, 2, preid);
        }
        assert.equal(sv('bump', 'major', '9007199254740991.0.0').code, 1);
    });
});

describe('semver satisfies', () => {
    test('comparators and exit codes', () => {
        const r = sv('satisfies', '1.2.3', '>=1.0.0');
        assert.deepEqual([r.code, r.out.trim()], [0, 'true']);
        const no = sv('satisfies', '1.2.3', '<1.0.0');
        assert.deepEqual([no.code, no.out.trim()], [1, 'false']);
        assert.ok(satisfies('1.2.3', '=1.2.3'));
        assert.ok(satisfies('1.2.3', '1.2.3'));
        assert.ok(satisfies('v1.2.3', '=v1.2.3'));
        assert.ok(satisfies('1.2.3', '<=1.2.3'));
        assert.ok(!satisfies('1.2.3', '>1.2.3'));
        assert.ok(!satisfies('1.2.3', '<1.2.3'));
        assert.ok(satisfies('1.2.3+build', '1.2.3'));
        assert.ok(satisfies('1.2.3', '>= 1.0.0 < 2.0.0'));
        assert.ok(satisfies('1.2.3', '>=1.0.0 <2.0.0'));
        assert.ok(!satisfies('2.0.0', '>=1.0.0 <2.0.0'));
    });

    test('caret', () => {
        assert.ok(satisfies('1.9.9', '^1.2.3'));
        assert.ok(!satisfies('2.0.0', '^1.2.3'));
        assert.ok(!satisfies('1.2.2', '^1.2.3'));
        assert.ok(satisfies('0.2.9', '^0.2.3'));
        assert.ok(!satisfies('0.3.0', '^0.2.3'));
        assert.ok(satisfies('0.0.3', '^0.0.3'));
        assert.ok(!satisfies('0.0.4', '^0.0.3'));
        assert.ok(satisfies('1.9.0', '^1.2'));
        assert.ok(satisfies('0.0.9', '^0.0'));
        assert.ok(!satisfies('0.1.0', '^0.0'));
        assert.ok(satisfies('0.9.0', '^0'));
        assert.ok(!satisfies('1.0.0', '^0'));
        assert.ok(satisfies('1.2.3', '^1.x'));
        assert.ok(satisfies('7.0.0', '^*'));
    });

    test('tilde', () => {
        assert.ok(satisfies('1.2.9', '~1.2.3'));
        assert.ok(!satisfies('1.3.0', '~1.2.3'));
        assert.ok(satisfies('1.2.9', '~1.2'));
        assert.ok(!satisfies('1.3.0', '~1.2'));
        assert.ok(satisfies('1.9.0', '~1'));
        assert.ok(!satisfies('2.0.0', '~1'));
        assert.ok(satisfies('1.2.9', '~>1.2.3'));
        assert.ok(satisfies('0.2.3', '~0'));
    });

    test('x-ranges and partials', () => {
        assert.ok(satisfies('1.2.9', '1.2.x'));
        assert.ok(satisfies('1.2.0', '1.2.X'));
        assert.ok(!satisfies('1.3.0', '1.2.x'));
        assert.ok(satisfies('1.9.9', '1.*'));
        assert.ok(!satisfies('2.0.0', '1.*'));
        assert.ok(satisfies('1.9.9', '1'));
        assert.ok(satisfies('1.2.9', '1.2'));
        assert.ok(satisfies('5.0.0', '*'));
        assert.ok(satisfies('5.0.0', ''));
        assert.ok(satisfies('5.0.0', 'x'));
        assert.ok(satisfies('1.3.0', '>1.2'));
        assert.ok(!satisfies('1.2.9', '>1.2'));
        assert.ok(satisfies('2.0.0', '>1'));
        assert.ok(!satisfies('1.9.9', '>1'));
        assert.ok(satisfies('1.2.0', '>=1.2'));
        assert.ok(!satisfies('1.1.9', '>=1.2'));
        assert.ok(satisfies('1.2.9', '<=1.2'));
        assert.ok(!satisfies('1.3.0', '<=1.2'));
        assert.ok(satisfies('1.1.9', '<1.2'));
        assert.ok(!satisfies('1.2.0', '<1.2'));
        assert.ok(!satisfies('0.0.0', '<*'));
        assert.ok(!satisfies('9.0.0', '>*'));
        assert.ok(satisfies('9.0.0', '>=*'));
    });

    test('hyphen ranges, AND, OR', () => {
        assert.ok(satisfies('1.2.3', '1.2.3 - 2.3.4'));
        assert.ok(satisfies('2.3.4', '1.2.3 - 2.3.4'));
        assert.ok(!satisfies('2.3.5', '1.2.3 - 2.3.4'));
        assert.ok(satisfies('1.2.0', '1.2 - 2.3.4'));
        assert.ok(!satisfies('1.1.9', '1.2 - 2.3.4'));
        assert.ok(satisfies('2.3.9', '1.2.3 - 2.3'));
        assert.ok(!satisfies('2.4.0', '1.2.3 - 2.3'));
        assert.ok(satisfies('2.9.9', '1.2.3 - 2'));
        assert.ok(!satisfies('3.0.0', '1.2.3 - 2'));
        assert.ok(satisfies('1.5.0', '1.2.3 - 2.3.4 || >=5'));
        assert.ok(satisfies('6.0.0', '1.2.3 - 2.3.4 || >=5'));
        assert.ok(!satisfies('3.0.0', '1.2.3 - 2.3.4 || >=5'));
        assert.ok(satisfies('1.0.0', '<1.0.0 || 1.0.0'));
        assert.ok(satisfies('3.0.0', '^1.2 || >=3'));
        assert.ok(satisfies('1.0.0', '1.0.0 ||'));
    });

    test('prerelease only matches sets that opt in on the same tuple', () => {
        assert.ok(!satisfies('1.2.3-beta.2', '^1.2.0'));
        assert.ok(!satisfies('1.2.3-beta.2', '*'));
        assert.ok(!satisfies('1.2.3-beta.2', '>=1.0.0'));
        assert.ok(satisfies('1.2.3-beta.2', '>=1.2.3-beta.1'));
        assert.ok(satisfies('1.2.3-beta.2', '^1.2.3-beta.1'));
        assert.ok(!satisfies('1.2.4-beta.2', '^1.2.3-beta.1'));
        assert.ok(satisfies('1.2.3', '^1.2.3-beta.1'));
        assert.ok(!satisfies('1.2.3-beta.0', '^1.2.3-beta.1'));
        assert.ok(satisfies('1.2.3-beta.2', '1.2.3-beta.2'));
        assert.ok(satisfies('1.2.3-beta.2', '~1.2.3-beta.1'));
        assert.ok(!satisfies('2.0.0-alpha', '<2.0.0'));
        assert.ok(!satisfies('2.0.0-alpha', '^1.0.0'));
        // opt-in is per comparator set
        assert.ok(satisfies('1.2.3-beta.2', '>=1.2.3-beta.1 || ^2.0.0'));
        assert.ok(!satisfies('1.2.3-beta.2', '^2.0.0 || ^1.0.0'));
        assert.ok(satisfies('1.2.3-beta.2', '>1.0.0 <1.2.3-rc'));
    });

    test('invalid ranges are usage errors', () => {
        for (const range of ['abc', '1.x.3', '>>1.0.0', '01.2.3', '1.2.3 -', '>=', '1.2.3-', '1.x-beta', '^1.2.3-01']) {
            const r = sv('satisfies', '1.0.0', range);
            assert.equal(r.code, 2, range);
            assert.match(r.err, /ange|Operator/);
        }
    });

    test('--json shape', () => {
        assert.deepEqual(JSON.parse(sv('satisfies', '--json', 'v1.4.0', '^1.2').out), {
            version: '1.4.0',
            range: '^1.2',
            satisfies: true,
        });
        const no = sv('satisfies', '--json', '2.0.0', '^1.2');
        assert.equal(no.code, 1);
        assert.equal(JSON.parse(no.out).satisfies, false);
    });
});

describe('semver usage', () => {
    test('command and option validation', () => {
        assert.equal(sv().code, 2);
        assert.equal(sv('frobnicate').code, 2);
        assert.equal(sv('compare', '1.0.0').code, 2);
        assert.equal(sv('valid').code, 2);
        assert.equal(sv('sort').code, 2);
        assert.equal(sv('parse', '--reverse', '1.0.0').code, 2);
        assert.equal(sv('compare', '--bogus', '1.0.0', '1.0.0').code, 2);
    });

    test('alias sv works and --json leaves only one document on stdout', () => {
        const r = cli(['sv', 'compare', '--json', '1.0.0', '2.0.0']);
        assert.equal(r.code, 0);
        assert.equal(r.err, '');
        assert.deepEqual(JSON.parse(r.out), { a: '1.0.0', b: '2.0.0', result: -1 });
    });
});

describe('semver strictness and overflow', () => {
    test('range build metadata with empty identifiers is rejected', () => {
        for (const range of ['1.2.3+..a', '1.2.3+a.', '1.2.3+a..b', '1.2.3+']) {
            const r = sv('satisfies', '1.2.3', range);
            assert.equal(r.code, 2, range);
            assert.match(r.err, /Range tidak valid/);
        }
        for (const range of ['1.2.3+--', '1.2.3+build.01-a']) {
            const r = sv('satisfies', '1.2.3', range);
            assert.deepEqual([r.code, r.out.trim()], [0, 'true'], range);
        }
    });

    test('bump prerelease handles numeric identifiers above 2^53', () => {
        const r = sv('bump', 'prerelease', '1.0.0-beta.9007199254740993');
        assert.equal(r.code, 0, r.err);
        assert.equal(r.out.trim(), '1.0.0-beta.9007199254740994');
    });

    test('help Exit line includes bump overflow', () => {
        const r = sv('--help');
        assert.equal(r.code, 0, r.err);
        assert.match(r.out, /^Exit: .*1 .*bump melebihi bilangan bulat aman/m);
    });
});
