import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { describe, test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { UsageError } from '../src/errors.js';
import { parseColor } from '../src/tools/color/color.js';

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

const fmt = (value: string, format: string) => cli(['color', '-f', format, value]).out.trim();

describe('color', () => {
    test('coral round trips across all notations', () => {
        for (const input of ['coral', 'CORAL', '#ff7f50', 'ff7f50', 'rgb(255, 127, 80)', 'rgb(255 127 80)']) {
            assert.equal(fmt(input, 'hex'), '#ff7f50', input);
            assert.equal(fmt(input, 'rgb'), 'rgb(255, 127, 80)', input);
            assert.equal(fmt(input, 'hsl'), 'hsl(16, 100%, 66%)', input);
            assert.equal(fmt(input, 'name'), 'coral', input);
        }
        assert.equal(fmt('coral', 'hwb'), 'hwb(16 31% 0%)');
    });

    test('hsl input is lossy but prints back unchanged', () => {
        assert.equal(fmt('hsl(16, 100%, 66%)', 'hsl'), 'hsl(16, 100%, 66%)');
    });

    test('hex shorthand, alpha forms, and positional joining', () => {
        assert.equal(fmt('#f80', 'hex'), '#ff8800');
        assert.equal(fmt('#f808', 'hex'), '#ff880088');
        assert.equal(fmt('#ff880080', 'rgb'), 'rgba(255, 136, 0, 0.502)');
        assert.equal(fmt('ff880080', 'hex'), '#ff880080');
        assert.equal(fmt('#ff8800ff', 'hex'), '#ff8800');
        assert.equal(cli(['color', '-f', 'hex', 'rgb(255', '0', '0)']).out.trim(), '#ff0000');
        assert.equal(cli(['color', '-f', 'hex'], 'navy\n').out.trim(), '#000080');
    });

    test('rgb, hsl and hwb syntax variants', () => {
        assert.equal(fmt('rgb(100% 0% 0% / 50%)', 'hex'), '#ff000080');
        assert.equal(fmt('rgba(0,0,255,.5)', 'hsl'), 'hsla(240, 100%, 50%, 0.5)');
        assert.equal(fmt('rgb(300 -5 127.5)', 'hex'), '#ff0080');
        assert.equal(fmt('hsl(120deg 100% 25%)', 'hex'), '#008000');
        assert.equal(fmt('hsl(0.5turn 100% 50%)', 'hex'), '#00ffff');
        assert.equal(fmt('hsl(200grad 100% 50%)', 'hex'), '#00ffff');
        assert.equal(fmt(`hsl(${Math.PI}rad 100% 50%)`, 'hex'), '#00ffff');
        assert.equal(fmt('hsl(-120 100% 50%)', 'hex'), '#0000ff');
        assert.equal(fmt('hsla(120, 100%, 50%, 0.25)', 'hex'), '#00ff0040');
        assert.equal(fmt('hwb(0 0% 0%)', 'hex'), '#ff0000');
        assert.equal(fmt('hwb(120 50% 50%)', 'hex'), '#808080');
        assert.equal(fmt('hwb(0 25% 25% / 0.5)', 'hex'), '#bf404080');
        assert.equal(fmt('#bf4040', 'hwb'), 'hwb(0 25% 25%)');
    });

    test('names: synonyms, transparent, and nearest', () => {
        assert.equal(fmt('#00ffff', 'name'), 'aqua');
        assert.equal(fmt('#808080', 'name'), 'gray');
        assert.equal(fmt('transparent', 'rgb'), 'rgba(0, 0, 0, 0)');
        assert.equal(fmt('rgba(0,0,0,0)', 'name'), 'transparent');
        assert.equal(fmt('#ff7f51', 'name'), '~coral');
        assert.equal(fmt('rebeccapurple', 'hex'), '#663399');
    });

    test('WCAG luminance and contrast', () => {
        const white = cli(['color', '--json', 'white']);
        const j = JSON.parse(white.out);
        assert.equal(j.luminance, 1);
        assert.deepEqual(j.contrast.black, { ratio: 21, aa: true, aaa: true, aaLarge: true, aaaLarge: true });
        assert.equal(j.contrast.white.ratio, 1);
        const c = JSON.parse(cli(['color', '--json', '--contrast', 'black', 'white']).out);
        assert.equal(c.ratio, 21);
        // #767676 on white is the classic 4.54:1 AA pass
        const g = JSON.parse(cli(['color', '--json', '--contrast', '#fff', '#767676']).out);
        assert.equal(g.ratio, 4.54);
        assert.equal(g.aa, true);
        assert.equal(g.aaa, false);
        // #777 on white is 4.47:1: fails AA normal, passes AA large
        const h = JSON.parse(cli(['color', '--json', '--contrast', '#fff', '#777']).out);
        assert.equal(h.ratio, 4.47);
        assert.equal(h.aa, false);
        assert.equal(h.aaLarge, true);
        assert.equal(h.aaaLarge, false);
        assert.match(cli(['color', '--contrast', 'white', 'black']).out, /21:1/);
        assert.equal(JSON.parse(cli(['color', '--json', '#ff0000']).out).luminance, 0.2126);
    });

    test('default output lists every field', () => {
        const out = cli(['color', 'coral']).out;
        assert.match(out, /hex\s+#ff7f50/);
        assert.match(out, /hsl\s+hsl\(16, 100%, 66%\)/);
        assert.match(out, /nama\s+coral/);
        assert.match(out, /vs putih\s+2\.49:1\s+AA gagal/);
        assert.match(out, /vs hitam\s+8\.4:1\s+AA lulus, AAA lulus/);
    });

    test('--json shape', () => {
        const r = cli(['color', '--json', 'rgba(255,127,80,0.5)']);
        assert.equal(r.code, 0);
        const j = JSON.parse(r.out);
        assert.deepEqual(Object.keys(j), [
            'input',
            'hex',
            'rgb',
            'hsl',
            'hwb',
            'css',
            'alpha',
            'name',
            'nearestName',
            'luminance',
            'contrast',
        ]);
        assert.equal(j.hex, '#ff7f5080');
        assert.deepEqual(j.rgb, { r: 255, g: 127, b: 80 });
        assert.deepEqual(j.hsl, { h: 16, s: 100, l: 66 });
        assert.deepEqual(j.hwb, { h: 16, w: 31, b: 0 });
        assert.equal(j.alpha, 0.5);
        assert.equal(j.name, 'coral');
        assert.equal(j.css.hsl, 'hsla(16, 100%, 66%, 0.5)');
        assert.equal(JSON.parse(cli(['color', '--json', '#ff7f51']).out).name, null);
    });

    test('invalid input is a usage error', () => {
        for (const bad of [
            '',
            'notacolor',
            '#12',
            '#12345',
            'abc',
            'ggg000',
            'rgb(1,2)',
            'rgb(1 2 3 4)',
            'rgb(1, 2 3)',
            'rgb(1%, 2, 3)',
            'hsl(10, 50, 50)',
            'hsl(10xyz 50% 50%)',
            'hwb(1, 2%, 3%)',
            'rgb(1 2 3 / )',
            'rgb(1, 2, 3 / 0.5)',
        ]) {
            const r = cli(['color', bad]);
            assert.equal(r.code, 2, bad);
            assert.ok(r.err.length > 0);
            assert.equal(r.out, '');
        }
    });

    test('option conflicts', () => {
        assert.equal(cli(['color', '-f', 'bogus', 'red']).code, 2);
        assert.equal(cli(['color', '-f', 'hex', '--json', 'red']).code, 2);
        assert.equal(cli(['color', '-f', 'hex', '--contrast', 'blue', 'red']).code, 2);
        assert.equal(cli(['color', '--contrast', 'nope', 'red']).code, 2);
    });
    test('non-finite components are rejected without output', () => {
        const huge = '9'.repeat(400);
        const finite = '9'.repeat(308);
        for (const value of [
            `hsl(${huge} 50% 50%)`,
            `hsl(-${huge}deg 50% 50%)`,
            `hsl(${huge}grad 50% 50%)`,
            `hsl(${huge}rad 50% 50%)`,
            `hsl(${huge}turn 50% 50%)`,
            `hsl(${finite}turn 50% 50%)`,
            `hsl(${finite}rad 50% 50%)`,
            `hsl(0 ${huge}% 50%)`,
            `hsl(0 50% ${huge}%)`,
            `hwb(${huge} 0% 0%)`,
            `hwb(0 ${huge}% 0%)`,
            `hwb(0 0% ${huge}%)`,
            `rgb(${huge} 0 0)`,
            `rgb(0 ${huge} 0)`,
            `rgb(0 0 -${huge})`,
            `rgb(${huge}% 0% 0%)`,
            `rgb(${finite}% 0% 0%)`,
            `rgb(0 0 0 / ${huge})`,
            `rgb(0 0 0 / ${huge}%)`,
            `rgba(0,0,0,-${huge})`,
            'rgb(NaN 0 0)',
            'rgb(0 0 0 / Infinity)',
        ]) {
            assert.throws(() => parseColor(value), UsageError, value.slice(0, 40));
            const result = cli(['color', '-f', 'rgb', value]);
            assert.equal(result.code, 2, value.slice(0, 40));
            assert.equal(result.out, '');
            assert.ok(result.err.length > 0);
        }
        assert.deepEqual(parseColor(`rgb(${finite} -${finite} 0)`), { r: 255, g: 0, b: 0, a: 1 });
    });

    test('alpha is rounded once so all outputs agree', () => {
        for (const [input, expected] of [
            [
                'rgb(1 2 3 / 0.9999)',
                {
                    hex: '#010203',
                    alpha: 1,
                    name: null,
                    nearestName: 'black',
                    css: { rgb: 'rgb(1, 2, 3)', hsl: 'hsl(210, 50%, 1%)', hwb: 'hwb(210 0% 99%)' },
                },
            ],
            [
                'rgba(0,0,0,0.0001)',
                {
                    hex: '#00000000',
                    alpha: 0,
                    name: 'transparent',
                    nearestName: 'transparent',
                    css: { rgb: 'rgba(0, 0, 0, 0)', hsl: 'hsla(0, 0%, 0%, 0)', hwb: 'hwb(0 0% 100% / 0)' },
                },
            ],
        ] as const) {
            assert.equal(parseColor(input).a, expected.alpha);
            assert.equal(fmt(input, 'hex'), expected.hex);
            for (const format of ['rgb', 'hsl', 'hwb'] as const) {
                assert.equal(fmt(input, format), expected.css[format]);
            }
            assert.equal(fmt(input, 'name'), expected.name ?? `~${expected.nearestName}`);
            const result = cli(['color', '--json', input]);
            assert.equal(result.code, 0);
            const json = JSON.parse(result.out);
            for (const field of ['hex', 'alpha', 'name', 'nearestName', 'css'] as const) {
                assert.deepEqual(json[field], expected[field], `${input}: ${field}`);
            }
            const plain = cli(['color', input]);
            assert.equal(plain.code, 0);
            assert.match(plain.out, new RegExp(`^alpha\\s+${expected.alpha}$`, 'm'));
            assert.ok(plain.out.includes(expected.hex));
            assert.ok(plain.out.includes(expected.css.rgb));
        }
    });

    test('all alpha syntax forms use canonical three-decimal precision', () => {
        for (const [input, expected] of [
            ['rgb(1 2 3 / 99.99%)', 1],
            ['rgb(0 0 0 / 0.01%)', 0],
            ['hsla(0,0%,0%,0.0001)', 0],
            ['hsl(0 0% 0% / 0.9999)', 1],
            ['hwb(0 0% 100% / 0.0001)', 0],
            ['hwb(0 0% 100% / 0.9999)', 1],
            ['rgba(0,0,0,0.12349)', 0.123],
            ['#00000001', 0.004],
            ['#00000080', 0.502],
        ] as const) {
            assert.equal(parseColor(input).a, expected, input);
        }
    });
});
