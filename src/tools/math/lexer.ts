import { syntaxError } from './errors.js';

export type TokenKind = 'num' | 'id' | 'op' | 'end';

export interface Token {
    kind: TokenKind;
    text: string;
    /** 1-based column of the first character. */
    col: number;
    value: number;
}

const DIGITS = String.raw`\d+(?:_\d+)*`;
const DECIMAL = new RegExp(String.raw`(?:${DIGITS}(?:\.(?:${DIGITS})?)?|\.${DIGITS})(?:[eE][+-]?${DIGITS})?`, 'y');
const HEX = /0[xX][0-9a-fA-F]+(?:_[0-9a-fA-F]+)*/y;
const IDENT = /[A-Za-z_][A-Za-z0-9_]*/y;

function match(re: RegExp, src: string, pos: number): string | undefined {
    re.lastIndex = pos;
    return re.exec(src)?.[0];
}

/** Reads the token starting at `pos` (after whitespace). Lazy so the parser reports the first problem in order. */
export function lex(src: string, pos: number): { token: Token; next: number } {
    while (pos < src.length && (src[pos] === ' ' || src[pos] === '\t')) pos++;
    const col = pos + 1;
    if (pos >= src.length) return { token: { kind: 'end', text: '', col, value: 0 }, next: pos };

    const ch = src[pos]!;
    if (/[\d.]/.test(ch)) {
        let text: string | undefined;
        if (/^0[xX]/.test(src.slice(pos, pos + 2))) {
            text = match(HEX, src, pos);
            if (!text) throw syntaxError(src, col + 2, 'Digit heksadesimal diharapkan setelah 0x');
        } else {
            text = match(DECIMAL, src, pos);
            if (!text) throw syntaxError(src, col, "Titik harus diikuti digit, mis. '.5'");
        }
        const end = pos + text.length;
        if (src[end] === '_') {
            throw syntaxError(src, end + 1, "Pemisah '_' hanya boleh berada di antara dua digit");
        }
        const value = Number(text.replaceAll('_', ''));
        if (!Number.isFinite(value)) throw syntaxError(src, col, `Angka terlalu besar: ${text}`);
        return { token: { kind: 'num', text, col, value }, next: end };
    }
    const id = match(IDENT, src, pos);
    if (id) return { token: { kind: 'id', text: id, col, value: 0 }, next: pos + id.length };
    if (ch === '*' && src[pos + 1] === '*') {
        return { token: { kind: 'op', text: '**', col, value: 0 }, next: pos + 2 };
    }
    if ('+-*/%^!(),'.includes(ch)) return { token: { kind: 'op', text: ch, col, value: 0 }, next: pos + 1 };
    throw syntaxError(
        src,
        col,
        `Karakter tidak dikenal: ${JSON.stringify(String.fromCodePoint(src.codePointAt(pos)!))}`,
    );
}
