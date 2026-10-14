const LINE = /^(?:export\s+)?([A-Za-z_][A-Za-z0-9_.-]*)\s*=(.*)$/;
/** Parses the value part after `=`; undefined when malformed. No variable interpolation. */
function parseValue(rest) {
    const text = rest.trimStart();
    const quote = text[0];
    if (quote === '"' || quote === "'") {
        let value = '';
        let i = 1;
        for (; i < text.length; i++) {
            const c = text[i];
            if (c === quote)
                break;
            if (quote === '"' && c === '\\' && i + 1 < text.length) {
                const next = text[i + 1];
                if (next === 'n')
                    value += '\n';
                else if (next === '"' || next === '\\')
                    value += next;
                else
                    value += c + next;
                i++;
            }
            else {
                value += c;
            }
        }
        if (i >= text.length)
            return undefined; // unterminated
        const tail = text.slice(i + 1).trim();
        if (tail !== '' && !tail.startsWith('#'))
            return undefined;
        if (tail.startsWith('#') && !/\s/.test(text[i + 1] ?? ''))
            return undefined;
        return value;
    }
    // `rest` keeps leading whitespace so that `K= #c` is an empty value but `K=#fff` is not a comment.
    return rest.replace(/\s#.*$/, '').trim();
}
export function parseEnv(content) {
    const values = new Map();
    const firstLine = new Map();
    const invalid = [];
    const duplicates = [];
    const lines = content.replace(/^\uFEFF/, '').split(/\r?\n/);
    lines.forEach((raw, index) => {
        const line = raw.trim();
        if (line === '' || line.startsWith('#'))
            return;
        const match = LINE.exec(line);
        const value = match ? parseValue(match[2] ?? '') : undefined;
        const key = match?.[1];
        if (key === undefined || value === undefined) {
            invalid.push(index + 1);
            return;
        }
        const previous = firstLine.get(key);
        if (previous !== undefined)
            duplicates.push({ key, line: index + 1, previous });
        firstLine.set(key, index + 1);
        values.set(key, value);
    });
    return { values, invalid, duplicates };
}
export function diffEnv(a, b, keysOnly) {
    const onlyA = [];
    const onlyB = [];
    const changed = [];
    let same = 0;
    for (const [key, value] of a) {
        if (!b.has(key))
            onlyA.push(key);
        else if (!keysOnly && b.get(key) !== value)
            changed.push(key);
        else
            same++;
    }
    for (const key of b.keys())
        if (!a.has(key))
            onlyB.push(key);
    const sort = (keys) => keys.sort((x, y) => (x < y ? -1 : x > y ? 1 : 0));
    return { onlyA: sort(onlyA), onlyB: sort(onlyB), changed: sort(changed), same };
}
//# sourceMappingURL=envfile.js.map