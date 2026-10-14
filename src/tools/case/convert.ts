export const STYLES = [
    'camel',
    'pascal',
    'snake',
    'screaming',
    'kebab',
    'train',
    'dot',
    'path',
    'title',
    'sentence',
    'lower',
    'upper',
] as const;
export type Style = (typeof STYLES)[number];

type Kind = 'lower' | 'upper' | 'digit';

function kindOf(ch: string): Kind {
    if (/\p{N}/u.test(ch)) return 'digit';
    if (/[\p{Lu}\p{Lt}]/u.test(ch)) return 'upper';
    return 'lower';
}

/**
 * Splits identifiers and prose into words. Separators are anything that is not a letter, mark or
 * number (apostrophes are dropped instead); boundaries are lower->Upper, letter<->digit and
 * the end of an acronym (HTTPServer -> HTTP, Server). Case is preserved.
 */
export function splitWords(text: string): string[] {
    const words: string[] = [];
    for (const chunk of text.replace(/['’]/g, '').split(/[^\p{L}\p{M}\p{N}]+/u)) {
        if (!chunk) continue;
        const chars = Array.from(chunk);
        let start = 0;
        for (let i = 1; i < chars.length; i++) {
            if (/\p{M}/u.test(chars[i]!)) continue; // combining marks stick to their base
            const prev = kindOf(chars[i - 1]!);
            const cur = kindOf(chars[i]!);
            const next = i + 1 < chars.length ? kindOf(chars[i + 1]!) : undefined;
            const boundary =
                (prev === 'digit') !== (cur === 'digit') ||
                (prev === 'lower' && cur === 'upper') ||
                (prev === 'upper' && cur === 'upper' && next === 'lower');
            if (boundary) {
                words.push(chars.slice(start, i).join(''));
                start = i;
            }
        }
        words.push(chars.slice(start).join(''));
    }
    return words;
}

function capitalize(word: string): string {
    const [first = '', ...rest] = Array.from(word.toLowerCase());
    return first.toUpperCase() + rest.join('');
}

export function convert(text: string, style: Style): string {
    const words = splitWords(text);
    const lower = words.map((w) => w.toLowerCase());
    switch (style) {
        case 'camel':
            return words.map((w, i) => (i === 0 ? w.toLowerCase() : capitalize(w))).join('');
        case 'pascal':
            return words.map(capitalize).join('');
        case 'snake':
            return lower.join('_');
        case 'screaming':
            return lower.join('_').toUpperCase();
        case 'kebab':
            return lower.join('-');
        case 'train':
            return words.map(capitalize).join('-');
        case 'dot':
            return lower.join('.');
        case 'path':
            return lower.join('/');
        case 'title':
            return words.map(capitalize).join(' ');
        case 'sentence':
            return words.map((w, i) => (i === 0 ? capitalize(w) : w.toLowerCase())).join(' ');
        case 'lower':
            return lower.join(' ');
        case 'upper':
            return lower.join(' ').toUpperCase();
    }
}

/** Lines of the input; one trailing newline is not a line of its own. */
export function splitLines(text: string): string[] {
    if (text === '') return [];
    return text.replace(/\r?\n$/, '').split(/\r?\n/);
}
