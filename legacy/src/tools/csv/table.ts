const segmenter = new Intl.Segmenter(undefined, { granularity: 'grapheme' });

const WIDE: readonly (readonly [number, number])[] = [
    [0x1100, 0x115f],
    [0x2e80, 0x303e],
    [0x3041, 0x33ff],
    [0x3400, 0x4dbf],
    [0x4e00, 0x9fff],
    [0xa000, 0xa4cf],
    [0xac00, 0xd7a3],
    [0xf900, 0xfaff],
    [0xfe30, 0xfe6f],
    [0xff00, 0xff60],
    [0xffe0, 0xffe6],
    [0x20000, 0x3fffd],
];

function codePointWidth(cp: number): number {
    if (cp < 0x20 || (cp >= 0x7f && cp < 0xa0)) return 0;
    if (
        (cp >= 0x300 && cp <= 0x36f) ||
        (cp >= 0x200b && cp <= 0x200f) ||
        (cp >= 0x20d0 && cp <= 0x20ff) ||
        (cp >= 0xfe00 && cp <= 0xfe0f)
    ) {
        return 0;
    }
    return WIDE.some(([lo, hi]) => cp >= lo && cp <= hi) ? 2 : 1;
}

const DEFAULT_EMOJI = /\p{Emoji_Presentation}/u;
const EMOJI = /[\p{Extended_Pictographic}\p{Emoji}]/u;

function graphemeWidth(grapheme: string): number {
    // VS15 requests text presentation; VS16 only widens an emoji-capable cluster.
    if (
        !grapheme.includes('\ufe0e') &&
        (DEFAULT_EMOJI.test(grapheme) || (grapheme.includes('\ufe0f') && EMOJI.test(grapheme)))
    ) {
        return 2;
    }
    let width = 0;
    for (const c of grapheme) width = Math.max(width, codePointWidth(c.codePointAt(0)!));
    return width;
}

/** Terminal columns taken by `text`, counting grapheme clusters (CJK and emoji are 2 wide). */
export function displayWidth(text: string): number {
    let width = 0;
    for (const { segment } of segmenter.segment(text)) width += graphemeWidth(segment);
    return width;
}

/** Cuts `text` to at most `max` columns, ending with an ellipsis when something was removed. */
export function truncate(text: string, max: number): string {
    if (displayWidth(text) <= max) return text;
    let out = '';
    let width = 0;
    for (const { segment } of segmenter.segment(text)) {
        const w = graphemeWidth(segment);
        if (width + w > max - 1) break;
        out += segment;
        width += w;
    }
    return `${out}…`;
}

/** Makes a cell safe for one-line display: line breaks become ↵, tab a space, other C0/C1 controls U+FFFD. */
export function sanitizeCell(cell: string): string {
    return cell
        .replace(/\r\n|[\r\n]/g, '↵')
        .replaceAll('\t', ' ')
        .replace(/[\u0000-\u001f\u007f-\u009f]/g, '\ufffd');
}

export function renderTable(header: string[], rows: string[][], maxWidth: number): string {
    const cells = [header, ...rows].map((r) => r.map((c) => truncate(sanitizeCell(c), maxWidth)));
    const widths = header.map(() => 1);
    for (const r of cells) widths.forEach((w, i) => (widths[i] = Math.max(w, displayWidth(r[i] ?? ''))));
    const last = widths.length - 1;
    const line = (r: string[]): string =>
        r.map((c, i) => (i === last ? c : c + ' '.repeat(widths[i]! - displayWidth(c)))).join(' | ');
    const separator = widths.map((w) => '-'.repeat(w)).join('-+-');
    const [head, ...body] = cells.map(line);
    return [head, separator, ...body].join('\n');
}
