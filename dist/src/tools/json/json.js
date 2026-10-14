import { UsageError } from '../../errors.js';
export class JsonSyntaxError extends Error {
    line;
    column;
    constructor(message, line, column) {
        super(`${line}:${column} ${message}`);
        this.line = line;
        this.column = column;
        this.name = 'JsonSyntaxError';
    }
}
function lineColumn(text, position) {
    let line = 1;
    let lineStart = 0;
    for (let i = text.indexOf('\n'); i !== -1 && i < position; i = text.indexOf('\n', i + 1)) {
        line++;
        lineStart = i + 1;
    }
    return { line, column: position - lineStart + 1 };
}
export const MAX_DEPTH = 2000;
export class JsonTooDeepError extends Error {
    constructor() {
        super(`JSON terlalu dalam (maks ${MAX_DEPTH} level)`);
    }
}
/** Index of the first character that makes `text` invalid JSON, or -1 when valid (text.length when cut short). */
function findErrorIndex(text) {
    let i = 0;
    const skipWs = () => {
        while (text[i] === ' ' || text[i] === '\t' || text[i] === '\n' || text[i] === '\r')
            i++;
    };
    const digits = () => {
        const start = i;
        while (text.charAt(i) >= '0' && text.charAt(i) <= '9')
            i++;
        return i > start;
    };
    const string = () => {
        i++; // opening quote
        for (;;) {
            if (i >= text.length)
                return false;
            const c = text.charAt(i);
            if (c === '"') {
                i++;
                return true;
            }
            if (c < ' ')
                return false;
            if (c === '\\') {
                i++;
                const e = text[i];
                if (e === 'u') {
                    for (let k = 1; k <= 4; k++)
                        if (!/[0-9a-fA-F]/.test(text[i + k] ?? ''))
                            return ((i += k), false);
                    i += 4;
                }
                else if (!e || !'"\\/bfnrt'.includes(e))
                    return false;
            }
            i++;
        }
    };
    const number = () => {
        if (text[i] === '-')
            i++;
        if (text[i] === '0')
            i++;
        else if (!digits())
            return false;
        if (text[i] === '.') {
            i++;
            if (!digits())
                return false;
        }
        if (text[i] === 'e' || text[i] === 'E') {
            i++;
            if (text[i] === '+' || text[i] === '-')
                i++;
            if (!digits())
                return false;
        }
        return true;
    };
    const states = ['end', 'value'];
    let depth = 0;
    while (states.length > 0) {
        skipWs();
        const state = states.pop();
        if (state === 'end')
            return i < text.length ? i : -1;
        const c = text.charAt(i);
        if (state === 'value') {
            if (c === '{' || c === '[') {
                if (++depth > MAX_DEPTH)
                    throw new JsonTooDeepError();
                i++;
                states.push(c === '{' ? 'objectFirst' : 'arrayFirst');
            }
            else if (c === '"') {
                if (!string())
                    return Math.min(i, text.length);
            }
            else if (c === '-' || (c >= '0' && c <= '9')) {
                if (!number())
                    return Math.min(i, text.length);
            }
            else {
                let matched = false;
                for (const literal of ['true', 'false', 'null']) {
                    if (text.startsWith(literal, i)) {
                        i += literal.length;
                        matched = true;
                        break;
                    }
                    let k = 0;
                    while (k < literal.length && text[i + k] === literal[k])
                        k++;
                    if (k > 0)
                        return Math.min(i + k, text.length);
                }
                if (!matched)
                    return Math.min(i, text.length);
            }
        }
        else if (state === 'arrayFirst') {
            if (c === ']') {
                i++;
                depth--;
            }
            else {
                states.push('arrayNext', 'value');
            }
        }
        else if (state === 'arrayNext') {
            if (c === ']') {
                i++;
                depth--;
            }
            else if (c === ',') {
                i++;
                states.push('arrayNext', 'value');
            }
            else {
                return Math.min(i, text.length);
            }
        }
        else if (state === 'objectFirst' || state === 'objectKey') {
            if (state === 'objectFirst' && c === '}') {
                i++;
                depth--;
            }
            else {
                if (c !== '"' || !string())
                    return Math.min(i, text.length);
                states.push('objectColon');
            }
        }
        else if (state === 'objectColon') {
            if (c !== ':')
                return Math.min(i, text.length);
            i++;
            states.push('objectNext', 'value');
        }
        else {
            if (c === '}') {
                i++;
                depth--;
            }
            else if (c === ',') {
                i++;
                states.push('objectKey');
            }
            else {
                return Math.min(i, text.length);
            }
        }
    }
    return -1;
}
/** JSON.parse with errors carrying line:column. Numbers beyond 2^53 lose precision. */
export function parseJson(text) {
    // A BOM is common in files saved on Windows and is not part of the document.
    if (text.charCodeAt(0) === 0xfeff)
        text = text.slice(1);
    // Scan iteratively before parsing so every mode has the same nesting limit.
    const position = findErrorIndex(text);
    try {
        return JSON.parse(text);
    }
    catch (error) {
        // V8 messages quote the input, so never mine them for the position; use the strict scanner.
        if (position === -1)
            throw error;
        const { line, column } = lineColumn(text, position);
        throw new JsonSyntaxError(error.message.replace(/ \(line \d+ column \d+\)$/, ''), line, column);
    }
}
/** Sorts object keys by UTF-16 code unit order without recursion; array order is kept. */
export function sortKeys(value) {
    if (value === null || typeof value !== 'object')
        return value;
    const source = value;
    // fromEntries defines own properties, so a "__proto__" key stays plain data.
    const result = Array.isArray(value)
        ? value.slice()
        : Object.fromEntries(Object.keys(source)
            .sort()
            .map((key) => [key, source[key]]));
    const pending = [{ source, target: result }];
    while (pending.length > 0) {
        const { source, target } = pending.pop();
        for (const key of Object.keys(source)) {
            const child = source[key];
            if (child === null || typeof child !== 'object')
                continue;
            const childSource = child;
            const childTarget = Array.isArray(child)
                ? child.slice()
                : Object.fromEntries(Object.keys(childSource)
                    .sort()
                    .map((name) => [name, childSource[name]]));
            target[key] = childTarget;
            pending.push({ source: childSource, target: childTarget });
        }
    }
    return result;
}
/** JSON.stringify that reports excessive nesting with a clear error instead of a raw RangeError. */
export function stringifyJson(data, indent) {
    try {
        return JSON.stringify(data, null, indent);
    }
    catch (error) {
        if (error instanceof RangeError)
            throw new JsonTooDeepError();
        throw error;
    }
}
/** Parses `a.b[0].c`, `$.a`, `.a`, `["key.with.dot"]` (also single quotes). Empty path / `$` is the root. */
export function parseQuery(path) {
    const fail = (at, why) => {
        throw new UsageError(`Path query tidak valid (posisi ${at + 1}): ${why}: ${path}`);
    };
    const segments = [];
    let i = 0;
    if (path[0] === '$')
        i = 1;
    if (i === path.length)
        return segments;
    let first = true;
    while (i < path.length) {
        const c = path[i];
        if (c === '[') {
            const q = path[i + 1];
            if (q === '"' || q === "'") {
                let j = i + 2;
                let key = '';
                for (; j < path.length && path[j] !== q; j++) {
                    if (path[j] === '\\' && j + 1 < path.length)
                        j++;
                    key += path[j];
                }
                if (path[j] !== q || path[j + 1] !== ']')
                    fail(i, 'kunci berkutip tidak ditutup dengan ]');
                segments.push(key);
                i = j + 2;
            }
            else {
                const end = path.indexOf(']', i);
                const index = end === -1 ? '' : path.slice(i + 1, end);
                if (!/^(0|[1-9]\d*)$/.test(index) || !Number.isSafeInteger(Number(index)))
                    fail(i, 'indeks harus bilangan bulat tak negatif tanpa nol di depan (maks 2^53-1) di dalam []');
                segments.push(Number(index));
                i = end + 1;
            }
        }
        else {
            // A leading name needs no dot; every other name must follow one.
            if (c === '.')
                i++;
            else if (!first)
                fail(i, 'diharapkan . atau [');
            let j = i;
            while (j < path.length && path[j] !== '.' && path[j] !== '[')
                j++;
            if (j === i) {
                // "." alone (or "$.") addresses the root
                if (first && i === path.length)
                    return segments;
                fail(i, 'nama kunci kosong');
            }
            segments.push(path.slice(i, j));
            i = j;
        }
        first = false;
    }
    return segments;
}
/** Walks `segments`; returns `{ found: false }` when any step is missing. Names only match objects, indexes arrays. */
export function getPath(root, segments) {
    let current = root;
    for (const segment of segments) {
        if (current === null || typeof current !== 'object')
            return { found: false };
        if (typeof segment === 'number') {
            if (!Array.isArray(current) || segment >= current.length)
                return { found: false };
            current = current[segment];
        }
        else {
            if (Array.isArray(current) || !Object.hasOwn(current, segment))
                return { found: false };
            current = current[segment];
        }
    }
    return { found: true, value: current };
}
//# sourceMappingURL=json.js.map