import { UsageError } from '../../errors.js';

/** Largest absolute timestamp (ms) representable by Date. */
export const MAX_MS = 8.64e15;

export type EpochUnit = 's' | 'ms';

const formatters = new Map<string, Intl.DateTimeFormat>();

function formatterFor(zone: string): Intl.DateTimeFormat {
    let f = formatters.get(zone);
    if (!f) {
        f = new Intl.DateTimeFormat('en-US', {
            timeZone: zone,
            hourCycle: 'h23',
            era: 'short',
            year: 'numeric',
            month: '2-digit',
            day: '2-digit',
            hour: '2-digit',
            minute: '2-digit',
            second: '2-digit',
        });
        formatters.set(zone, f);
    }
    return f;
}

/** Returns the zone name (system zone by default); throws UsageError for unknown zones. */
export function resolveZone(zone: string | undefined): string {
    if (zone === undefined) return Intl.DateTimeFormat().resolvedOptions().timeZone;
    try {
        new Intl.DateTimeFormat('en-US', { timeZone: zone });
        return zone;
    } catch (error) {
        throw new UsageError(`Zona waktu tidak valid: ${zone}`, { cause: error });
    }
}

interface Wall {
    year: number;
    month: number;
    day: number;
    hour: number;
    minute: number;
    second: number;
}

function wallAt(zone: string, ms: number): Wall {
    const w: Record<string, number> = {};
    let bc = false;
    for (const { type, value } of formatterFor(zone).formatToParts(ms)) {
        if (type === 'era') bc = /^b/i.test(value);
        else if (type !== 'literal') w[type] = Number(value);
    }
    // Proleptic Gregorian year numbering: 1 BC is year 0.
    return {
        year: bc ? 1 - w.year! : w.year!,
        month: w.month!,
        day: w.day!,
        hour: w.hour!,
        minute: w.minute!,
        second: w.second!,
    };
}

const DAY_MS = 86400000;

/** Days since 1970-01-01 of a proleptic Gregorian date (Hinnant's days_from_civil). */
function daysFromCivil(year: number, month: number, day: number): number {
    const y = month <= 2 ? year - 1 : year;
    const era = Math.floor(y / 400);
    const yoe = y - era * 400;
    const doy = Math.floor((153 * (month + (month > 2 ? -3 : 9)) + 2) / 5) + day - 1;
    const doe = yoe * 365 + Math.floor(yoe / 4) - Math.floor(yoe / 100) + doy;
    return era * 146097 + doe - 719468;
}

/** Wall fields read as UTC, in ms. Pure arithmetic: never builds a Date, so it may exceed the Date range. */
function wallToUtc(w: Wall): number {
    return daysFromCivil(w.year, w.month, w.day) * DAY_MS + ((w.hour * 60 + w.minute) * 60 + w.second) * 1000;
}

function daysInMonth(year: number, month: number): number {
    return (
        daysFromCivil(month === 12 ? year + 1 : year, month === 12 ? 1 : month + 1, 1) - daysFromCivil(year, month, 1)
    );
}

/** UTC offset of `zone` at instant `ms`, in milliseconds (whole seconds). */
export function offsetAt(zone: string, ms: number): number {
    return wallToUtc(wallAt(zone, ms)) - Math.floor(ms / 1000) * 1000;
}

/** Like offsetAt, but clamps the probe instant into the Date range. */
function probeOffset(zone: string, ms: number): number {
    return offsetAt(zone, Math.max(-MAX_MS, Math.min(MAX_MS, ms)));
}

function pad(n: number, width: number): string {
    return String(n).padStart(width, '0');
}

function formatYear(year: number): string {
    if (year >= 0 && year <= 9999) return pad(year, 4);
    return (year < 0 ? '-' : '+') + pad(Math.abs(year), 6);
}

function formatOffset(offsetMs: number): string {
    const sign = offsetMs < 0 ? '-' : '+';
    const total = Math.abs(offsetMs) / 1000;
    const h = Math.floor(total / 3600);
    const m = Math.floor((total % 3600) / 60);
    const s = total % 60;
    return `${sign}${pad(h, 2)}:${pad(m, 2)}${s ? ':' + pad(s, 2) : ''}`;
}

/** ISO-8601 with milliseconds and numeric offset, in the given zone. */
export function formatInZone(ms: number, zone: string): string {
    const w = wallAt(zone, ms);
    const millis = ((ms % 1000) + 1000) % 1000;
    return (
        `${formatYear(w.year)}-${pad(w.month, 2)}-${pad(w.day, 2)}T` +
        `${pad(w.hour, 2)}:${pad(w.minute, 2)}:${pad(w.second, 2)}.${pad(millis, 3)}` +
        formatOffset(offsetAt(zone, ms))
    );
}

export function checkRange(ms: number, what: string): number {
    if (!Number.isFinite(ms) || Math.abs(ms) > MAX_MS) {
        throw new UsageError(`${what} di luar rentang yang didukung (±8,64e15 ms): ${ms}`);
    }
    return ms;
}

const ISO =
    /^([+-]\d{6}|\d{4})-(\d{2})-(\d{2})(?:[T ](\d{2}):(\d{2})(?::(\d{2})(?:\.(\d{1,9}))?)?(Z|[+-]\d{2}(?::?\d{2})?)?)?$/i;

/** Parses an ISO-8601 date/datetime; wall times without an offset are read in `zone`. */
function parseIso(text: string, zone: string): number {
    const m = ISO.exec(text.trim());
    if (!m) throw new UsageError(`Format waktu tidak dikenali: ${text}. Pakai epoch atau ISO-8601.`);
    const [, y, mo, d, h = '0', mi = '0', s = '0', frac = '', tz] = m;
    const wall: Wall = {
        year: Number(y),
        month: Number(mo),
        day: Number(d),
        hour: Number(h),
        minute: Number(mi),
        second: Number(s),
    };
    if (y === '-000000') throw new UsageError(`Tahun tidak valid: ${text}`);
    const wallMs = wallToUtc(wall) + Number(frac.slice(0, 3).padEnd(3, '0'));
    // Reject overflowed components (month 13, Feb 30, hour 24, second 60...).
    if (
        wall.month < 1 ||
        wall.month > 12 ||
        wall.day < 1 ||
        wall.day > daysInMonth(wall.year, wall.month) ||
        wall.hour > 23 ||
        wall.minute > 59 ||
        wall.second > 59
    ) {
        throw new UsageError(`Tanggal atau jam tidak valid: ${text}`);
    }
    if (tz !== undefined) {
        if (tz.toUpperCase() === 'Z') return wallMs;
        const digits = tz.slice(1).replace(':', '');
        const oh = Number(digits.slice(0, 2));
        const om = Number(digits.slice(2) || '0');
        if (oh > 23 || om > 59) throw new UsageError(`Offset zona tidak valid: ${text}`);
        return wallMs - (tz[0] === '-' ? -1 : 1) * (oh * 60 + om) * 60000;
    }
    // Wall time in `zone`. Candidates use the offsets in force a day before and after; a candidate is valid when
    // the zone really has that offset at the resulting instant. Overlap (two valid): earlier instant.
    // Gap (none valid): the pre-transition offset, which shifts the time forward by the gap length.
    const before = probeOffset(zone, wallMs - DAY_MS);
    const after = probeOffset(zone, wallMs + DAY_MS);
    const valid = [before, after].map((o) => wallMs - o).filter((utc) => probeOffset(zone, utc) === wallMs - utc);
    return valid.length ? Math.min(...valid) : wallMs - before;
}

export function isEpoch(text: string): boolean {
    return /^-?\d+$/.test(text.trim());
}

/** Epoch number (seconds when abs < 1e11, else ms, unless `unit` is given) or ISO-8601 text. */
export function parseInstant(text: string, zone: string, unit?: EpochUnit): number {
    const trimmed = text.trim();
    if (!isEpoch(trimmed) && unit !== undefined) throw new UsageError(`--unit hanya berlaku untuk epoch: ${text}`);
    if (isEpoch(trimmed)) {
        const n = Number(trimmed);
        const u = unit ?? (Math.abs(n) < 1e11 ? 's' : 'ms');
        return checkRange(u === 's' ? n * 1000 : n, 'Epoch');
    }
    return checkRange(parseIso(trimmed, zone), 'Waktu');
}

/** Indonesian relative description of `ms` against `now`. */
export function relative(ms: number, now: number): string {
    const diff = ms - now;
    const abs = Math.abs(diff);
    if (abs < 1000) return 'sekarang';
    const seconds = Math.floor(abs / 1000);
    const minutes = Math.floor(seconds / 60);
    const hours = Math.floor(minutes / 60);
    const days = Math.floor(hours / 24);
    let text: string;
    if (seconds < 60) text = `${seconds} detik`;
    else if (minutes < 60) text = `${minutes} menit`;
    else if (hours < 24) text = `${hours} jam`;
    else if (days < 30) text = `${days} hari`;
    else if (days < 365) text = `${Math.floor(days / 30)} bulan`;
    else text = `${Math.floor(days / 365)} tahun`;
    return `${text} ${diff < 0 ? 'lalu' : 'lagi'}`;
}
