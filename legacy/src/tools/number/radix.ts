import { UsageError } from '../../errors.js';

export type Base = 2 | 8 | 10 | 16;
export const BASES: readonly Base[] = [2, 8, 10, 16];
export const WIDTHS = [8, 16, 32, 64] as const;

const PREFIX_BASE: Record<string, Base | undefined> = { x: 16, b: 2, o: 8 };
const BASE_PREFIX: Record<Base, string> = { 2: '0b', 8: '0o', 10: '', 16: '0x' };
const DIGITS: Record<Base, RegExp> = {
    2: /^[01]+$/,
    8: /^[0-7]+$/,
    10: /^\d+$/,
    16: /^[0-9a-f]+$/i,
};

export interface ParsedNumber {
    value: bigint;
    /** Base actually used: from the prefix, or the explicit --from. */
    base: Base;
}

/** Parses [sign][0x|0b|0o]digits with optional single `_` separators between digits. */
export function parseNumber(text: string, from: Base | 'auto'): ParsedNumber {
    let rest = text.trim();
    let negative = false;
    if (rest[0] === '-' || rest[0] === '+') {
        negative = rest[0] === '-';
        rest = rest.slice(1);
    }
    let base: Base = from === 'auto' ? 10 : from;
    const prefixBase = PREFIX_BASE[/^0([xbo])/i.exec(rest)?.[1]?.toLowerCase() ?? ''];
    // With --from 16, "0b1" is plain hex digits, not a binary prefix.
    if (prefixBase && (from === 'auto' || prefixBase === from)) {
        base = prefixBase;
        rest = rest.slice(2);
    }
    if (!rest) throw new UsageError(`Angka tidak valid: ${text.trim() || '(kosong)'}`);
    if (/^_|_$|__/.test(rest)) throw new UsageError(`Pemisah _ hanya boleh di antara digit: ${text.trim()}`);
    const digits = rest.replaceAll('_', '');
    if (!DIGITS[base].test(digits)) throw new UsageError(`Angka tidak valid untuk basis ${base}: ${text.trim()}`);
    const magnitude = BigInt(`${BASE_PREFIX[base] || ''}${digits}`);
    return { value: negative ? -magnitude : magnitude, base };
}

function group(digits: string, size: number): string {
    const padded = digits.padStart(Math.ceil(digits.length / size) * size, '0');
    return padded.match(new RegExp(`.{${size}}`, 'g'))!.join('_');
}

export interface FormatOptions {
    prefix?: boolean;
    group?: boolean;
    /** Zero-pad digits to this count (two's complement). */
    pad?: number;
}

/** Formats a value in `base`; negatives become -0x.. (sign before prefix). Grouping: binary by nibble, hex by byte. */
export function formatBase(value: bigint, base: Base, options: FormatOptions = {}): string {
    let digits = (value < 0n ? -value : value).toString(base);
    if (options.pad) digits = digits.padStart(options.pad, '0');
    if (options.group && base === 2) digits = group(digits, 4);
    else if (options.group && base === 16) digits = group(digits, 2);
    return `${value < 0n ? '-' : ''}${options.prefix ? BASE_PREFIX[base] : ''}${digits}`;
}

/** Two's complement bit pattern of `value` for `width` bits; accepts the signed range and the unsigned range. */
export function twosComplement(value: bigint, width: number): bigint {
    const w = BigInt(width);
    const min = -(1n << (w - 1n));
    const max = (1n << w) - 1n;
    if (value < min || value > max) {
        throw new UsageError(`Nilai ${value} tidak muat dalam ${width} bit (${min} sampai ${max}).`);
    }
    return value < 0n ? (1n << w) + value : value;
}
