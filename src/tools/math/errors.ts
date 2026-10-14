import { UsageError } from '../../errors.js';

/** Raised by function/operator implementations; the evaluator attaches the source column. */
export class DomainError extends Error {
    override name = 'DomainError';
}

/** Control characters would break the layout of the echoed source; show them as escapes. */
function escape(text: string): string {
    return text.replace(/[\p{Cc}\p{Zl}\p{Zp}]/gu, (ch) => {
        if (ch === '\t') return '\\t';
        if (ch === '\n') return '\\n';
        if (ch === '\r') return '\\r';
        return `\\u{${ch.codePointAt(0)!.toString(16)}}`;
    });
}

function locate(src: string, col: number): string {
    // Echoing a very long expression would bury the message.
    if (src.length > 200) return '';
    // col counts UTF-16 units; the caret is placed by code points of the escaped prefix.
    const pad = [...escape(src.slice(0, col - 1))].length;
    return `\n  ${escape(src)}\n  ${' '.repeat(pad)}^`;
}

/** Malformed expression (exit 2). */
export function syntaxError(src: string, col: number, message: string): UsageError {
    return new UsageError(`${message} (kolom ${col})${locate(src, col)}`);
}

/** Well-formed expression that cannot be computed (exit 1). */
export function domainError(src: string, col: number, message: string): Error {
    return new Error(`${message} (kolom ${col})${locate(src, col)}`);
}
