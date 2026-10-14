import { UsageError } from '../../errors.js';
import { NAMES, TRANSPARENT } from './names.js';
const clamp = (n, min, max) => {
    if (!Number.isFinite(n))
        throw new UsageError(`Komponen warna harus berupa angka hingga: ${n}`);
    return Math.min(max, Math.max(min, n));
};
const byte = (n) => clamp(Math.round(n), 0, 255);
// ---- parsing ----
const NUM = String.raw `[+-]?(?:\d+\.?\d*|\.\d+)`;
const NUM_RE = new RegExp(`^(${NUM})(%?)$`);
const HUE_RE = new RegExp(`^(${NUM})(deg|grad|rad|turn)?$`, 'i');
function bad(kind, token) {
    throw new UsageError(`${kind} tidak valid: ${token}`);
}
/** Number or percentage; `percentScale` is the value a percentage of 100% maps to. */
function component(token, percentScale, kind) {
    const m = NUM_RE.exec(token);
    if (!m)
        bad(kind, token);
    const n = Number(m[1]);
    if (!Number.isFinite(n))
        bad(kind, token.length > 40 ? `${token.slice(0, 40)}...` : token);
    const value = m[2] ? (n / 100) * percentScale : n;
    if (!Number.isFinite(value))
        bad(kind, token.length > 40 ? `${token.slice(0, 40)}...` : token);
    return { value, percent: Boolean(m[2]) };
}
function percentOnly(token, kind) {
    const c = component(token, 1, kind);
    if (!c.percent)
        throw new UsageError(`${kind} harus berupa persen (%): ${token}`);
    return clamp(c.value, 0, 1);
}
function alpha(token) {
    if (token === undefined)
        return 1;
    // Canonical alpha is shared by formatting, hex byte conversion, and name matching.
    return Math.round(clamp(component(token, 1, 'Alpha').value, 0, 1) * 1000) / 1000;
}
function hue(token) {
    const m = HUE_RE.exec(token);
    if (!m)
        bad('Hue', token);
    const n = Number(m[1]);
    if (!Number.isFinite(n))
        bad('Hue', token.length > 40 ? `${token.slice(0, 40)}...` : token);
    const unit = (m[2] ?? 'deg').toLowerCase();
    const degrees = unit === 'turn' ? n * 360 : unit === 'rad' ? (n * 180) / Math.PI : unit === 'grad' ? n * 0.9 : n;
    if (!Number.isFinite(degrees))
        bad('Hue', token.length > 40 ? `${token.slice(0, 40)}...` : token);
    return ((degrees % 360) + 360) % 360;
}
function hslToRgb(h, s, l) {
    const f = (n) => {
        const k = (n + h / 30) % 12;
        return l - s * Math.min(l, 1 - l) * Math.max(-1, Math.min(k - 3, 9 - k, 1));
    };
    return [f(0) * 255, f(8) * 255, f(4) * 255];
}
function hwbToRgb(h, w, bl) {
    if (w + bl >= 1) {
        const gray = (w / (w + bl)) * 255;
        return [gray, gray, gray];
    }
    return hslToRgb(h, 1, 0.5).map((c) => (c / 255) * (1 - w - bl) * 255 + w * 255);
}
function parseHex(digits, hasHash) {
    if (!/^[0-9a-f]+$/i.test(digits) || ![3, 4, 6, 8].includes(digits.length))
        bad('Warna hex', (hasHash ? '#' : '') + digits);
    if (!hasHash && digits.length !== 6 && digits.length !== 8) {
        throw new UsageError(`Hex tanpa '#' harus 6 atau 8 digit: ${digits}`);
    }
    const full = digits.length <= 4 ? [...digits].map((c) => c + c).join('') : digits;
    const [r, g, b, a] = [0, 2, 4, 6].map((i) => (full.length > i ? parseInt(full.slice(i, i + 2), 16) : 255));
    return { r, g, b, a: Math.round((a / 255) * 1000) / 1000 };
}
function parseFunction(fn, body, original) {
    const family = fn.toLowerCase().replace(/a$/, '');
    let parts;
    let legacy = false;
    let alphaToken;
    if (body.includes(',')) {
        legacy = true;
        if (family === 'hwb')
            throw new UsageError(`hwb() tidak mendukung koma; pakai spasi: ${original}`);
        if (body.includes('/'))
            throw new UsageError(`Sintaks campuran koma dan '/': ${original}`);
        const list = body.split(',').map((s) => s.trim());
        if (list.length !== 3 && list.length !== 4)
            throw new UsageError(`${fn}() butuh 3 atau 4 nilai: ${original}`);
        parts = list.slice(0, 3);
        alphaToken = list[3];
    }
    else {
        const slash = body.split('/');
        if (slash.length > 2)
            throw new UsageError(`Terlalu banyak '/': ${original}`);
        parts = slash[0].trim().split(/\s+/);
        if (slash.length === 2) {
            alphaToken = slash[1].trim();
            if (!alphaToken || /\s/.test(alphaToken))
                bad('Alpha', alphaToken || '(kosong)');
        }
        if (parts.length !== 3)
            throw new UsageError(`${fn}() butuh 3 nilai warna: ${original}`);
    }
    if (parts.some((p) => !p))
        throw new UsageError(`Nilai kosong: ${original}`);
    const a = alpha(alphaToken);
    if (family === 'rgb') {
        const cs = parts.map((p) => component(p, 255, 'Kanal RGB'));
        if (legacy && new Set(cs.map((c) => c.percent)).size > 1) {
            throw new UsageError(`Sintaks koma tidak boleh mencampur angka dan persen: ${original}`);
        }
        return { r: byte(cs[0].value), g: byte(cs[1].value), b: byte(cs[2].value), a };
    }
    const h = hue(parts[0]);
    const [r, g, b] = family === 'hsl'
        ? hslToRgb(h, percentOnly(parts[1], 'Saturasi'), percentOnly(parts[2], 'Lightness'))
        : hwbToRgb(h, percentOnly(parts[1], 'Whiteness'), percentOnly(parts[2], 'Blackness'));
    return { r: byte(r), g: byte(g), b: byte(b), a };
}
/** Parses hex (#rgb #rgba #rrggbb #rrggbbaa; '#' optional for 6/8 digits), rgb/hsl/hwb functions, and CSS names. */
export function parseColor(text) {
    const s = text.trim();
    if (!s)
        throw new UsageError('Warna kosong.');
    const lower = s.toLowerCase();
    if (lower === TRANSPARENT)
        return { r: 0, g: 0, b: 0, a: 0 };
    if (Object.hasOwn(NAMES, lower))
        return parseHex(NAMES[lower], false);
    if (s.startsWith('#'))
        return parseHex(s.slice(1), true);
    const fn = /^(rgba?|hsla?|hwb)\(\s*([^()]*?)\s*\)$/i.exec(s);
    if (fn)
        return parseFunction(fn[1], fn[2], s);
    if (/^[0-9a-f]+$/i.test(s))
        return parseHex(s, false);
    throw new UsageError(`Warna tidak dikenali: ${s}`);
}
// ---- conversion & formatting ----
export function toHsl({ r, g, b }) {
    const [R, G, B] = [r / 255, g / 255, b / 255];
    const max = Math.max(R, G, B);
    const min = Math.min(R, G, B);
    const d = max - min;
    const l = (max + min) / 2;
    const s = d === 0 ? 0 : d / (1 - Math.abs(2 * l - 1));
    return { h: Math.round(hueOf(R, G, B, max, d)) % 360, s: Math.round(s * 100), l: Math.round(l * 100) };
}
export function toHwb({ r, g, b }) {
    const [R, G, B] = [r / 255, g / 255, b / 255];
    const max = Math.max(R, G, B);
    const min = Math.min(R, G, B);
    return {
        h: Math.round(hueOf(R, G, B, max, max - min)) % 360,
        w: Math.round(min * 100),
        b: Math.round((1 - max) * 100),
    };
}
function hueOf(R, G, B, max, d) {
    if (d === 0)
        return 0;
    const sector = max === R ? ((G - B) / d) % 6 : max === G ? (B - R) / d + 2 : (R - G) / d + 4;
    return (sector * 60 + 360) % 360;
}
const hex2 = (n) => n.toString(16).padStart(2, '0');
export function toHex(c) {
    const base = `#${hex2(c.r)}${hex2(c.g)}${hex2(c.b)}`;
    return c.a < 1 ? base + hex2(Math.round(c.a * 255)) : base;
}
export function toRgbString(c) {
    return c.a < 1 ? `rgba(${c.r}, ${c.g}, ${c.b}, ${c.a})` : `rgb(${c.r}, ${c.g}, ${c.b})`;
}
export function toHslString(c) {
    const { h, s, l } = toHsl(c);
    return c.a < 1 ? `hsla(${h}, ${s}%, ${l}%, ${c.a})` : `hsl(${h}, ${s}%, ${l}%)`;
}
export function toHwbString(c) {
    const { h, w, b } = toHwb(c);
    return `hwb(${h} ${w}% ${b}%${c.a < 1 ? ` / ${c.a}` : ''})`;
}
// ---- names ----
const NAME_LIST = Object.entries(NAMES).map(([name, hex]) => ({
    name,
    r: parseInt(hex.slice(0, 2), 16),
    g: parseInt(hex.slice(2, 4), 16),
    b: parseInt(hex.slice(4, 6), 16),
}));
/** Exact CSS name (first alphabetically among synonyms) or the nearest by squared RGB distance; alpha is ignored. */
export function nameOf(c) {
    if (c.a === 0 && c.r === 0 && c.g === 0 && c.b === 0)
        return { name: TRANSPARENT, exact: true };
    let best = NAME_LIST[0];
    let bestDist = Infinity;
    for (const n of NAME_LIST) {
        const dist = (n.r - c.r) ** 2 + (n.g - c.g) ** 2 + (n.b - c.b) ** 2;
        if (dist < bestDist) {
            best = n;
            bestDist = dist;
        }
    }
    return { name: best.name, exact: bestDist === 0 };
}
// ---- WCAG 2.x ----
const linear = (v) => {
    const s = v / 255;
    return s <= 0.04045 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
};
/** Relative luminance (alpha ignored). */
export function luminance({ r, g, b }) {
    return 0.2126 * linear(r) + 0.7152 * linear(g) + 0.0722 * linear(b);
}
export function contrast(a, b) {
    const [l1, l2] = [luminance(a), luminance(b)];
    const ratio = (Math.max(l1, l2) + 0.05) / (Math.min(l1, l2) + 0.05);
    return { ratio, aaNormal: ratio >= 4.5, aaaNormal: ratio >= 7, aaLarge: ratio >= 3, aaaLarge: ratio >= 4.5 };
}
/** Ratio truncated (never rounded up) to 2 decimals so a displayed value never overstates a passing grade. */
export const ratioValue = (ratio) => Math.floor(ratio * 100 + 1e-9) / 100;
//# sourceMappingURL=color.js.map