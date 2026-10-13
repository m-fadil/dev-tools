import { spawn } from 'node:child_process';
import { stat } from 'node:fs/promises';
import { isAbsolute, resolve } from 'node:path';

export interface CopyGitOptions {
    repo: string;
    source: string;
    destination: string;
    mailmap: string;
    /** Empty: the source remote's default branch (see sourceDefaultBranch). */
    branches: string[];
    dryRun: boolean;
    push: boolean;
    deleteTags: boolean;
}

interface Commit {
    hash: string;
    tree: string;
    authorName: string;
    authorEmail: string;
    authorDate: string;
    committerName: string;
    committerEmail: string;
    committerDate: string;
    subject: string;
}

interface Tag {
    name: string;
    object: string;
    annotated: boolean;
    commit: string | undefined;
}

interface GitResult {
    code: number;
    stdout: string;
    stderr: string;
}

const commitFormat = ['%H', '%T', '%aN', '%aE', '%aI', '%cN', '%cE', '%cI', '%s'].join('%x00');
const warning = (message: string): void => console.warn(message);
const key = (commit: Commit): string => JSON.stringify([commit.tree, commit.authorDate, commit.subject]);
const shellQuote = (value: string): string => `'${value.replaceAll("'", "'\\''")}'`;

class Git {
    constructor(
        readonly repo: string,
        readonly mailmap?: string,
    ) {}

    private spawn(args: string[], environment?: NodeJS.ProcessEnv) {
        const mailmap = this.mailmap ? ['-c', `mailmap.file=${this.mailmap}`] : [];
        return spawn('git', ['-C', this.repo, ...mailmap, ...args], {
            env: { ...process.env, ...environment },
            stdio: ['pipe', 'pipe', 'pipe'],
        });
    }

    async result(args: string[], input?: string, environment?: NodeJS.ProcessEnv): Promise<GitResult> {
        const { promise, resolve: accept, reject } = Promise.withResolvers<GitResult>();
        const child = this.spawn(args, environment);
        const stdout: Buffer[] = [];
        const stderr: Buffer[] = [];
        child.stdout.on('data', (data: Buffer) => stdout.push(data));
        child.stderr.on('data', (data: Buffer) => stderr.push(data));
        child.on('error', reject);
        // A command may exit before consuming stdin; its exit status is the useful error.
        child.stdin.on('error', (error: NodeJS.ErrnoException) => {
            if (error.code !== 'EPIPE') reject(error);
        });
        child.on('close', (code, signal) => {
            accept({
                code: code ?? 128,
                stdout: Buffer.concat(stdout).toString('utf8'),
                stderr: Buffer.concat(stderr).toString('utf8') || (signal ? `terminated by ${signal}` : ''),
            });
        });
        child.stdin.end(input);
        return promise;
    }

    async run(args: string[], input?: string, environment?: NodeJS.ProcessEnv): Promise<string> {
        const result = await this.result(args, input, environment);
        if (result.code !== 0) {
            throw new Error(
                `git ${args.join(' ')} gagal (${result.code}): ${result.stderr.trim() || result.stdout.trim()}`,
            );
        }
        return result.stdout;
    }

    async exists(ref: string): Promise<boolean> {
        const result = await this.result(['show-ref', '--verify', '--quiet', ref]);
        if (result.code === 0) return true;
        if (result.code === 1) return false;
        throw new Error(`Tidak dapat membaca ${ref}: ${result.stderr.trim()}`);
    }

    async rev(ref: string): Promise<string> {
        return (await this.run(['rev-parse', '--verify', ref])).trim();
    }

    /**
     * Streams `git log` commits in output order without buffering the whole history.
     * `visit` returning true stops the walk early (the git process is killed).
     */
    async walk(revisions: string[], visit: (commit: Commit) => boolean | void, extra: string[] = []): Promise<void> {
        if (revisions.length === 0) return;
        const args = ['log', '-z', '--use-mailmap', `--format=${commitFormat}`, ...extra, ...revisions, '--'];
        const { promise, resolve: accept, reject } = Promise.withResolvers<void>();
        const child = this.spawn(args);
        const stderr: Buffer[] = [];
        let fields: string[] = [];
        let partial = '';
        let stopped = false;
        let failure: unknown;
        // Returns true when the walk must stop: visitor finished or threw.
        const emit = (): boolean => {
            const [
                hash,
                tree,
                authorName,
                authorEmail,
                authorDate,
                committerName,
                committerEmail,
                committerDate,
                subject,
            ] = fields as [string, string, string, string, string, string, string, string, string];
            fields = [];
            try {
                return (
                    visit({
                        hash,
                        tree,
                        authorName,
                        authorEmail,
                        authorDate,
                        committerName,
                        committerEmail,
                        committerDate,
                        subject,
                    }) === true
                );
            } catch (error) {
                failure = error;
                return true;
            }
        };
        child.stdout.setEncoding('utf8');
        child.stdout.on('data', (chunk: string) => {
            if (stopped) return;
            const parts = (partial + chunk).split('\0');
            partial = parts.pop()!;
            // -z with a tformat terminates every field, including each record's last, with NUL.
            for (const part of parts) {
                fields.push(part);
                if (fields.length === 9 && emit()) {
                    stopped = true;
                    child.kill();
                    return;
                }
            }
        });
        child.stderr.on('data', (data: Buffer) => stderr.push(data));
        child.on('error', reject);
        child.stdin.end();
        child.on('close', (code, signal) => {
            if (failure !== undefined) return reject(failure);
            if (stopped) return accept();
            if (code !== 0) {
                const message = Buffer.concat(stderr).toString('utf8').trim() || `terminated by ${signal}`;
                return reject(new Error(`git ${args.join(' ')} gagal (${code ?? 128}): ${message}`));
            }
            if (partial !== '' || fields.length !== 0) return reject(new Error('Metadata commit Git tidak valid'));
            accept();
        });
        return promise;
    }

    async log(revisions: string[], extra: string[] = []): Promise<Commit[]> {
        const commits: Commit[] = [];
        await this.walk(revisions, (commit) => void commits.push(commit), extra);
        return commits;
    }

    async tags(namespace: string): Promise<Tag[]> {
        const output = await this.run([
            'for-each-ref',
            '--format=%(refname)%00%(objecttype)%00%(objectname)%00',
            `${namespace}/`,
        ]);
        const tags: Tag[] = [];
        for (const record of output.split('\0\n')) {
            if (!record) continue;
            const [ref, kind, object] = record.split('\0');
            if (!ref || !kind || !object || !ref.startsWith(`${namespace}/`))
                throw new Error('Metadata tag Git tidak valid');
            const peeled = await this.rev(`${ref}^{}`);
            const type = (await this.run(['cat-file', '-t', peeled])).trim();
            tags.push({
                name: ref.slice(namespace.length + 1),
                object,
                annotated: kind === 'tag',
                commit: type === 'commit' ? peeled : undefined,
            });
        }
        return tags;
    }
}

function identityEnvironment(commit: Commit): NodeJS.ProcessEnv {
    return {
        GIT_AUTHOR_NAME: commit.authorName,
        GIT_AUTHOR_EMAIL: commit.authorEmail,
        GIT_AUTHOR_DATE: commit.authorDate,
        GIT_COMMITTER_NAME: commit.committerName,
        GIT_COMMITTER_EMAIL: commit.committerEmail,
        GIT_COMMITTER_DATE: commit.committerDate,
    };
}

async function tagIdentity(
    git: Git,
    tag: Tag,
): Promise<{ message: string; environment: NodeJS.ProcessEnv } | undefined> {
    const object = await git.run(['cat-file', 'tag', tag.object]);
    const boundary = object.indexOf('\n\n');
    const header = boundary < 0 ? object : object.slice(0, boundary);
    const tagger = /^tagger (.*) <([^<>]*)> (\d+ [+-]\d{4})$/m.exec(header);
    if (!tagger) {
        warning(`   ${tag.name}: tagger kosong/tidak valid, lewati`);
        return undefined;
    }
    const mapped = (await git.run(['check-mailmap', '--stdin'], `${tagger[1]} <${tagger[2]}>\n`)).trimEnd();
    const identity = /^(.*) <([^<>]*)>$/.exec(mapped);
    if (!identity) throw new Error(`Identitas mailmap tag ${tag.name} tidak valid`);
    const contents = boundary < 0 ? '' : object.slice(boundary + 2);
    // Transformed objects cannot retain a signature covering their original target/identity.
    const signature = /^-----BEGIN (?:PGP|SSH) SIGNATURE-----/m.exec(contents);
    return {
        message: signature ? contents.slice(0, signature.index) : contents,
        environment: {
            GIT_COMMITTER_NAME: identity[1],
            GIT_COMMITTER_EMAIL: identity[2],
            GIT_COMMITTER_DATE: tagger[3],
        },
    };
}

async function destinationTags(git: Git, remote: string): Promise<Map<string, string>> {
    const refs = new Map<string, string>();
    const output = await git.run(['ls-remote', '--refs', '--tags', '--', remote]);
    for (const line of output.split('\n')) {
        if (!line) continue;
        const separator = line.indexOf('\t');
        if (separator < 0) throw new Error('Metadata remote tag Git tidak valid');
        refs.set(line.slice(separator + 1), line.slice(0, separator));
    }
    return refs;
}

/** Branch the source remote's HEAD points to (e.g. main or master); asks the remote, so it needs network access. */
export async function sourceDefaultBranch(repo: string, source: string): Promise<string> {
    if (source.startsWith('-')) throw new Error(`Remote sumber tidak valid: ${source}`);
    const output = await new Git(resolve(repo)).run(['ls-remote', '--symref', '--', source, 'HEAD']);
    const branch = /^ref: refs\/heads\/(\S+)\tHEAD$/m.exec(output)?.[1];
    if (!branch) throw new Error(`Branch default ${source} tidak terdeteksi; sebutkan branch secara eksplisit.`);
    return branch;
}

export async function copyGit(options: CopyGitOptions): Promise<void> {
    const repo = resolve(options.repo);
    const mailmap = isAbsolute(options.mailmap) ? options.mailmap : resolve(repo, options.mailmap);
    if (!(await stat(mailmap).catch(() => undefined))?.isFile()) throw new Error(`mailmap tidak ada: ${mailmap}`);
    const git = new Git(repo, mailmap);
    const remotes = (await git.run(['remote'])).trim().split('\n');
    if (!remotes.includes(options.source) || options.source.startsWith('-'))
        throw new Error(`Remote sumber tidak ada/tidak valid: ${options.source}`);
    if (options.push && (!remotes.includes(options.destination) || options.destination.startsWith('-')))
        throw new Error(`Remote tujuan tidak ada/tidak valid: ${options.destination}`);
    const namespace = `refs/copygit-tags/${options.source}`;
    await git.run(['check-ref-format', `${namespace}/check`]);
    const branches = options.branches.length
        ? [...new Set(options.branches)]
        : [await sourceDefaultBranch(repo, options.source)];
    if (!options.branches.length) console.log(`>> branch default ${options.source}: ${branches[0]}`);
    for (const branch of branches) {
        if (branch.startsWith('-')) throw new Error(`Nama branch tidak valid: ${branch}`);
        await git.run(['check-ref-format', `refs/heads/${branch}`]);
    }
    if ((await git.run(['status', '--porcelain=v1', '--untracked-files=no'])).trim())
        throw new Error(`working tree ${repo} kotor; simpan perubahan terlebih dahulu`);
    for (const state of [
        'CHERRY_PICK_HEAD',
        'MERGE_HEAD',
        'REVERT_HEAD',
        'rebase-merge',
        'rebase-apply',
        'sequencer',
    ]) {
        const path = (await git.run(['rev-parse', '--git-path', state])).trim();
        if (await stat(resolve(repo, path)).catch(() => undefined))
            throw new Error(`Operasi Git ${state} masih berlangsung; selesaikan terlebih dahulu`);
    }
    const originalHead = await git.rev('HEAD');
    const symbolic = await git.result(['symbolic-ref', '--quiet', 'HEAD']);
    if (symbolic.code !== 0 && symbolic.code !== 1)
        throw new Error(`Tidak dapat membaca HEAD: ${symbolic.stderr.trim()}`);
    const originalBranch = symbolic.code === 0 ? symbolic.stdout.trim().replace(/^refs\/heads\//, '') : undefined;
    const pending = new Set<string>();
    const syncedBranches: string[] = [];
    const plainTags: string[] = [];
    const movedTags: string[] = [];
    const deletedTags: string[] = [];
    const observed =
        options.push && !options.dryRun ? await destinationTags(git, options.destination) : new Map<string, string>();
    let preserveRecovery = false;
    let switched = false;
    const restore = async (): Promise<void> => {
        if (!switched || preserveRecovery) return;
        if (originalBranch) await git.run(['switch', '--no-guess', '--', originalBranch]);
        else await git.run(['switch', '--detach', originalHead]);
    };

    try {
        console.log(`>> fetch remote ${options.source}`);
        // Explicit destinations prevent remote fetch configuration from writing local heads/tags.
        await git.run([
            'fetch',
            '-q',
            '--no-tags',
            '--refmap=',
            '--',
            options.source,
            `+refs/heads/*:refs/remotes/${options.source}/*`,
        ]);
        await git.run([
            'fetch',
            '-q',
            '--no-tags',
            '--prune',
            '--force',
            '--refmap=',
            '--',
            options.source,
            `+refs/tags/*:${namespace}/*`,
        ]);

        for (const branch of branches) {
            console.log(`== ${branch}`);
            const localRef = `refs/heads/${branch}`;
            const sourceRef = `refs/remotes/${options.source}/${branch}`;
            if (!(await git.exists(localRef)) || !(await git.exists(sourceRef))) {
                warning(`   branch lokal atau ${options.source}/${branch} tidak ada, lewati`);
                continue;
            }
            const tip = (await git.log([localRef], ['-1']))[0]!;
            const tipKey = key(tip);
            // Match by the tag identity (tree, author date, subject), not the tree alone: an empty commit shares
            // its parent's tree, so a tree-only match could select a newer upstream commit and skip it.
            const match: { key?: Commit; tree?: Commit; trees: number } = { trees: 0 };
            await git.walk([sourceRef], (commit) => {
                if (key(commit) === tipKey) {
                    match.key = commit;
                    return true;
                }
                if (commit.tree !== tip.tree) return false;
                match.tree ??= commit;
                match.trees++;
                return false;
            });
            if (!match.key && match.trees === 0)
                throw new Error(`tree tip ${branch} tidak ditemukan di ${options.source}/${branch}; periksa manual`);
            // Tree-only fallback (e.g. upstream rewrote the tip's subject) is safe only when the tree is unique.
            if (!match.key && match.trees > 1)
                throw new Error(
                    `tip ${branch} tidak cocok author date/subject di ${options.source}/${branch} dan tree-nya ambigu (${match.trees} commit); periksa manual`,
                );
            if (!match.key)
                warning(
                    `   author date/subject tip ${branch} tidak ditemukan di ${options.source}/${branch}; titik sinkron dipilih lewat tree saja`,
                );
            const base = (match.key ?? match.tree)!;
            const commits = await git.log([`${base.hash}..${sourceRef}`], ['--reverse']);
            if (commits.length === 0) console.log('   sudah terbaru');
            else {
                console.log(`   titik sinkron: ${base.hash.slice(0, 7)}; commit baru:`);
                for (const commit of commits) {
                    console.log(`     ${commit.hash.slice(0, 7)} ${commit.subject}`);
                    pending.add(commit.hash);
                }
                if (!options.dryRun) {
                    await git.run(['switch', '--no-guess', '--', branch]);
                    switched = true;
                    for (const commit of commits) {
                        try {
                            await git.run([
                                '-c',
                                'commit.gpgSign=false',
                                'cherry-pick',
                                '--no-gpg-sign',
                                '--no-commit',
                                commit.hash,
                            ]);
                            await git.run(
                                [
                                    '-c',
                                    'commit.gpgSign=false',
                                    'commit',
                                    '-q',
                                    '--no-gpg-sign',
                                    '--allow-empty',
                                    '--cleanup=verbatim',
                                    '-C',
                                    commit.hash,
                                    '--reset-author',
                                ],
                                undefined,
                                identityEnvironment(commit),
                            );
                        } catch (error) {
                            let unmerged: string;
                            let dirty: string;
                            try {
                                unmerged = await git.run(['diff', '--name-only', '--diff-filter=U', '-z']);
                                dirty = await git.run(['status', '--porcelain=v1', '--untracked-files=no']);
                            } catch (inspectError) {
                                // Unknown state: never switch away from possible changes, and keep the real error.
                                preserveRecovery = true;
                                warning(
                                    `   Status working tree ${branch} tidak terbaca (${inspectError instanceof Error ? inspectError.message : String(inspectError)}); HEAD tidak dipulihkan. Periksa git status secara manual.`,
                                );
                                throw error;
                            }
                            preserveRecovery = unmerged.length > 0 || dirty.trim().length > 0;
                            if (unmerged.length > 0) {
                                const environment = Object.entries(identityEnvironment(commit))
                                    .map(([name, value]) => `${name}=${shellQuote(value ?? '')}`)
                                    .join(' ');
                                throw new Error(
                                    `KONFLIK di ${commit.hash} pada ${branch}. Working tree dibiarkan untuk pemulihan manual. Selesaikan konflik, jalankan git add, lalu:\n${environment} git -c commit.gpgSign=false commit --no-gpg-sign --allow-empty --cleanup=verbatim -C ${commit.hash} --reset-author\nJalankan ulang copygit setelah commit selesai.`,
                                    { cause: error },
                                );
                            }
                            if (preserveRecovery)
                                warning(
                                    `   Perubahan belum di-commit pada ${branch}; HEAD tidak dipulihkan. Periksa git status dan selesaikan operasi manual sebelum menjalankan ulang.`,
                                );
                            throw error;
                        }
                    }
                    if ((await git.rev(`${localRef}^{tree}`)) !== (await git.rev(`${sourceRef}^{tree}`)))
                        throw new Error(`PERINGATAN: tree ${branch} != ${options.source}/${branch}; periksa manual`);
                    console.log(`   OK -> ${(await git.rev(localRef)).slice(0, 7)}`);
                }
            }
            syncedBranches.push(branch);
        }

        console.log('== tag');
        const upstreamTags = await git.tags(namespace);
        const localTags = new Map((await git.tags('refs/tags')).map((tag) => [tag.name, tag]));
        const targets = [...new Set(upstreamTags.flatMap((tag) => (tag.commit ? [tag.commit] : [])))];
        const upstreamCommits = new Map(
            (await git.log(targets, ['--no-walk=unsorted'])).map((commit) => [commit.hash, commit]),
        );
        // Index only local commits that can match a source tag; the history is streamed, not held in memory.
        const wanted = new Set([...upstreamCommits.values()].map(key));
        const wantedTrees = new Set([...upstreamCommits.values()].map((commit) => commit.tree));
        const heads: string[] = [];
        for (const branch of branches) {
            const ref = `refs/heads/${branch}`;
            if (await git.exists(ref)) heads.push(ref);
        }
        const index = new Map<string, string[]>();
        const trees = new Set<string>();
        if (wanted.size)
            await git.walk(heads, (commit) => {
                if (wantedTrees.has(commit.tree)) trees.add(commit.tree);
                const identity = key(commit);
                if (!wanted.has(identity)) return;
                const matches = index.get(identity);
                if (matches) matches.push(commit.hash);
                else index.set(identity, [commit.hash]);
            });
        for (const tag of upstreamTags) {
            if (!tag.commit) {
                warning(`   ${tag.name}: target tag bukan commit, lewati`);
                continue;
            }
            const commit = upstreamCommits.get(tag.commit);
            if (!commit) throw new Error(`Metadata commit tag ${tag.name} tidak ditemukan`);
            const candidates = index.get(key(commit)) ?? [];
            if (candidates.length === 0) {
                if (options.dryRun && pending.has(commit.hash))
                    console.log(`   ${tag.name}: akan dibuat setelah commit ${commit.hash.slice(0, 7)} dibawa`);
                else if (trees.has(commit.tree))
                    warning(`   ${tag.name}: tree cocok tapi author date/subject beda; periksa manual`);
                else warning(`   ${tag.name}: commit padanan belum ada, lewati`);
                continue;
            }
            if (candidates.length > 1) {
                warning(`   ${tag.name}: ambigu (${candidates.length} commit lokal cocok), lewati`);
                continue;
            }
            const target = candidates[0]!;
            const current = localTags.get(tag.name);
            if (current?.commit === target && current.annotated === tag.annotated) {
                plainTags.push(tag.name);
                continue;
            }
            const annotation = tag.annotated ? await tagIdentity(git, tag) : undefined;
            if (tag.annotated && !annotation) continue;
            console.log(
                `   ${tag.name} -> ${target.slice(0, 7)}${current ? ` (pindah dari ${current.object.slice(0, 7)})` : ''}`,
            );
            (current ? movedTags : plainTags).push(tag.name);
            if (!options.dryRun) {
                if (annotation)
                    await git.run(
                        [
                            '-c',
                            'tag.gpgSign=false',
                            'tag',
                            '--no-sign',
                            '-f',
                            '-a',
                            '--cleanup=verbatim',
                            '-F',
                            '-',
                            '--',
                            tag.name,
                            target,
                        ],
                        annotation.message,
                        annotation.environment,
                    );
                else await git.run(['-c', 'tag.gpgSign=false', 'tag', '--no-sign', '-f', '--', tag.name, target]);
            }
        }
        const upstreamNames = new Set(upstreamTags.map((tag) => tag.name));
        for (const name of [...localTags.keys()].sort()) {
            if (upstreamNames.has(name)) continue;
            if (!options.deleteTags) console.log(`   ${name}: hanya ada lokal (pakai --delete-tags untuk menghapus)`);
            else {
                warning(
                    `   ${name}: tidak ada di ${options.source}; ${options.dryRun ? 'akan dihapus' : 'dihapus'} (termasuk tag buatan lokal)`,
                );
                if (!options.dryRun) {
                    await git.run(['tag', '-d', '--', name]);
                    deletedTags.push(name);
                }
            }
        }
    } catch (error) {
        // The operation's error is what the user must act on; a failed restore is only reported.
        await restore().catch((restoreError: unknown) =>
            warning(
                `   HEAD awal tidak dapat dipulihkan: ${restoreError instanceof Error ? restoreError.message : String(restoreError)}`,
            ),
        );
        throw error;
    }
    await restore();

    if (options.push && !options.dryRun) {
        console.log(`>> push ke ${options.destination}`);
        // Pushes are not atomic. Tag pushes are independent of each other, so one rejected tag does not block the
        // rest; every failure is reported together at the end.
        const failures: string[] = [];
        const push = async (args: string[]): Promise<boolean> => {
            const result = await git.result(['push', ...args]);
            // git push reports ref updates and rejections on stderr.
            const output = `${result.stdout}${result.stderr}`.trimEnd();
            if (result.code === 0) {
                if (output) console.log(output);
                return true;
            }
            if (output) console.error(output);
            failures.push(`git push ${args.join(' ')} (${result.code})`);
            return false;
        };
        const branchesPushed =
            syncedBranches.length === 0 ||
            (await push([
                '--',
                options.destination,
                ...syncedBranches.map((branch) => `refs/heads/${branch}:refs/heads/${branch}`),
            ]));
        // Tags point at the synced branch commits; without the branches on the destination they are held back.
        if (!branchesPushed) {
            if (plainTags.length || movedTags.length || deletedTags.length)
                failures.push('push tag dilewati karena push branch gagal');
        } else {
            if (plainTags.length)
                await push([
                    '--',
                    options.destination,
                    ...plainTags.map((name) => `refs/tags/${name}:refs/tags/${name}`),
                ]);
            for (const name of movedTags) {
                const ref = `refs/tags/${name}`;
                await push([
                    `--force-with-lease=${ref}:${observed.get(ref) ?? ''}`,
                    '--',
                    options.destination,
                    `${ref}:${ref}`,
                ]);
            }
            for (const name of deletedTags) {
                const ref = `refs/tags/${name}`;
                const previous = observed.get(ref);
                if (previous)
                    await push([`--force-with-lease=${ref}:${previous}`, '--', options.destination, `:${ref}`]);
            }
        }
        if (failures.length) throw new Error(`Push gagal:\n${failures.map((failure) => `  ${failure}`).join('\n')}`);
    }
    console.log('>> selesai');
}
