/** A text line: raw bytes without the terminator; `eol` is false only for an unterminated last line. */
export interface Line {
    /** Exact content (latin1 of the raw bytes), so invalid UTF-8 still compares byte-for-byte. */
    key: string;
    text: string;
    eol: boolean;
}

export type OpKind = ' ' | '-' | '+';

export interface Op {
    kind: OpKind;
    line: Line;
}

/** Splits on LF only; CR stays part of the line. An empty buffer has no lines. */
export function splitLines(data: Buffer): Line[] {
    const lines: Line[] = [];
    let start = 0;
    while (start < data.length) {
        const nl = data.indexOf(0x0a, start);
        const end = nl === -1 ? data.length : nl;
        const raw = data.subarray(start, end);
        lines.push({ key: raw.toString('latin1'), text: raw.toString('utf8'), eol: nl !== -1 });
        start = end + 1;
    }
    return lines;
}

const same = (x: Line, y: Line): boolean => x.eol === y.eol && x.key === y.key;

/** Longest-common-subsequence edit script. Cost is O(n*m) time and memory after trimming common ends. */
export function diffLines(a: Line[], b: Line[]): Op[] {
    let head = 0;
    while (head < a.length && head < b.length && same(a[head]!, b[head]!)) head++;
    let tail = 0;
    while (tail < a.length - head && tail < b.length - head && same(a[a.length - 1 - tail]!, b[b.length - 1 - tail]!)) {
        tail++;
    }
    const n = a.length - head - tail;
    const m = b.length - head - tail;

    const ops: Op[] = [];
    for (let i = 0; i < head; i++) ops.push({ kind: ' ', line: a[i]! });

    if (n === 0 || m === 0) {
        for (let i = 0; i < n; i++) ops.push({ kind: '-', line: a[head + i]! });
        for (let j = 0; j < m; j++) ops.push({ kind: '+', line: b[head + j]! });
    } else {
        // Intern lines so the table fill compares integers. Distinct lines of one side cannot exceed n or m.
        const ids = new Map<string, number>();
        const id = (l: Line): number => {
            const k = (l.eol ? '1' : '0') + l.key;
            let v = ids.get(k);
            if (v === undefined) ids.set(k, (v = ids.size));
            return v;
        };
        const ai = Int32Array.from({ length: n }, (_, i) => id(a[head + i]!));
        const bi = Int32Array.from({ length: m }, (_, j) => id(b[head + j]!));

        // lcs[i][j] = LCS length of a[i..] and b[j..]; bounded by min(n, m), which fits Uint16 under the cost guard.
        const w = m + 1;
        const lcs = new Uint16Array((n + 1) * w);
        for (let i = n - 1; i >= 0; i--) {
            for (let j = m - 1; j >= 0; j--) {
                lcs[i * w + j] =
                    ai[i] === bi[j]
                        ? lcs[(i + 1) * w + j + 1]! + 1
                        : Math.max(lcs[(i + 1) * w + j]!, lcs[i * w + j + 1]!);
            }
        }
        let i = 0;
        let j = 0;
        while (i < n && j < m) {
            if (ai[i] === bi[j]) {
                ops.push({ kind: ' ', line: a[head + i]! });
                i++;
                j++;
            } else if (lcs[(i + 1) * w + j]! >= lcs[i * w + j + 1]!) {
                ops.push({ kind: '-', line: a[head + i++]! });
            } else {
                ops.push({ kind: '+', line: b[head + j++]! });
            }
        }
        while (i < n) ops.push({ kind: '-', line: a[head + i++]! });
        while (j < m) ops.push({ kind: '+', line: b[head + j++]! });
    }

    for (let t = tail; t > 0; t--) ops.push({ kind: ' ', line: a[a.length - t]! });
    return ops;
}

export interface Hunk {
    aStart: number;
    aLines: number;
    bStart: number;
    bLines: number;
    lines: { op: OpKind; text: string }[];
    /** Indexes into `lines` whose line lacks a trailing newline */
    noEol: Set<number>;
}

/** Groups changes into hunks with `context` unchanged lines around them. */
export function buildHunks(ops: Op[], context: number): Hunk[] {
    const hunks: Hunk[] = [];
    const changed: number[] = [];
    ops.forEach((op, idx) => {
        if (op.kind !== ' ') changed.push(idx);
    });

    // Cumulative line counters before each op.
    const aBefore: number[] = new Array(ops.length + 1);
    const bBefore: number[] = new Array(ops.length + 1);
    aBefore[0] = bBefore[0] = 0;
    ops.forEach((op, idx) => {
        aBefore[idx + 1] = aBefore[idx]! + (op.kind === '+' ? 0 : 1);
        bBefore[idx + 1] = bBefore[idx]! + (op.kind === '-' ? 0 : 1);
    });

    let c = 0;
    while (c < changed.length) {
        // Extend the hunk while the next change is within 2*context unchanged lines.
        let last = c;
        while (last + 1 < changed.length && changed[last + 1]! - changed[last]! - 1 <= 2 * context) last++;
        const from = Math.max(0, changed[c]! - context);
        const to = Math.min(ops.length, changed[last]! + 1 + context);
        const aCount = aBefore[to]! - aBefore[from]!;
        const bCount = bBefore[to]! - bBefore[from]!;
        const hunk: Hunk = {
            aStart: aCount === 0 ? aBefore[from]! : aBefore[from]! + 1,
            aLines: aCount,
            bStart: bCount === 0 ? bBefore[from]! : bBefore[from]! + 1,
            bLines: bCount,
            lines: [],
            noEol: new Set(),
        };
        for (let k = from; k < to; k++) {
            if (!ops[k]!.line.eol) hunk.noEol.add(hunk.lines.length);
            hunk.lines.push({ op: ops[k]!.kind, text: ops[k]!.line.text });
        }
        hunks.push(hunk);
        c = last + 1;
    }
    return hunks;
}

/** Renders unified-style hunks (without the ---/+++ header); empty string when there are no changes. */
export function formatUnified(hunks: Hunk[]): string {
    const out: string[] = [];
    for (const h of hunks) {
        out.push(`@@ -${h.aStart},${h.aLines} +${h.bStart},${h.bLines} @@`);
        h.lines.forEach((l, idx) => {
            out.push(l.op + l.text);
            if (h.noEol.has(idx)) out.push('\\ Tidak ada newline di akhir file');
        });
    }
    return out.join('\n');
}
