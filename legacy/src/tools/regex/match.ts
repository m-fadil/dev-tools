export interface MatchJob {
    pattern: string;
    /** User flags; `g` is added internally. */
    flags: string;
    text: string;
    first: boolean;
    replace?: string;
}

export interface MatchRecord {
    index: number;
    match: string;
    groups: (string | null)[];
    named: Record<string, string | null>;
}

export interface MatchResult {
    matches: MatchRecord[];
    replaced?: string;
}

export type WorkerReply = { ok: true; result: MatchResult } | { ok: false; error: string };

const FLAG_CHARS = 'dgimsuvy';

/** Rejects unknown flag letters; duplicates and u+v clashes are left to the engine. */
export function checkFlags(flags: string): void {
    for (const flag of flags) {
        if (!FLAG_CHARS.includes(flag)) throw new SyntaxError(`Flag tidak dikenal: ${flag}. Pilihan: ${FLAG_CHARS}`);
    }
}

export function withGlobal(flags: string): string {
    return flags.includes('g') ? flags : `${flags}g`;
}

function advance(text: string, index: number, unicode: boolean): number {
    if (unicode) {
        const code = text.codePointAt(index);
        if (code !== undefined && code > 0xffff) return index + 2;
    }
    return index + 1;
}

/** Runs the whole match on the calling thread; may block for long on pathological patterns. */
export function runMatch(job: MatchJob): MatchResult {
    const re = new RegExp(job.pattern, withGlobal(job.flags));
    const unicode = re.unicode || re.unicodeSets;
    const matches: MatchRecord[] = [];
    let m: RegExpExecArray | null;
    while ((m = re.exec(job.text)) !== null) {
        matches.push({
            index: m.index,
            match: m[0],
            groups: m.slice(1).map((g) => g ?? null),
            named: Object.fromEntries(Object.entries(m.groups ?? {}).map(([k, v]) => [k, v ?? null])),
        });
        if (job.first) break;
        // A zero-length match leaves lastIndex in place; step over one character to make progress.
        if (m[0] === '') re.lastIndex = advance(job.text, re.lastIndex, unicode);
    }
    if (job.replace === undefined) return { matches };
    const replacer = new RegExp(job.pattern, job.first ? job.flags.replaceAll('g', '') : withGlobal(job.flags));
    return { matches, replaced: job.text.replace(replacer, job.replace) };
}
