import { UsageError } from '../../errors.js';
export const RELEASES = ['major', 'minor', 'patch', 'premajor', 'preminor', 'prepatch', 'prerelease'];
const NUM = '0|[1-9]\\d*';
const PRE_ID = '(?:0|[1-9]\\d*|\\d*[A-Za-z-][0-9A-Za-z-]*)';
const BUILD_ID = '[0-9A-Za-z-]+';
export const PRE_RE = new RegExp(`^${PRE_ID}(?:\\.${PRE_ID})*$`);
const SEMVER_RE = new RegExp(`^(${NUM})\\.(${NUM})\\.(${NUM})(?:-(${PRE_ID}(?:\\.${PRE_ID})*))?(?:\\+(${BUILD_ID}(?:\\.${BUILD_ID})*))?$`);
const isNumeric = (id) => /^\d+$/.test(id);
export function toNumber(text, what) {
    const n = Number(text);
    if (!Number.isSafeInteger(n))
        throw new UsageError(`Angka ${what} terlalu besar: ${text}`);
    return n;
}
/** Strict SemVer 2.0.0 parse; one leading `v` is stripped. Returns an error message instead of throwing. */
export function tryParse(text) {
    const body = text.startsWith('v') ? text.slice(1) : text;
    const m = SEMVER_RE.exec(body);
    if (!m)
        return `bukan SemVer 2.0.0 yang valid: ${text}`;
    try {
        return {
            major: toNumber(m[1], 'major'),
            minor: toNumber(m[2], 'minor'),
            patch: toNumber(m[3], 'patch'),
            prerelease: m[4] ? m[4].split('.') : [],
            build: m[5] ? m[5].split('.') : [],
        };
    }
    catch (error) {
        return error.message;
    }
}
export function parseVersion(text) {
    const v = tryParse(text);
    if (typeof v === 'string')
        throw new UsageError(`Versi tidak valid, ${v}`);
    return v;
}
export function format(v, withBuild = true) {
    let s = `${v.major}.${v.minor}.${v.patch}`;
    if (v.prerelease.length)
        s += `-${v.prerelease.join('.')}`;
    if (withBuild && v.build.length)
        s += `+${v.build.join('.')}`;
    return s;
}
function compareId(a, b) {
    const an = isNumeric(a);
    const bn = isNumeric(b);
    if (an && bn) {
        // Numeric identifiers have no leading zeros, so length then lexical order equals numeric order (any size).
        if (a.length !== b.length)
            return a.length < b.length ? -1 : 1;
        return a < b ? -1 : a > b ? 1 : 0;
    }
    if (an)
        return -1;
    if (bn)
        return 1;
    return a < b ? -1 : a > b ? 1 : 0;
}
export function comparePrerelease(a, b) {
    if (!a.length && !b.length)
        return 0;
    if (!a.length)
        return 1;
    if (!b.length)
        return -1;
    for (let i = 0; i < Math.min(a.length, b.length); i++) {
        const c = compareId(a[i], b[i]);
        if (c)
            return c < 0 ? -1 : 1;
    }
    return a.length === b.length ? 0 : a.length < b.length ? -1 : 1;
}
/** SemVer 2.0.0 section 11 precedence; build metadata is ignored. */
export function compare(a, b) {
    if (a.major !== b.major)
        return a.major < b.major ? -1 : 1;
    if (a.minor !== b.minor)
        return a.minor < b.minor ? -1 : 1;
    if (a.patch !== b.patch)
        return a.patch < b.patch ? -1 : 1;
    return comparePrerelease(a.prerelease, b.prerelease);
}
export function checkPreid(preid) {
    if (!PRE_RE.test(preid) || preid.includes('.') || isNumeric(preid)) {
        throw new UsageError(`--preid harus satu identifier alfanumerik (mengandung huruf atau -): ${preid}`);
    }
}
/** Increments the prerelease list like npm: bump the last numeric identifier, else append 0; --preid resets the tag. */
function incrementPre(pre, preid) {
    let next = [...pre];
    if (!next.length) {
        next = ['0'];
    }
    else {
        let i = next.length;
        while (--i >= 0) {
            if (isNumeric(next[i])) {
                next[i] = String(BigInt(next[i]) + 1n);
                break;
            }
        }
        if (i < 0)
            next.push('0');
    }
    if (preid !== undefined) {
        // Same tag keeps its counter when it has one; a different tag restarts at 0.
        next = next[0] === preid && next.length > 1 && isNumeric(next[1]) ? next : [preid, '0'];
    }
    return next;
}
/** npm `semver.inc` semantics; the result never carries build metadata. */
export function bump(v, release, preid) {
    let { major, minor, patch } = v;
    let pre = [...v.prerelease];
    const wasPre = pre.length > 0;
    switch (release) {
        case 'major':
            if (minor !== 0 || patch !== 0 || !wasPre)
                major++;
            minor = patch = 0;
            pre = [];
            break;
        case 'minor':
            if (patch !== 0 || !wasPre)
                minor++;
            patch = 0;
            pre = [];
            break;
        case 'patch':
            if (!wasPre)
                patch++;
            pre = [];
            break;
        case 'premajor':
            major++;
            minor = patch = 0;
            pre = incrementPre([], preid);
            break;
        case 'preminor':
            minor++;
            patch = 0;
            pre = incrementPre([], preid);
            break;
        case 'prepatch':
            patch++;
            pre = incrementPre([], preid);
            break;
        case 'prerelease':
            if (!wasPre)
                patch++;
            pre = incrementPre(pre, preid);
            break;
    }
    for (const n of [major, minor, patch]) {
        if (!Number.isSafeInteger(n))
            throw new Error('Hasil bump melebihi bilangan bulat aman.');
    }
    return { major, minor, patch, prerelease: pre, build: [] };
}
//# sourceMappingURL=version.js.map