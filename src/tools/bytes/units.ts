import { UsageError } from '../../errors.js';

export interface Unit {
    name: string;
    factor: bigint;
}

export const DECIMAL_UNITS: readonly Unit[] = ['KB', 'MB', 'GB', 'TB', 'PB'].map((name, i) => ({
    name,
    factor: 1000n ** BigInt(i + 1),
}));
export const BINARY_UNITS: readonly Unit[] = ['KiB', 'MiB', 'GiB', 'TiB', 'PiB'].map((name, i) => ({
    name,
    factor: 1024n ** BigInt(i + 1),
}));
export const BYTE_UNIT: Unit = { name: 'B', factor: 1n };

const BY_LOWER = new Map<string, Unit>(
    [BYTE_UNIT, ...DECIMAL_UNITS, ...BINARY_UNITS].map((u) => [u.name.toLowerCase(), u]),
);

/** Case-insensitive unit lookup; "kb" is 1000-based and "kib" 1024-based. */
export function findUnit(name: string): Unit | undefined {
    return BY_LOWER.get(name.toLowerCase());
}

export const UNIT_NAMES = [BYTE_UNIT, ...DECIMAL_UNITS, ...BINARY_UNITS].map((u) => u.name);

export interface ParsedSize {
    /** Rounded byte count (half up). */
    bytes: bigint;
    /** True when the exact size was not a whole number of bytes. */
    rounded: boolean;
}

/** Parses "1.5GB", "2 GiB", "1536" exactly (rational arithmetic on BigInt, no floats). */
export function parseSize(text: string): ParsedSize {
    const input = text.trim();
    if (input.startsWith('-')) throw new UsageError(`Ukuran tidak boleh negatif: ${input}`);
    const match = /^(\d+)(?:\.(\d+))?\s*([a-z]*)$/i.exec(input) ?? /^()\.(\d+)\s*([a-z]*)$/i.exec(input);
    if (!match) throw new UsageError(`Ukuran tidak valid: ${text}. Contoh: 1536, 1.5GB, 2 GiB`);
    const [, whole, fraction = '', unitName] = match;
    let unit = BYTE_UNIT;
    if (unitName) {
        const found = findUnit(unitName);
        if (!found) throw new UsageError(`Satuan tidak dikenal: ${unitName}. Pilihan: ${UNIT_NAMES.join(', ')}`);
        unit = found;
    }
    const numerator = BigInt((whole || '0') + fraction) * unit.factor;
    const denominator = 10n ** BigInt(fraction.length);
    const quotient = numerator / denominator;
    const remainder = numerator % denominator;
    if (remainder === 0n) return { bytes: quotient, rounded: false };
    return { bytes: remainder * 2n >= denominator ? quotient + 1n : quotient, rounded: true };
}

/** bytes / factor as a decimal string with up to `precision` fraction digits (half up, zeros trimmed). */
export function formatValue(bytes: bigint, factor: bigint, precision: number): string {
    const scale = 10n ** BigInt(precision);
    const scaled = (bytes * scale * 2n + factor) / (factor * 2n);
    const digits = scaled.toString().padStart(precision + 1, '0');
    const whole = digits.slice(0, digits.length - precision);
    const fraction = digits.slice(digits.length - precision).replace(/0+$/, '');
    return fraction ? `${whole}.${fraction}` : whole;
}

/** Largest unit with value >= 1, promoting if display rounding reaches the next unit. */
export function readable(bytes: bigint, units: readonly Unit[], precision: number): string {
    let unit = BYTE_UNIT;
    let index = -1;
    for (let i = 0; i < units.length; i++) {
        const candidate = units[i]!;
        if (bytes >= candidate.factor) {
            unit = candidate;
            index = i;
        }
    }
    let value = formatValue(bytes, unit.factor, precision);
    // Rounding can reach the unit base (999999 B -> "1000 KB"); promote to the next unit.
    const next = units[index + 1];
    if (next !== undefined && value === (next.factor / unit.factor).toString()) {
        unit = next;
        value = formatValue(bytes, unit.factor, precision);
    }
    return `${value} ${unit.name}`;
}
