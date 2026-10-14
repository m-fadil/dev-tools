export const WORD = /[\p{L}\p{M}\p{N}]+(?:['’-][\p{L}\p{M}\p{N}]+)*/gu;
const segmenter = new Intl.Segmenter(undefined, { granularity: 'grapheme' });
export function graphemes(text) {
    return Array.from(segmenter.segment(text), (s) => s.segment);
}
/** Lines of the input; one trailing newline is not a line of its own. */
export function splitLines(text) {
    if (text === '')
        return [];
    return text.replace(/\r?\n$/, '').split(/\r?\n/);
}
/** Input without its single trailing newline (so `echo x | ...` equals `... x`). */
export function body(text) {
    return text.replace(/\r?\n$/, '');
}
export function words(text) {
    return text.match(WORD) ?? [];
}
function trailingNewlineBytes(buf) {
    if (buf.length && buf[buf.length - 1] === 0x0a)
        return buf.length > 1 && buf[buf.length - 2] === 0x0d ? 2 : 1;
    return 0;
}
/** Single pass: a maximal run of . ! ? … followed by whitespace/end closes a segment; segments with a letter/digit count. */
function countSentences(text) {
    const isEnd = (c) => c === '.' || c === '!' || c === '?' || c === '…';
    let count = 0;
    let hasContent = false;
    let i = 0;
    while (i < text.length) {
        if (isEnd(text[i])) {
            let j = i;
            while (j < text.length && isEnd(text[j]))
                j++;
            if (j === text.length || /\s/u.test(text[j])) {
                if (hasContent)
                    count++;
                hasContent = false;
            }
            i = j;
            continue;
        }
        if (!hasContent) {
            const cp = text.codePointAt(i);
            const ch = String.fromCodePoint(cp);
            if (/[\p{L}\p{N}]/u.test(ch))
                hasContent = true;
            i += ch.length;
        }
        else
            i++;
    }
    return count + (hasContent ? 1 : 0);
}
export function stats(raw, input) {
    const text = body(raw);
    const lines = splitLines(raw);
    const w = words(text);
    let longest = { number: 0, length: 0 };
    lines.forEach((line, i) => {
        const length = graphemes(line).length;
        if (length > longest.length)
            longest = { number: i + 1, length };
    });
    const letters = w.reduce((sum, word) => sum + graphemes(word).length, 0);
    // A sentence ends at . ! ? … followed by whitespace or the end, so "3.14" stays whole.
    const sentences = countSentences(text);
    return {
        characters: Array.from(text).length,
        graphemes: graphemes(text).length,
        bytes: input.length - trailingNewlineBytes(input),
        words: w.length,
        lines: lines.length,
        sentences,
        longestLine: longest,
        averageWordLength: w.length ? Math.round((letters / w.length) * 100) / 100 : 0,
    };
}
export function reverseText(raw, byLines) {
    const lines = splitLines(raw);
    return byLines ? lines.reverse() : lines.map((l) => graphemes(l).reverse().join(''));
}
/** Strip U+0300–U+036F diacritics, recompose, and lowercase; preserve other scripts' combining marks. */
export function slugify(text, sep) {
    const clean = text
        .replace(/['’]/g, '')
        .normalize('NFD')
        .replace(/[\u0300-\u036f]/g, '')
        .normalize('NFC')
        .toLowerCase();
    return clean
        .split(/[^\p{L}\p{M}\p{N}]+/u)
        .filter(Boolean)
        .join(sep);
}
export function frequency(text, o) {
    const counts = new Map();
    let total = 0;
    for (const raw of words(text)) {
        const word = o.caseSensitive ? raw : raw.toLowerCase();
        if (Array.from(word).length < o.minLength)
            continue;
        total++;
        counts.set(word, (counts.get(word) ?? 0) + 1);
    }
    const sorted = [...counts].sort((a, b) => b[1] - a[1] || (a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : 0));
    return { total, unique: counts.size, top: sorted.slice(0, o.top).map(([word, count]) => ({ word, count })) };
}
/** Order of operations: trim, unique (first occurrence wins), sort (code point order), reverse. */
export function processLines(raw, o) {
    let lines = splitLines(raw);
    if (o.trim)
        lines = lines.map((l) => l.trim());
    if (o.unique)
        lines = [...new Set(lines)];
    if (o.sort)
        lines.sort(compareCodePoints);
    if (o.reverse)
        lines.reverse();
    return lines;
}
/** Lexicographic Unicode code point order, including unpaired UTF-16 surrogates. */
function compareCodePoints(a, b) {
    let i = 0;
    let j = 0;
    while (i < a.length && j < b.length) {
        const x = a.codePointAt(i);
        const y = b.codePointAt(j);
        if (x !== y)
            return x - y;
        i += x > 0xffff ? 2 : 1;
        j += y > 0xffff ? 2 : 1;
    }
    return a.length - i - (b.length - j);
}
//# sourceMappingURL=analyze.js.map