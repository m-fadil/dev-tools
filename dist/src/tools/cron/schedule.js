import { UsageError } from '../../errors.js';
const MONTHS = ['jan', 'feb', 'mar', 'apr', 'may', 'jun', 'jul', 'aug', 'sep', 'oct', 'nov', 'dec'];
const WEEKDAYS = ['sun', 'mon', 'tue', 'wed', 'thu', 'fri', 'sat'];
const SPECS = {
    minute: { label: 'menit', min: 0, max: 59 },
    hour: { label: 'jam', min: 0, max: 23 },
    dom: { label: 'tanggal', min: 1, max: 31 },
    month: { label: 'bulan', min: 1, max: 12, names: MONTHS, nameBase: 1 },
    dow: { label: 'hari', min: 0, max: 7, names: WEEKDAYS, nameBase: 0 },
};
export const MACROS = {
    '@yearly': '0 0 1 1 *',
    '@annually': '0 0 1 1 *',
    '@monthly': '0 0 1 * *',
    '@weekly': '0 0 * * 0',
    '@daily': '0 0 * * *',
    '@midnight': '0 0 * * *',
    '@hourly': '0 * * * *',
};
function fieldError(spec, text, reason) {
    return new UsageError(`Field ${spec.label} tidak valid: "${text}" (${reason})`);
}
function parseValue(spec, field, token) {
    let n;
    if (/^\d+$/.test(token)) {
        n = Number(token);
    }
    else if (spec.names && /^[a-z]+$/i.test(token)) {
        const index = spec.names.indexOf(token.toLowerCase());
        if (index < 0)
            throw fieldError(spec, field, `nama tidak dikenal: ${token}`);
        n = index + (spec.nameBase ?? 0);
    }
    else {
        throw fieldError(spec, field, `nilai bukan angka${spec.names ? ' atau nama' : ''}: ${token || '(kosong)'}`);
    }
    if (n < spec.min || n > spec.max)
        throw fieldError(spec, field, `${n} di luar rentang ${spec.min}-${spec.max}`);
    return n;
}
function parseField(spec, field) {
    const set = new Set();
    // For weekdays `*` means 0-6 so that `*/n` does not count Sunday twice.
    const starMax = spec.max === 7 ? 6 : spec.max;
    for (const item of field.split(',')) {
        if (item === '')
            throw fieldError(spec, field, 'elemen kosong');
        const parts = item.split('/');
        if (parts.length > 2)
            throw fieldError(spec, field, `terlalu banyak "/" di "${item}"`);
        const base = parts[0] ?? '';
        const stepText = parts[1];
        let step = 1;
        if (stepText !== undefined) {
            if (!/^\d+$/.test(stepText) || Number(stepText) < 1) {
                throw fieldError(spec, field, `step harus bilangan bulat >= 1: ${stepText || '(kosong)'}`);
            }
            step = Number(stepText);
        }
        let lo;
        let hi;
        if (base === '*') {
            lo = spec.min;
            hi = starMax;
        }
        else if (base.includes('-')) {
            const range = base.split('-');
            if (range.length !== 2)
                throw fieldError(spec, field, `rentang salah: ${base}`);
            lo = parseValue(spec, field, range[0] ?? '');
            hi = parseValue(spec, field, range[1] ?? '');
            if (lo > hi)
                throw fieldError(spec, field, `rentang terbalik: ${base}`);
        }
        else {
            lo = parseValue(spec, field, base);
            // "a/n" means a through the maximum; a lone value is just itself.
            hi = stepText === undefined ? lo : spec.max;
        }
        for (let v = lo; v <= hi; v += step)
            set.add(spec.max === 7 && v === 7 ? 0 : v);
    }
    return { values: [...set].sort((a, b) => a - b), star: field.startsWith('*') };
}
/** Parses the cron expression tokens (a single macro or exactly five fields). */
export function parseCron(tokens) {
    if (tokens.length === 1 && tokens[0]?.startsWith('@')) {
        const expanded = MACROS[tokens[0].toLowerCase()];
        if (!expanded) {
            throw new UsageError(`Makro tidak dikenal: ${tokens[0]}. Pilihan: ${Object.keys(MACROS).join(', ')}`);
        }
        tokens = expanded.split(' ');
    }
    if (tokens.length !== 5) {
        throw new UsageError(`Ekspresi cron harus 5 field (menit jam tanggal bulan hari), diterima ${tokens.length}: ${tokens.join(' ')}`);
    }
    const [t0 = '', t1 = '', t2 = '', t3 = '', t4 = ''] = tokens;
    const minute = parseField(SPECS.minute, t0);
    const hour = parseField(SPECS.hour, t1);
    const dom = parseField(SPECS.dom, t2);
    const month = parseField(SPECS.month, t3);
    const dow = parseField(SPECS.dow, t4);
    return {
        minute: minute.values,
        hour: hour.values,
        dom: dom.values,
        month: month.values,
        dow: dow.values,
        domStar: dom.star,
        dowStar: dow.star,
    };
}
const MINUTE_MS = 60_000;
/** Search window per fire time, in years. */
export const HORIZON_YEARS = 8;
/** Wall-clock fields encoded as a UTC timestamp ("naive" time), so calendar math ignores time zones. */
function naiveOf(date, utc) {
    if (utc)
        return Math.floor(date.getTime() / MINUTE_MS) * MINUTE_MS;
    // Setters preserve years 0000-0099, unlike Date.UTC's 1900-based shortcut.
    const naive = new Date(0);
    naive.setUTCFullYear(date.getFullYear(), date.getMonth(), date.getDate());
    naive.setUTCHours(date.getHours(), date.getMinutes(), 0, 0);
    return naive.getTime();
}
function dayMatches(s, date) {
    const domOk = s.dom.includes(date.getUTCDate());
    const dowOk = s.dow.includes(date.getUTCDay());
    // Classic cron: when both day fields are restricted, either one matching is enough.
    return !s.domStar && !s.dowStar ? domOk || dowOk : domOk && dowOk;
}
/** First matching naive minute >= start and <= limit, jumping by month/day/hour instead of per minute. */
function searchNaive(s, start, limit) {
    let cur = start;
    while (cur <= limit) {
        const d = new Date(cur);
        const [y, mo, day, h, mi] = [
            d.getUTCFullYear(),
            d.getUTCMonth(),
            d.getUTCDate(),
            d.getUTCHours(),
            d.getUTCMinutes(),
        ];
        if (!s.month.includes(mo + 1)) {
            d.setUTCFullYear(y, mo + 1, 1);
            d.setUTCHours(0, 0, 0, 0);
        }
        else if (!dayMatches(s, d)) {
            d.setUTCDate(day + 1);
            d.setUTCHours(0, 0, 0, 0);
        }
        else if (!s.hour.includes(h)) {
            d.setUTCHours(h + 1, 0, 0, 0);
        }
        else {
            const next = s.minute.find((m) => m >= mi);
            d.setUTCHours(next === undefined ? h + 1 : h, next ?? 0, 0, 0);
            if (next !== undefined)
                return d.getTime();
        }
        cur = d.getTime();
    }
    return undefined;
}
/** Real instant for a naive wall time; undefined when that wall time does not exist (DST gap). */
function resolve(naive, utc) {
    if (utc)
        return new Date(naive);
    const n = new Date(naive);
    // Local setters pick the earlier overlap occurrence without remapping years 0000-0099.
    const date = new Date(0);
    date.setFullYear(n.getUTCFullYear(), n.getUTCMonth(), n.getUTCDate());
    date.setHours(n.getUTCHours(), n.getUTCMinutes(), 0, 0);
    return naiveOf(date, false) === naive ? date : undefined;
}
/**
 * Next `count` fire times strictly after `from`. `exhausted` is true when the search found no further
 * time within HORIZON_YEARS of the previous one (or of `from`).
 * DST: wall times skipped by a forward jump never fire; a repeated wall time fires once, at its first occurrence.
 */
export function nextFires(s, from, count, utc) {
    const times = [];
    let previous = from.getTime();
    let base = naiveOf(from, utc);
    let start = base + MINUTE_MS;
    while (times.length < count) {
        const limitDate = new Date(base);
        limitDate.setUTCFullYear(limitDate.getUTCFullYear() + HORIZON_YEARS);
        const limit = limitDate.getTime();
        let found;
        let naive;
        while (found === undefined) {
            naive = searchNaive(s, start, limit);
            if (naive === undefined)
                return { times, exhausted: true };
            start = naive + MINUTE_MS;
            const instant = resolve(naive, utc);
            if (instant && instant.getTime() > previous)
                found = instant;
        }
        times.push(found);
        previous = found.getTime();
        base = naive;
    }
    return { times, exhausted: false };
}
const pad = (n, width = 2) => String(n).padStart(width, '0');
/** ISO-8601 to the second: `Z` for UTC, otherwise numeric offset of the system time zone. */
export function formatTime(date, utc) {
    if (utc)
        return date.toISOString().replace(/\.\d{3}Z$/, 'Z');
    const offset = -Math.round(date.getTimezoneOffset());
    const sign = offset < 0 ? '-' : '+';
    const abs = Math.abs(offset);
    return (`${pad(date.getFullYear(), 4)}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}` +
        `T${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}` +
        `${sign}${pad(Math.floor(abs / 60))}:${pad(abs % 60)}`);
}
const ISO_RE = /^(\d{4})-(\d{2})-(\d{2})(?:[T ](\d{2}):(\d{2})(?::(\d{2})(?:\.\d{1,3})?)?(Z|[+-]\d{2}:\d{2})?)?$/i;
/** Strict ISO-8601 parser. Without an explicit offset the value is read in the chosen zone (UTC or local). */
export function parseFrom(text, utc) {
    const m = ISO_RE.exec(text);
    const bad = () => new UsageError(`Nilai --from bukan waktu ISO-8601 yang valid: ${text}`);
    if (!m)
        throw bad();
    const y = Number(m[1]);
    const mo = Number(m[2]);
    const d = Number(m[3]);
    const h = Number(m[4] ?? 0);
    const mi = Number(m[5] ?? 0);
    const sec = Number(m[6] ?? 0);
    // Date.UTC/new Date map years 0-99 to 19xx; setUTCFullYear/setFullYear do not.
    const check = new Date(0);
    check.setUTCFullYear(y, mo - 1, d);
    check.setUTCHours(h, mi, sec, 0);
    if (h > 23 ||
        mi > 59 ||
        sec > 59 ||
        check.getUTCFullYear() !== y ||
        check.getUTCMonth() !== mo - 1 ||
        check.getUTCDate() !== d) {
        throw bad();
    }
    if (m[7]) {
        const offset = /^[+-](\d{2}):(\d{2})$/.exec(m[7]);
        if (offset && (Number(offset[1]) > 23 || Number(offset[2]) > 59))
            throw bad();
        const ms = Date.parse(`${pad(y, 4)}-${m[2]}-${m[3]}T${pad(h)}:${pad(mi)}:${pad(sec)}${m[7].toUpperCase()}`);
        if (Number.isNaN(ms))
            throw bad();
        return new Date(ms);
    }
    if (utc)
        return check;
    const local = new Date(0);
    local.setFullYear(y, mo - 1, d);
    local.setHours(h, mi, sec, 0);
    return local;
}
const MONTH_NAMES = [
    'Januari',
    'Februari',
    'Maret',
    'April',
    'Mei',
    'Juni',
    'Juli',
    'Agustus',
    'September',
    'Oktober',
    'November',
    'Desember',
];
const DAY_NAMES = ['Minggu', 'Senin', 'Selasa', 'Rabu', 'Kamis', 'Jumat', 'Sabtu'];
function describe(label, values, total, every, names, base = 0) {
    if (values.length === total)
        return `${label}: ${every}`;
    return `${label}: ${values.map((v) => (names ? names[v - base] : v)).join(', ')}`;
}
/** Indonesian per-field explanation, one line each. */
export function explain(s) {
    const lines = [
        describe('menit', s.minute, 60, 'setiap menit'),
        describe('jam', s.hour, 24, 'setiap jam'),
        describe('tanggal', s.dom, 31, 'setiap tanggal'),
        describe('bulan', s.month, 12, 'setiap bulan', MONTH_NAMES, 1),
        describe('hari', s.dow, 7, 'setiap hari', DAY_NAMES),
    ];
    if (!s.domStar && !s.dowStar)
        lines.push('catatan: tanggal dan hari sama-sama dibatasi, cocok bila salah satunya terpenuhi (ATAU)');
    return lines;
}
//# sourceMappingURL=schedule.js.map