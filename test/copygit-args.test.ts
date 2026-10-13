import assert from 'node:assert/strict';
import { describe, test } from 'node:test';
import { UsageError } from '../src/errors.js';
import { parseCopyGitArgs } from '../src/tools/copygit/index.js';

const cwd = '/work/current';
const parse = (args: string[], env: NodeJS.ProcessEnv = {}) => parseCopyGitArgs(args, cwd, env);

describe('copygit argument parsing', () => {
    test('defaults resolve against the current directory and never enable writes', () => {
        assert.deepEqual(parse(['-n']).options, {
            repo: cwd,
            source: 'github',
            destination: 'origin',
            mailmap: 'mailmap.txt',
            branches: [],
            dryRun: true,
            push: false,
            deleteTags: false,
        });
    });

    test('relative repo resolves from cwd; mailmap stays as given for copyGit to resolve against the repo', () => {
        assert.equal(parse(['--repo', '../other']).options.repo, '/work/other');
        assert.equal(parse(['--repo=../other']).options.repo, '/work/other');
        assert.equal(parse(['--repo', '../other', '--mailmap', 'ids/map.txt']).options.mailmap, 'ids/map.txt');
        assert.equal(parse(['--mailmap', '/etc/map.txt']).options.mailmap, '/etc/map.txt');
    });

    test('destination precedence: flag over non-empty PUSH_REMOTE over origin', () => {
        assert.equal(parse(['-n']).options.destination, 'origin');
        assert.equal(parse(['-n'], { PUSH_REMOTE: 'mirror' }).options.destination, 'mirror');
        assert.equal(parse(['-n'], { PUSH_REMOTE: '' }).options.destination, 'origin');
        assert.equal(parse(['--destination', 'gitlab'], { PUSH_REMOTE: 'mirror' }).options.destination, 'gitlab');
    });

    test('no branch arguments leaves branches empty for source default detection', () => {
        assert.deepEqual(parse(['-n']).options.branches, []);
    });

    test('positional branches keep order and drop duplicates', () => {
        assert.deepEqual(parse(['main', 'release', 'main']).options.branches, ['main', 'release']);
        assert.deepEqual(parse(['-n', '--', '-odd']).options.branches, ['-odd']);
    });

    test('short and long flags are equivalent and combinable', () => {
        for (const args of [['-npd'], ['--dry-run', '--push', '--delete-tags']]) {
            const { dryRun, push, deleteTags } = parse(args).options;
            assert.deepEqual(
                { dryRun, push, deleteTags },
                { dryRun: true, push: true, deleteTags: true },
                args.join(' '),
            );
        }
    });

    test('wizard opens with no arguments or --interactive; any other argument runs directly', () => {
        assert.equal(parse([]).interactive, true);
        assert.equal(parse(['-i', '--repo', 'x']).interactive, true);
        assert.equal(parse(['--repo', 'x']).interactive, false);
        assert.equal(parse(['main']).interactive, false);
    });

    test('help is reported without requiring other options', () => {
        assert.equal(parse(['-h']).help, true);
        assert.equal(parse(['--help', '--repo', 'x']).help, true);
        assert.equal(parse(['-n']).help, false);
    });

    test('usage errors are UsageError (exit 2), keeping the parser message', () => {
        for (const [args, message] of [
            [['--nope'], /Unknown option '--nope'/],
            [['--repo'], /argument missing/],
            [['--push=yes'], /does not take an argument/],
        ] as const) {
            assert.throws(
                () => parse([...args]),
                (e) => e instanceof UsageError && message.test(e.message),
                args.join(' '),
            );
        }
    });
});
