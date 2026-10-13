import { checkbox, confirm, input, select } from '@inquirer/prompts';
import { execFile } from 'node:child_process';
import { access } from 'node:fs/promises';
import { resolve } from 'node:path';
import { parseArgs, promisify } from 'node:util';
import { UsageError } from '../../errors.js';
import type { Tool } from '../../tool.js';
import { syncGit, sourceDefaultBranch, type SyncGitOptions } from './sync.js';

const exec = promisify(execFile);

function help(): void {
    console.log(`dt syncgit [options] [branch ...]

Tanpa opsi: wizard Inquirer. Dengan opsi: langsung berjalan, cocok untuk CI.
  --repo <path>           Repo kerja (default: direktori saat ini)
  --source <remote>       Remote sumber (default: github)
  --destination <remote>  Remote push (default: PUSH_REMOTE atau origin)
  --mailmap <path>        Mailmap, relatif ke repo (default: mailmap.txt)
  -n, --dry-run           Preview; fetch tetap dilakukan, tidak commit/tag/push
  -p, --push              Push branch dan tag setelah sinkronisasi lokal
  -d, --delete-tags       Hapus SEMUA tag lokal yang tidak ada di sumber;
                         bila --push, hapus juga tag bernama sama di tujuan
  -i, --interactive       Buka wizard dengan opsi sebagai nilai awal
  -h, --help              Tampilkan bantuan

Tanpa branch: memakai branch default remote sumber (HEAD, mis. main/master).
Tag lama yang dipindah diperbarui dengan lease; branch tidak pernah force-push.
Exit: 0 sukses, 1 gagal, 2 penggunaan salah, 130 dibatalkan.
Contoh: dt syncgit --repo /path/to/repo -n main`);
}

async function gitLines(repo: string, ...args: string[]): Promise<string[]> {
    const { stdout } = await exec('git', ['-C', repo, ...args]);
    return stdout.trim().split('\n').filter(Boolean);
}

async function wizard(initial: SyncGitOptions): Promise<SyncGitOptions | undefined> {
    if (!process.stdin.isTTY || !process.stdout.isTTY) {
        throw new Error('Wizard memerlukan terminal. Gunakan --repo <path> dan flag tanpa --interactive.');
    }
    const repo = resolve(
        (
            await input({
                message: 'Path repo kerja',
                default: initial.repo,
                validate: async (value) => {
                    try {
                        const result = await gitLines(resolve(value.trim()), 'rev-parse', '--is-inside-work-tree');
                        return result[0] === 'true' || 'Gunakan repo Git dengan working tree.';
                    } catch {
                        return 'Repo Git tidak ditemukan.';
                    }
                },
            })
        ).trim(),
    );
    const remotes = await gitLines(repo, 'remote');
    if (!remotes.length) throw new Error('Repo belum memiliki remote Git.');
    const source = await select({
        message: 'Remote sumber (hanya dibaca)',
        default: initial.source,
        choices: remotes.map((value) => ({ value, name: value })),
    });
    const destinations = remotes.filter((name) => name !== source);
    if (!destinations.length) throw new Error('Tambahkan remote tujuan yang berbeda dari sumber.');
    const destination = await select({
        message: 'Remote tujuan push',
        default: initial.destination,
        choices: destinations.map((value) => ({ value, name: value })),
    });
    // initial.mailmap is still relative to whichever repo is chosen here, unless given as an absolute path.
    const mailmap = resolve(
        repo,
        (
            await input({
                message: 'Path mailmap (relatif ke repo atau absolut)',
                default: initial.mailmap,
                validate: async (value) => {
                    try {
                        await access(resolve(repo, value.trim()));
                        return true;
                    } catch {
                        return 'File mailmap tidak ditemukan.';
                    }
                },
            })
        ).trim(),
    );
    const heads = await gitLines(repo, 'for-each-ref', '--format=%(refname:strip=2)', 'refs/heads');
    const missing = initial.branches.filter((branch) => !heads.includes(branch));
    if (missing.length) console.warn(`Branch lokal tidak ada, tidak dapat dipilih: ${missing.join(', ')}`);
    // Preselect the source's default branch when none were given; a lookup failure only drops the preselection.
    if (!initial.branches.length) console.log(`Membaca branch default ${source} (ls-remote)...`);
    const preselected = initial.branches.length
        ? initial.branches
        : [await sourceDefaultBranch(repo, source).catch(() => '')];
    const branches = await checkbox({
        message: 'Branch yang disinkronkan',
        choices: heads.map((value) => ({ value, name: value, checked: preselected.includes(value) })),
        required: true,
    });
    const mode = await select({
        message: 'Mode sinkronisasi',
        default: initial.dryRun ? 'preview' : initial.push ? 'push' : 'preview',
        choices: [
            { name: 'Preview / dry-run (Recommended)', value: 'preview' },
            { name: 'Sinkron lokal saja', value: 'local' },
            { name: `Sinkron lokal dan push ke ${destination}`, value: 'push' },
        ],
    });
    const deleteTags = await confirm({
        message: 'Hapus SEMUA tag lokal yang tidak ada di sumber? Termasuk tag buatan manual.',
        default: initial.deleteTags,
    });
    console.log(
        `\n${source} -> lokal${mode === 'push' ? ` -> ${destination}` : ''}\nRepo: ${repo}\nBranch: ${branches.join(', ')}\nMailmap: ${mailmap}\n`,
    );
    if (mode !== 'preview' && !(await confirm({ message: 'Jalankan perubahan ini?', default: false }))) {
        console.log('Dibatalkan; repo lokal dan remote tidak diubah.');
        return undefined;
    }
    return {
        repo,
        source,
        destination,
        mailmap,
        branches,
        dryRun: mode === 'preview',
        push: mode === 'push',
        deleteTags,
    };
}

export interface SyncGitArgs {
    help: boolean;
    interactive: boolean;
    options: SyncGitOptions;
}

// Pure parsing seam: resolves defaults without touching Git, prompts, or the terminal.
export function parseSyncGitArgs(
    args: string[],
    cwd = process.cwd(),
    env: NodeJS.ProcessEnv = process.env,
): SyncGitArgs {
    try {
        const { values, positionals } = parseArgs({
            args,
            allowPositionals: true,
            options: {
                repo: { type: 'string' },
                source: { type: 'string' },
                destination: { type: 'string' },
                mailmap: { type: 'string' },
                'dry-run': { type: 'boolean', short: 'n' },
                push: { type: 'boolean', short: 'p' },
                'delete-tags': { type: 'boolean', short: 'd' },
                interactive: { type: 'boolean', short: 'i' },
                help: { type: 'boolean', short: 'h' },
            },
        });
        const repo = resolve(cwd, values.repo ?? '.');
        return {
            help: values.help ?? false,
            interactive: args.length === 0 || (values.interactive ?? false),
            options: {
                repo,
                source: values.source ?? 'github',
                destination: values.destination ?? (env.PUSH_REMOTE || 'origin'),
                // Kept relative: syncGit resolves it against the repo, which the wizard may still change.
                mailmap: values.mailmap ?? 'mailmap.txt',
                // Empty means the source remote's default branch, resolved by syncGit.
                branches: [...new Set(positionals)],
                dryRun: values['dry-run'] ?? false,
                push: values.push ?? false,
                deleteTags: values['delete-tags'] ?? false,
            },
        };
    } catch (error) {
        if ((error as NodeJS.ErrnoException).code?.startsWith('ERR_PARSE_ARGS')) {
            throw new UsageError((error as Error).message, { cause: error });
        }
        throw error;
    }
}

async function run(args: string[]): Promise<void> {
    const parsed = parseSyncGitArgs(args);
    if (parsed.help) {
        help();
        return;
    }
    const options = parsed.interactive ? await wizard(parsed.options) : parsed.options;
    if (!options) return;
    // Only pushing writes to the destination; without --push it is unused and may equal the source.
    if (options.push && options.source === options.destination) {
        throw new UsageError('Remote sumber dan tujuan harus berbeda untuk --push: syncgit tidak menulis ke sumber.');
    }
    await syncGit(options);
}

export const syncgit: Tool = {
    name: 'syncgit',
    description: 'Sinkron commit dan tag antar remote Git dengan pemetaan mailmap',
    run,
};
