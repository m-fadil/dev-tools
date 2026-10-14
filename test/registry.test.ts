import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { describe, test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { findTool, tools } from '../src/tools/index.js';

const CLI = fileURLToPath(new URL('../src/cli.js', import.meta.url));
const cli = (args: string[]) => {
    const r = spawnSync(process.execPath, [CLI, ...args], { encoding: 'utf8', env: { PATH: process.env.PATH } });
    return { code: r.status, out: r.stdout, err: r.stderr };
};

describe('registry', () => {
    test('names and aliases are unique across all tools', () => {
        const seen = new Set<string>();
        for (const tool of tools) {
            for (const name of [tool.name, ...(tool.aliases ?? [])]) {
                assert.ok(!seen.has(name), `duplicate command name: ${name}`);
                seen.add(name);
            }
        }
    });

    test('aliases dispatch to their tool', () => {
        assert.equal(findTool('ts')?.name, 'time');
        assert.equal(cli(['ts', '0', '--json']).code, 0);
        assert.equal(JSON.parse(cli(['ts', '0', '-z', 'UTC', '--json']).out).epochSeconds, 0);
    });
});

describe('completion', () => {
    test('lists tool names and aliases for each shell', () => {
        for (const shell of ['bash', 'zsh', 'fish']) {
            const r = cli(['completion', shell]);
            assert.equal(r.code, 0, r.err);
            for (const name of ['syncgit', 'semver', 'sv', 'ts'])
                assert.match(r.out, new RegExp(`\\b${name}\\b`), shell);
        }
        assert.equal(cli(['completion', 'powershell']).code, 2);
        assert.equal(cli(['completion']).code, 2);
    });
});
