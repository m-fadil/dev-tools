import { UsageError } from '../../errors.js';
/** Builds the table; rows whose width differs from the header (or the first row) are reported as ragged and padded. */
export function buildTable(records, hasHeader) {
    const head = hasHeader ? records[0] : undefined;
    const data = hasHeader ? records.slice(1) : records;
    const expected = (head ?? data[0])?.fields.length ?? 0;
    const ragged = data.filter((r) => r.fields.length !== expected).map((r) => ({ row: r.row, cols: r.fields.length }));
    // loop, not Math.max(...spread): spreading one argument per row overflows the stack on large input
    let count = expected;
    for (const r of data)
        if (r.fields.length > count)
            count = r.fields.length;
    const names = columnNames(head?.fields, count);
    const rows = data.map((r) => Array.from({ length: count }, (_, i) => r.fields[i] ?? ''));
    return { names, rows, expected, ragged };
}
/** Header names, padded to `count` with c<n>; a generated name never collides with an existing one. */
function columnNames(head, count) {
    const names = Array.from({ length: count }, (_, i) => head?.[i]);
    const used = new Set(head);
    for (let i = 0; i < count; i++) {
        if (names[i] !== undefined)
            continue;
        let n = i + 1;
        while (used.has(`c${n}`))
            n++;
        const name = `c${n}`;
        names[i] = name;
        used.add(name);
    }
    return names;
}
/** First duplicated column name, with its 1-based positions. */
export function findDuplicate(names) {
    for (let i = 0; i < names.length; i++) {
        const j = names.indexOf(names[i], i + 1);
        if (j >= 0)
            return `${JSON.stringify(names[i])} (kolom ${i + 1} dan ${j + 1})`;
    }
    return undefined;
}
/** Resolves --select tokens to 0-based column indexes: exact header name first, then 1-based index. */
export function resolveSelect(spec, names) {
    const picked = [];
    for (const raw of spec.split(',')) {
        const token = raw.trim();
        if (!token)
            throw new UsageError(`Nilai --select berisi kolom kosong: ${spec}`);
        let index = names.indexOf(raw);
        if (index < 0)
            index = names.indexOf(token);
        if (index < 0 && /^\d+$/.test(token) && Number(token) >= 1 && Number(token) <= names.length) {
            index = Number(token) - 1;
        }
        if (index < 0)
            throw new Error(`Kolom tidak ditemukan: ${token} (ada ${names.length} kolom).`);
        if (picked.includes(index))
            throw new UsageError(`Kolom dipilih dua kali pada --select: ${token}`);
        picked.push(index);
    }
    return picked;
}
//# sourceMappingURL=data.js.map