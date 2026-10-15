import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { after, describe, test } from 'node:test';
import { fileURLToPath } from 'node:url';

const CLI = fileURLToPath(new URL('../src/cli.js', import.meta.url));
const dir = mkdtempSync(join(tmpdir(), 'dt-cli-'));
after(() => rmSync(dir, { recursive: true, force: true }));
const config = join(dir, 'gitconfig');
writeFileSync(config, '[user]\n\tname = Test\n\temail = test@example.com\n');

/** run the CLI with no terminal at all, as cron/CI would */
const cli = (args: string[], cwd = dir) => {
    const r = spawnSync(process.execPath, [CLI, ...args], {
        cwd,
        encoding: 'utf8',
        stdio: ['ignore', 'pipe', 'pipe'],
        env: { PATH: process.env.PATH, GIT_CONFIG_GLOBAL: config, GIT_CONFIG_NOSYSTEM: '1' },
    });
    return { code: r.status, out: r.stdout, err: r.stderr };
};

const git = (cwd: string, ...args: string[]) => {
    const r = spawnSync('git', ['-C', cwd, ...args], {
        encoding: 'utf8',
        env: { PATH: process.env.PATH, GIT_CONFIG_GLOBAL: config, GIT_CONFIG_NOSYSTEM: '1' },
    });
    assert.equal(r.status, 0, r.stderr);
    return r.stdout.trim();
};

describe('cli', () => {
    test('top-level and syncgit --help succeed and list the registered command', () => {
        const top = cli(['--help']);
        assert.equal(top.code, 0);
        assert.match(top.out, /^ {2}syncgit {6}\S/m);

        const tool = cli(['syncgit', '--help']);
        assert.equal(tool.code, 0);
    });

    test('unknown command and bad option exit 2', () => {
        const unknown = cli(['nope']);
        assert.equal(unknown.code, 2);

        const bad = cli(['syncgit', '--nope']);
        assert.equal(bad.code, 2);
        assert.match(bad.err, /Unknown option '--nope'/);
    });

    test('no terminal: menu and wizard fail fast instead of hanging', () => {
        const menu = cli([]);
        assert.equal(menu.code, 1);
        assert.match(menu.err, /Menu memerlukan terminal/);

        const wizard = cli(['syncgit', '--interactive']);
        assert.equal(wizard.code, 1);
        assert.match(wizard.err, /Wizard memerlukan terminal/);
    });

    test('identical source and destination with --push is a usage error, rejected before touching Git', () => {
        const r = cli([
            'syncgit',
            '--repo',
            join(dir, 'absent'),
            '--source',
            'origin',
            '--destination',
            'origin',
            '-p',
        ]);
        assert.equal(r.code, 2);
        assert.match(r.err, /Remote sumber dan tujuan harus berbeda/);
    });

    test('missing mailmap is reported with its resolved path', () => {
        const repo = join(dir, 'no-mailmap');
        git(dir, 'init', '-q', repo);
        const r = cli(['syncgit', '--repo', repo, '-n']);
        assert.equal(r.code, 1);
        assert.match(r.err, new RegExp(`mailmap tidak ada: ${repo}/mailmap\\.txt`));
    });

    test('dry-run from a different cwd uses the repo mailmap and leaves refs untouched', () => {
        const source = join(dir, 'source');
        const work = join(dir, 'work');
        git(dir, 'init', '-q', '-b', 'main', source);
        writeFileSync(join(source, 'f.txt'), '0\n');
        git(source, 'add', 'f.txt');
        git(source, 'commit', '-qm', 'initial');
        git(dir, 'clone', '-q', '-o', 'github', source, work);
        writeFileSync(join(work, 'mailmap.txt'), 'Test <test@destination.example> <test@example.com>\n');
        writeFileSync(join(source, 'f.txt'), '1\n');
        git(source, 'commit', '-qam', 'feature');
        const before = git(work, 'rev-parse', 'main');

        const r = cli(['syncgit', '--repo', work, '--destination', 'gitlab', '-n', 'main']);
        assert.equal(r.code, 0, r.err);
        assert.match(r.out, /== main\n {3}titik sinkron: \w+; commit baru:\n {5}\w+ feature\n/);
        assert.equal(git(work, 'rev-parse', 'main'), before);

        // Without --push the destination is unused, so it may equal the source.
        const same = cli(['syncgit', '--repo', work, '--source', 'github', '--destination', 'github', '-n', 'main']);
        assert.equal(same.code, 0, same.err);
    });

    test('without branch arguments the source default branch is used, main or master', () => {
        for (const branch of ['main', 'master']) {
            const source = join(dir, `source-${branch}`);
            const work = join(dir, `work-${branch}`);
            git(dir, 'init', '-q', '-b', branch, source);
            writeFileSync(join(source, 'f.txt'), '0\n');
            git(source, 'add', 'f.txt');
            git(source, 'commit', '-qm', 'initial');
            git(dir, 'clone', '-q', '-o', 'github', source, work);
            writeFileSync(join(work, 'mailmap.txt'), '');
            writeFileSync(join(source, 'f.txt'), '1\n');
            git(source, 'commit', '-qam', 'feature');

            const r = cli(['syncgit', '--repo', work, '--destination', 'gitlab', '-n']);
            assert.equal(r.code, 0, r.err);
            assert.match(r.out, new RegExp(`>> branch default github: ${branch}\n`), branch);
            assert.match(r.out, new RegExp(`== ${branch}\n {3}titik sinkron: \\w+; commit baru:\n {5}\\w+ feature\n`));
        }
    });
});
