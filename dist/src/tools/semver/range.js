import { UsageError } from '../../errors.js';
import { compare, PRE_RE, toNumber } from './version.js';
const X = '(\\d+|[xX*])';
const PARTIAL_RE = new RegExp(`^v?${X}(?:\\.${X}(?:\\.${X}(?:-([0-9A-Za-z.-]+))?(?:\\+[0-9A-Za-z-]+(?:\\.[0-9A-Za-z-]+)*)?)?)?$`);
const OPERATOR_RE = /^(\^|~>?|<=|>=|<|>|=)?(.*)$/;
const ver = (major, minor, patch, prerelease = []) => ({
    major,
    minor,
    patch,
    prerelease,
    build: [],
});
function parsePartial(text, range) {
    const m = PARTIAL_RE.exec(text);
    const bad = () => new UsageError(`Range tidak valid: "${text}" dalam "${range}"`);
    if (!m)
        throw bad();
    const parts = [m[1], m[2], m[3]].map((p) => (p === undefined || /^[xX*]$/.test(p) ? undefined : p));
    // After a wildcard nothing more specific may follow (1.x.3 is meaningless).
    for (let i = 1; i < 3; i++) {
        if (parts[i] !== undefined && parts[i - 1] === undefined)
            throw bad();
    }
    for (const p of parts) {
        if (p !== undefined && p.length > 1 && p.startsWith('0'))
            throw bad();
    }
    const pre = m[4];
    if (pre !== undefined && (parts[2] === undefined || !PRE_RE.test(pre)))
        throw bad();
    const [major, minor, patch] = parts.map((p, i) => p === undefined ? undefined : toNumber(p, ['major', 'minor', 'patch'][i]));
    return { major, minor, patch, prerelease: pre ? pre.split('.') : [] };
}
/** Upper bound that excludes every prerelease of the given tuple. */
const below = (major, minor, patch) => ({
    op: '<',
    version: ver(major, minor, patch, ['0']),
});
const NOTHING = [below(0, 0, 0)];
function caret(p) {
    const { major, minor, patch, prerelease } = p;
    if (major === undefined)
        return [];
    if (minor === undefined)
        return [{ op: '>=', version: ver(major, 0, 0) }, below(major + 1, 0, 0)];
    if (patch === undefined) {
        return [
            { op: '>=', version: ver(major, minor, 0) },
            major > 0 ? below(major + 1, 0, 0) : below(0, minor + 1, 0),
        ];
    }
    const lower = { op: '>=', version: ver(major, minor, patch, prerelease) };
    if (major > 0)
        return [lower, below(major + 1, 0, 0)];
    if (minor > 0)
        return [lower, below(0, minor + 1, 0)];
    return [lower, below(0, 0, patch + 1)];
}
function tilde(p) {
    const { major, minor, patch, prerelease } = p;
    if (major === undefined)
        return [];
    if (minor === undefined)
        return [{ op: '>=', version: ver(major, 0, 0) }, below(major + 1, 0, 0)];
    return [{ op: '>=', version: ver(major, minor, patch ?? 0, prerelease) }, below(major, minor + 1, 0)];
}
function primitive(op, p) {
    const { major, minor, patch, prerelease } = p;
    if (major === undefined)
        return op === '>' || op === '<' ? NOTHING : [];
    if (patch !== undefined)
        return [{ op, version: ver(major, minor, patch, prerelease) }];
    // PartialVersion version: treat the missing parts as a wildcard.
    const lo = ver(major, minor ?? 0, 0);
    const next = minor === undefined ? below(major + 1, 0, 0) : below(major, minor + 1, 0);
    switch (op) {
        case '=':
            return [{ op: '>=', version: lo }, next];
        case '>':
            return [{ op: '>=', version: { ...next.version, prerelease: [] } }];
        case '>=':
            return [{ op: '>=', version: lo }];
        case '<':
            return [{ op: '<', version: { ...lo, prerelease: ['0'] } }];
        case '<=':
            return [next];
    }
}
function hyphen(from, to) {
    const set = [];
    if (from.major !== undefined) {
        const { major, minor, patch, prerelease } = from;
        set.push({ op: '>=', version: ver(major, minor ?? 0, patch ?? 0, prerelease) });
    }
    if (to.major !== undefined) {
        const { major, minor, patch, prerelease } = to;
        if (minor === undefined)
            set.push(below(major + 1, 0, 0));
        else if (patch === undefined)
            set.push(below(major, minor + 1, 0));
        else
            set.push({ op: '<=', version: ver(major, minor, patch, prerelease) });
    }
    return set;
}
function parseSet(text, range) {
    const tokens = text.split(/\s+/).filter(Boolean);
    const set = [];
    for (let i = 0; i < tokens.length; i++) {
        let token = tokens[i];
        if (tokens[i + 1] === '-') {
            if (i + 2 >= tokens.length)
                throw new UsageError(`Range hyphen tidak lengkap: "${range}"`);
            set.push(...hyphen(parsePartial(token, range), parsePartial(tokens[i + 2], range)));
            i += 2;
            continue;
        }
        // Allow a detached operator: ">= 1.2.3".
        if (/^(\^|~>?|<=|>=|<|>|=)$/.test(token)) {
            if (i + 1 >= tokens.length)
                throw new UsageError(`Operator tanpa versi dalam range: "${range}"`);
            token += tokens[++i];
        }
        const [, op = '', rest = ''] = OPERATOR_RE.exec(token);
        const partial = parsePartial(rest, range);
        if (op === '^')
            set.push(...caret(partial));
        else if (op === '~' || op === '~>')
            set.push(...tilde(partial));
        else
            set.push(...primitive((op || '='), partial));
    }
    return set;
}
/** Parses an npm-style range into OR-ed comparator sets. */
export function parseRange(range) {
    return range.split('||').map((part) => parseSet(part, range));
}
function test(c, v) {
    const r = compare(v, c.version);
    switch (c.op) {
        case '<':
            return r < 0;
        case '<=':
            return r <= 0;
        case '>':
            return r > 0;
        case '>=':
            return r >= 0;
        case '=':
            return r === 0;
    }
}
function setMatches(set, v) {
    if (!set.every((c) => test(c, v)))
        return false;
    if (!v.prerelease.length)
        return true;
    // A prerelease only matches a set that opts in with a prerelease on the same major.minor.patch.
    return set.some((c) => c.version.prerelease.length > 0 &&
        c.version.major === v.major &&
        c.version.minor === v.minor &&
        c.version.patch === v.patch);
}
export function satisfies(v, sets) {
    return sets.some((set) => setMatches(set, v));
}
//# sourceMappingURL=range.js.map