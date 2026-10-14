import { createHmac, timingSafeEqual } from 'node:crypto';

export const HMAC_ALGORITHMS = { HS256: 'sha256', HS384: 'sha384', HS512: 'sha512' } as const;

export interface DecodedJwt {
    header: Record<string, unknown>;
    payload: Record<string, unknown>;
    /** the "header.payload" string the signature covers */
    signingInput: string;
    signature: Buffer;
}

/** Trims whitespace and one optional leading "Bearer " (case-insensitive). */
export function normalizeToken(text: string): string {
    return text.trim().replace(/^bearer\s+/i, '');
}

/** Strict base64url (no padding, no foreign characters, canonical encoding) to bytes. */
function base64urlBytes(segment: string, label: string): Buffer {
    const bytes = Buffer.from(segment, 'base64url');
    if (!/^[A-Za-z0-9_-]*$/.test(segment) || bytes.toString('base64url') !== segment) {
        throw new Error(`${label} bukan base64url yang valid.`);
    }
    return bytes;
}

function jsonObject(segment: string, label: string): Record<string, unknown> {
    const text = base64urlBytes(segment, label).toString('utf8');
    let value: unknown;
    try {
        value = JSON.parse(text);
    } catch {
        throw new Error(`${label} bukan JSON yang valid.`);
    }
    if (typeof value !== 'object' || value === null || Array.isArray(value)) {
        throw new Error(`${label} harus berupa objek JSON.`);
    }
    return value as Record<string, unknown>;
}

/** Splits and decodes a compact JWS token. Does not verify anything. */
export function decodeJwt(token: string): DecodedJwt {
    if (token === '') throw new Error('Token kosong.');
    const parts = token.split('.');
    if (parts.length !== 3)
        throw new Error(`Token harus terdiri dari 3 segmen dipisah titik, ditemukan ${parts.length}.`);
    const [h, p, s] = parts as [string, string, string];
    return {
        header: jsonObject(h, 'Header'),
        payload: jsonObject(p, 'Payload'),
        signingInput: `${h}.${p}`,
        signature: base64urlBytes(s, 'Signature'),
    };
}

/** Constant-time HMAC check; false for a signature of the wrong length. */
export function verifyHmac(decoded: DecodedJwt, alg: keyof typeof HMAC_ALGORITHMS, key: string): boolean {
    const expected = createHmac(HMAC_ALGORITHMS[alg], key).update(decoded.signingInput).digest();
    return expected.length === decoded.signature.length && timingSafeEqual(expected, decoded.signature);
}

const ISO_NOW =
    /^(\d{4})-(\d{2})-(\d{2})(?:[T ](\d{2}):(\d{2})(?::(\d{2})(\.\d+)?)?(?:(Z)|([+-])(\d{2}):?(\d{2}))?)?$/i;

/** ISO-8601 ("T" or space separator; zone-less means UTC) or epoch seconds (integer or decimal) to epoch seconds. */
export function parseNow(value: string): number {
    if (/^\d+(\.\d+)?$/.test(value)) return Number(value);
    const m = ISO_NOW.exec(value);
    if (m) {
        const [year, month, day, hour, minute, second, offsetHour, offsetMinute] = [1, 2, 3, 4, 5, 6, 10, 11].map((i) =>
            Number(m[i] ?? 0),
        );
        const date = new Date(0);
        date.setUTCFullYear(year!, month! - 1, day!);
        date.setUTCHours(hour!, minute!, second!, 0);
        // setUTC* rolls impossible dates over (Feb 30 -> Mar 2); require an exact round trip instead.
        const exact =
            date.getUTCFullYear() === year &&
            date.getUTCMonth() === month! - 1 &&
            date.getUTCDate() === day &&
            hour! < 24 &&
            minute! < 60 &&
            second! < 60 &&
            offsetHour! < 24 &&
            offsetMinute! < 60;
        if (exact) {
            const offset = (m[9] === '-' ? -1 : 1) * (offsetHour! * 3600 + offsetMinute! * 60);
            return date.getTime() / 1000 + Number(`0${m[7] ?? ''}`) - offset;
        }
    }
    throw new Error(`Nilai --now tidak valid: ${value}. Pakai ISO-8601 atau epoch detik.`);
}

function span(seconds: number): string {
    const s = Math.floor(Math.abs(seconds));
    if (s < 60) return `${s} detik`;
    if (s < 3600) return `${Math.floor(s / 60)} menit`;
    if (s < 86400) return `${Math.floor(s / 3600)} jam`;
    return `${Math.floor(s / 86400)} hari`;
}

export type ClaimName = 'exp' | 'iat' | 'nbf';

export interface ClaimInfo {
    line: string;
    /** null when the timestamp is outside the representable Date range */
    iso: string | null;
    status: string;
    /** exp passed or nbf not reached yet */
    invalid: boolean;
}

/** Describes a numeric time claim relative to `now` (epoch seconds); undefined if not a number. ±Infinity compare numerically. */
export function describeClaim(name: ClaimName, value: unknown, now: number): ClaimInfo | undefined {
    if (typeof value !== 'number' || Number.isNaN(value)) return undefined;
    const finite = Number.isFinite(value);
    const date = new Date(value * 1000);
    const iso = finite && !Number.isNaN(date.getTime()) ? date.toISOString() : null;
    const diff = value - now;
    // An infinite claim has no meaningful distance, so it gets a fixed description instead of a span.
    const distance = finite ? span(diff) : '';
    let status: string;
    let invalid = false;
    if (name === 'exp') {
        invalid = value <= now;
        if (!finite) status = invalid ? 'kedaluwarsa' : 'tidak pernah kedaluwarsa';
        else status = invalid ? `kedaluwarsa ${distance} lalu` : `berlaku ${distance} lagi`;
    } else if (name === 'nbf') {
        invalid = value > now;
        if (!finite) status = invalid ? 'belum berlaku' : 'sudah berlaku';
        else status = invalid ? `belum berlaku, ${distance} lagi` : 'sudah berlaku';
    } else if (!finite) {
        status = diff > 0 ? 'di masa depan' : 'di masa lalu';
    } else {
        status = diff > 0 ? `${distance} lagi (di masa depan)` : `${distance} lalu`;
    }
    return { line: `${name}: ${iso ?? 'di luar rentang waktu'} (${status})`, iso, status, invalid };
}

/** Time claims that are present but not numbers (null included); with --key such a token is invalid. */
export function mistypedClaims(payload: Record<string, unknown>): ClaimName[] {
    return (['exp', 'nbf'] as const).filter(
        (name) => Object.hasOwn(payload, name) && typeof payload[name] !== 'number',
    );
}

const DURATION_UNITS = { s: 1, m: 60, h: 3600, d: 86400, w: 604800 } as const;

/** Strict duration (`30s`, `15m`, `1h`, `7d`, `2w`) added to `now`, or an absolute epoch-seconds integer. */
export function parseExpiry(value: string, now: number): number {
    const duration = /^(\d+)([smhdw])$/.exec(value);
    let result: number;
    if (duration)
        result = Math.floor(now) + Number(duration[1]) * DURATION_UNITS[duration[2] as keyof typeof DURATION_UNITS];
    else if (/^\d+$/.test(value)) result = Number(value);
    else throw new Error(`Nilai --exp tidak valid: ${value}. Pakai durasi (30s, 15m, 1h, 7d, 2w) atau epoch detik.`);
    if (!Number.isSafeInteger(result)) throw new Error(`Nilai --exp terlalu besar: ${value}`);
    return result;
}

/** Builds a compact HMAC-signed JWS from a header object and payload object. */
export function signJwt(
    header: Record<string, unknown>,
    payload: Record<string, unknown>,
    alg: keyof typeof HMAC_ALGORITHMS,
    key: string,
): string {
    const encode = (value: unknown) => Buffer.from(JSON.stringify(value), 'utf8').toString('base64url');
    const signingInput = `${encode(header)}.${encode(payload)}`;
    return `${signingInput}.${createHmac(HMAC_ALGORITHMS[alg], key).update(signingInput).digest('base64url')}`;
}
