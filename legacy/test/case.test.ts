import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { describe, test } from 'node:test';
import { fileURLToPath } from 'node:url';

const CLI = fileURLToPath(new URL('../src/cli.js', import.meta.url));

/** run the CLI without a terminal; `input` is piped to stdin */
const cli = (args: string[], input = '') => {
    const r = spawnSync(process.execPath, [CLI, ...args], { encoding: 'utf8', input, env: { PATH: process.env.PATH } });
    return { code: r.status, out: r.stdout, err: r.stderr };
};

const to = (style: string, text: string) => cli(['case', '-t', style, text]).out.trimEnd();

describe('case', () => {
    test('word splitting: acronyms, digits, separators', () => {
        assert.equal(to('snake', 'HTTPServerError'), 'http_server_error');
        assert.equal(to('lower', 'parseURL2Json'), 'parse url 2 json');
        assert.equal(to('kebab', 'foo_bar-baz.qux/quux  corge'), 'foo-bar-baz-qux-quux-corge');
        assert.equal(to('snake', 'getHTTPResponse'), 'get_http_response');
        assert.equal(to('snake', "don't stop"), 'dont_stop');
    });

    test('all styles', () => {
        const text = 'hello WORLD fooBar';
        assert.equal(to('camel', text), 'helloWorldFooBar');
        assert.equal(to('pascal', text), 'HelloWorldFooBar');
        assert.equal(to('snake', text), 'hello_world_foo_bar');
        assert.equal(to('screaming', text), 'HELLO_WORLD_FOO_BAR');
        assert.equal(to('kebab', text), 'hello-world-foo-bar');
        assert.equal(to('train', text), 'Hello-World-Foo-Bar');
        assert.equal(to('dot', text), 'hello.world.foo.bar');
        assert.equal(to('path', text), 'hello/world/foo/bar');
        assert.equal(to('title', text), 'Hello World Foo Bar');
        assert.equal(to('sentence', text), 'Hello world foo bar');
        assert.equal(to('lower', text), 'hello world foo bar');
        assert.equal(to('upper', text), 'HELLO WORLD FOO BAR');
    });

    test('Unicode letters', () => {
        assert.equal(to('snake', 'ÉcoleNormale'), 'école_normale');
        assert.equal(to('pascal', 'straße größe'), 'StraßeGröße');
        assert.equal(to('camel', 'привет мир'), 'приветМир');
    });

    test('default is all, labelled; multi-line converts each line', () => {
        const r = cli(['case'], 'fooBar\nbaz_qux\n');
        assert.equal(r.code, 0);
        const lines = r.out.split('\n');
        assert.ok(lines.includes('camel      fooBar'));
        assert.ok(lines.includes('screaming  BAZ_QUX'));
        assert.equal(lines.filter((l) => l.startsWith('camel')).length, 2);
        assert.equal(cli(['case', '-t', 'snake'], 'fooBar\r\n\r\nbazQux\n').out, 'foo_bar\n\nbaz_qux\n');
    });

    test('--json shape', () => {
        assert.deepEqual(JSON.parse(cli(['case', '-t', 'kebab', '--json'], 'fooBar\nx Y').out), {
            style: 'kebab',
            lines: [
                { input: 'fooBar', output: 'foo-bar' },
                { input: 'x Y', output: 'x-y' },
            ],
        });
        const all = JSON.parse(cli(['case', '--json', 'a b']).out);
        assert.equal(all.style, 'all');
        assert.equal(all.lines[0].output.pascal, 'AB');
        assert.equal(Object.keys(all.lines[0].output).length, 12);
    });

    test('invalid style is a usage error', () => {
        const r = cli(['case', '-t', 'weird', 'x']);
        assert.equal(r.code, 2);
        assert.match(r.err, /--to tidak valid: weird/);
    });
});
