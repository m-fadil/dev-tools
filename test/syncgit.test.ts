import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { describe, test, type TestContext } from 'node:test';
import { syncGit, type SyncGitOptions } from '../src/tools/syncgit/sync.js';

const D1 = '2026-01-01T10:00:00+00:00';
const D2 = '2026-01-02T10:00:00+00:00';
const D3 = '2026-01-03T10:00:00+00:00';
/** git `--date=raw` form of the UTC fixture dates above. */
const raw = (date: string): string => `${Date.parse(date) / 1000} +0000`;

function identity(date: string, committerDate = date): NodeJS.ProcessEnv {
    return {
        GIT_AUTHOR_NAME: 'Dev',
        GIT_AUTHOR_EMAIL: 'developer@source.example',
        GIT_AUTHOR_DATE: date,
        GIT_COMMITTER_NAME: 'Dev',
        GIT_COMMITTER_EMAIL: 'developer@source.example',
        GIT_COMMITTER_DATE: committerDate,
    };
}

function git(repo: string, args: string[], env: NodeJS.ProcessEnv = {}): string {
    return execFileSync('git', ['-C', repo, ...args], {
        encoding: 'utf8',
        env: { ...process.env, ...env },
        stdio: ['ignore', 'pipe', 'pipe'],
    }).trim();
}

function commit(repo: string, message: string, content: string, date = D1, committerDate = date): string {
    writeFileSync(join(repo, 'f.txt'), content);
    git(repo, ['add', '--', 'f.txt']);
    git(repo, ['commit', '-q', '-m', message], identity(date, committerDate));
    return git(repo, ['rev-parse', 'HEAD']);
}

function annotatedTag(repo: string, name: string, date = D1, message = 'release', ref = 'HEAD'): void {
    git(repo, ['tag', '-f', '-a', '-m', message, name, ref], identity(date));
}

function refs(repo: string): string {
    return git(repo, ['for-each-ref', '--format=%(refname) %(objectname)', 'refs/heads', 'refs/tags']);
}

function fixture(t: TestContext) {
    const root = mkdtempSync(join(tmpdir(), 'syncgit-test-'));
    // HOME/XDG too: user-level ~/.config/git files (attributes, ignore) must not leak into fixtures.
    const isolated = /^(GIT_|HOME$|XDG_CONFIG_HOME$)/;
    const previous = new Map(Object.entries(process.env).filter(([key]) => isolated.test(key)));
    for (const key of previous.keys()) delete process.env[key];
    const config = join(root, 'gitconfig');
    writeFileSync(
        config,
        '[user]\n\tname = Test\n\temail = test@example.com\n[commit]\n\tgpgSign = false\n[tag]\n\tgpgSign = false\n[core]\n\tautocrlf = false\n',
    );
    process.env.GIT_CONFIG_GLOBAL = config;
    process.env.GIT_CONFIG_NOSYSTEM = '1';
    process.env.GIT_TERMINAL_PROMPT = '0';
    process.env.HOME = root;
    process.env.XDG_CONFIG_HOME = join(root, '.config');
    t.after(() => {
        for (const key of Object.keys(process.env)) if (isolated.test(key)) delete process.env[key];
        for (const [key, value] of previous) process.env[key] = value;
        rmSync(root, { recursive: true, force: true });
    });

    const source = join(root, 'source');
    const work = join(root, 'work');
    const destination = join(root, 'destination.git');
    git(root, ['init', '-q', '-b', 'develop', source]);
    const initial = commit(source, 'initial', '0\n');
    git(root, ['clone', '-q', '--no-local', source, work]);
    git(work, ['remote', 'rename', 'origin', 'github']);
    git(root, ['init', '-q', '--bare', destination]);
    git(work, ['remote', 'add', 'origin', destination]);
    const mailmap = join(root, 'mailmap.txt');
    writeFileSync(mailmap, 'Dev <developer@destination.example> <developer@source.example>\n');
    const options: SyncGitOptions = {
        repo: work,
        source: 'github',
        destination: 'origin',
        mailmap,
        branches: ['develop'],
        dryRun: false,
        push: false,
        deleteTags: false,
    };
    return {
        source,
        work,
        destination,
        initial,
        sync: (overrides: Partial<SyncGitOptions> = {}) => syncGit({ ...options, ...overrides }),
    };
}

// Fixtures temporarily isolate process-level Git configuration used by syncGit.
// Keep them sequential even when the runner's concurrency setting changes.
describe('syncGit integration', { concurrency: false }, () => {
    test('maps author and committer while preserving dates, full message and tree', async (t) => {
        const f = fixture(t);
        const upstream = commit(f.source, 'feat: x\n\nbody with ünicode', '1\n', D2, D3);
        await f.sync();
        // raw dates (epoch + offset): iso-strict prints UTC as `Z` or `+00:00` depending on the git version.
        const format = '%an <%ae>|%ad|%cn <%ce>|%cd|%B';
        assert.equal(
            git(f.work, ['log', '-1', '--date=raw', `--format=${format}`, 'develop']),
            `Dev <developer@destination.example>|${raw(D2)}|Dev <developer@destination.example>|${raw(D3)}|feat: x\n\nbody with ünicode`,
        );
        assert.equal(git(f.work, ['rev-parse', 'develop^{tree}']), git(f.source, ['rev-parse', `${upstream}^{tree}`]));
        assert.notEqual(git(f.work, ['rev-parse', 'develop']), upstream);
        assert.equal(git(f.work, ['rev-parse', 'develop^']), f.initial);
    });

    test('matches a rewritten upstream tip by content without duplicating its commit', async (t) => {
        const f = fixture(t);
        git(f.source, ['commit', '-q', '--amend', '--no-edit'], identity(D1, D2));
        assert.notEqual(git(f.source, ['rev-parse', 'HEAD']), f.initial);
        const upstream = commit(f.source, 'new', '1\n', D2);
        await f.sync();
        assert.equal(git(f.work, ['rev-list', '--count', 'develop']), '2');
        assert.equal(git(f.work, ['rev-parse', 'develop^']), f.initial);
        assert.equal(git(f.work, ['rev-parse', 'develop^{tree}']), git(f.source, ['rev-parse', `${upstream}^{tree}`]));
    });

    test('a tip whose subject was rewritten upstream falls back to a unique tree match', async (t) => {
        const f = fixture(t);
        git(f.source, ['commit', '-q', '--amend', '-m', 'initial renamed'], identity(D1));
        commit(f.source, 'new', '1\n', D2);
        await f.sync();
        assert.equal(git(f.work, ['log', '--format=%s', 'develop']), 'new\ninitial');
    });

    test('a tree-only fallback that is ambiguous stops instead of guessing which commits are new', async (t) => {
        const f = fixture(t);
        git(f.source, ['commit', '-q', '--amend', '-m', 'initial renamed'], identity(D1));
        git(f.source, ['commit', '-q', '--allow-empty', '-m', 'empty'], identity(D2));
        const before = refs(f.work);
        await assert.rejects(f.sync(), /tree-nya ambigu \(2 commit\)/);
        assert.equal(refs(f.work), before);
    });

    test('an empty upstream commit after the last synced one is still imported', async (t) => {
        const f = fixture(t);
        git(f.source, ['commit', '-q', '--allow-empty', '-m', 'empty'], identity(D2));
        await f.sync();
        git(f.source, ['commit', '-q', '--allow-empty', '-m', 'empty2'], identity(D3));
        git(f.source, ['tag', 'v2']);
        await f.sync();
        assert.equal(git(f.work, ['log', '--format=%s', 'develop']), 'empty2\nempty\ninitial');
        assert.equal(git(f.work, ['rev-parse', 'v2']), git(f.work, ['rev-parse', 'develop']));
    });

    test('restores the original branch or detached HEAD after syncing another branch', async (t) => {
        const f = fixture(t);
        git(f.work, ['switch', '-q', '-c', 'other']);
        commit(f.source, 'one', '1\n', D2);
        await f.sync();
        assert.equal(git(f.work, ['symbolic-ref', 'HEAD']), 'refs/heads/other');
        git(f.work, ['switch', '-q', '--detach', 'other']);
        commit(f.source, 'two', '2\n', D3);
        await f.sync();
        assert.equal(git(f.work, ['rev-parse', '--abbrev-ref', 'HEAD']), 'HEAD');
        assert.equal(git(f.work, ['rev-parse', 'HEAD']), f.initial);
        assert.equal(git(f.work, ['log', '--format=%s', 'develop']), 'two\none\ninitial');
    });

    test('dry-run does not change local heads, tags, checkout or destination even with push and deletion', async (t) => {
        const f = fixture(t);
        git(f.work, ['branch', 'untouched']);
        git(f.work, ['tag', 'manual']);
        git(f.work, ['push', '-q', 'origin', 'develop', 'refs/tags/manual']);
        commit(f.source, 'new', '1\n', D2);
        annotatedTag(f.source, 'v1', D2);
        const localBefore = refs(f.work);
        const remoteBefore = refs(f.destination);
        const checkoutBefore = git(f.work, ['symbolic-ref', 'HEAD']);
        await f.sync({ dryRun: true, push: true, deleteTags: true });
        assert.equal(refs(f.work), localBefore);
        assert.equal(refs(f.destination), remoteBefore);
        assert.equal(git(f.work, ['symbolic-ref', 'HEAD']), checkoutBefore);
        assert.equal(git(f.work, ['show', 'HEAD:f.txt']), '0');
        assert.equal(git(f.work, ['status', '--porcelain']), '');
    });

    test('maps annotated and lightweight tags to local commits and preserves mapped tagger metadata', async (t) => {
        const f = fixture(t);
        const upstream = commit(f.source, 'new', '1\n', D2);
        annotatedTag(f.source, 'v1', D3, 'release one\n\nnotes');
        git(f.source, ['tag', 'light']);
        await f.sync();
        const local = git(f.work, ['rev-parse', 'develop']);
        assert.notEqual(local, upstream);
        assert.equal(git(f.work, ['rev-parse', 'v1^{commit}']), local);
        assert.equal(git(f.work, ['rev-parse', 'light']), local);
        assert.equal(git(f.work, ['cat-file', '-t', 'light']), 'commit');
        assert.equal(git(f.work, ['cat-file', '-t', 'v1']), 'tag');
        assert.equal(
            git(f.work, [
                'for-each-ref',
                'refs/tags/v1',
                '--format=%(taggername)|%(taggeremail)|%(taggerdate:raw)|%(contents)',
            ]),
            `Dev|<developer@destination.example>|${raw(D3)}|release one\n\nnotes`,
        );
        const before = refs(f.work);
        await f.sync();
        assert.equal(refs(f.work), before);
    });

    test('pushes destination branch and tags and force-updates a moved source tag', async (t) => {
        const f = fixture(t);
        commit(f.source, 'one', '1\n', D1);
        annotatedTag(f.source, 'v1');
        await f.sync({ push: true });
        const firstTag = git(f.destination, ['rev-parse', 'v1']);
        assert.equal(git(f.destination, ['rev-parse', 'develop']), git(f.work, ['rev-parse', 'develop']));
        assert.equal(git(f.destination, ['rev-parse', 'v1']), git(f.work, ['rev-parse', 'v1']));
        const second = commit(f.source, 'two', '2\n', D2);
        annotatedTag(f.source, 'v1', D2, 'moved release', second);
        await f.sync({ push: true });
        assert.notEqual(git(f.destination, ['rev-parse', 'v1']), firstTag);
        assert.equal(git(f.destination, ['rev-parse', 'v1']), git(f.work, ['rev-parse', 'v1']));
        assert.equal(git(f.destination, ['rev-parse', 'v1^{commit}']), git(f.work, ['rev-parse', 'develop']));
        assert.equal(git(f.destination, ['rev-parse', 'develop']), git(f.work, ['rev-parse', 'develop']));
    });

    test('preserves absent source tags by default and deletes local and remote tags only on opt-in', async (t) => {
        const f = fixture(t);
        git(f.work, ['tag', 'manual']);
        git(f.work, ['push', '-q', 'origin', 'develop', 'refs/tags/manual']);
        await f.sync({ push: true });
        assert.equal(git(f.work, ['tag']), 'manual');
        assert.equal(git(f.destination, ['tag']), 'manual');
        await f.sync({ push: true, deleteTags: true });
        assert.equal(git(f.work, ['tag']), '');
        assert.equal(git(f.destination, ['tag']), '');
    });

    test('a rejected destination tag is not overwritten and does not block the other pushes', async (t) => {
        const f = fixture(t);
        commit(f.source, 'one', '1\n', D1);
        annotatedTag(f.source, 'v1');
        await f.sync({ push: true });
        const second = commit(f.source, 'two', '2\n', D2);
        annotatedTag(f.source, 'v1', D2, 'moved release', second);
        annotatedTag(f.source, 'v2', D2);
        git(f.destination, ['tag', 'v2', 'develop']);
        const conflicting = git(f.destination, ['rev-parse', 'v2']);
        await assert.rejects(f.sync({ push: true }), /Push gagal:\n.*refs\/tags\/v2/);
        assert.equal(git(f.destination, ['rev-parse', 'develop']), git(f.work, ['rev-parse', 'develop']));
        assert.equal(git(f.destination, ['rev-parse', 'v1']), git(f.work, ['rev-parse', 'v1']));
        assert.equal(git(f.destination, ['rev-parse', 'v2']), conflicting);
        assert.equal(git(f.destination, ['cat-file', '-t', 'v2']), 'commit');
    });

    test('skips tags whose matching local commit is ambiguous while synchronizing the branch', async (t) => {
        const f = fixture(t);
        commit(f.source, 'x', '1\n', D2);
        commit(f.source, 'y', '2\n', D2);
        commit(f.source, 'x', '1\n', D2);
        annotatedTag(f.source, 'v1', D2);
        await f.sync({ push: true });
        assert.equal(git(f.work, ['tag']), '');
        assert.equal(git(f.destination, ['tag']), '');
        assert.equal(git(f.work, ['rev-list', '--count', 'develop']), '4');
        assert.equal(git(f.destination, ['rev-parse', 'develop']), git(f.work, ['rev-parse', 'develop']));
    });

    test('rejects a dirty working tree without changing local or destination refs', async (t) => {
        const f = fixture(t);
        commit(f.source, 'new', '1\n', D2);
        const before = refs(f.work);
        const remoteBefore = refs(f.destination);
        writeFileSync(join(f.work, 'f.txt'), 'dirty\n');
        await assert.rejects(f.sync({ push: true }), /working tree .* kotor/);
        assert.equal(refs(f.work), before);
        assert.equal(refs(f.destination), remoteBefore);
        assert.equal(git(f.work, ['diff', '--', 'f.txt']).includes('+dirty'), true);
    });
});
