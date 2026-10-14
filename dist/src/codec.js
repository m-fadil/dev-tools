export const CODECS = ['base64', 'base64url', 'hex', 'url'];
// Same unreserved set as encodeURIComponent, so text output is unchanged.
const URL_UNRESERVED = /[A-Za-z0-9\-_.!~*'()]/;
const isHexDigit = (byte) => byte !== undefined && ((byte >= 0x30 && byte <= 0x39) || ((byte | 0x20) >= 0x61 && (byte | 0x20) <= 0x66));
/** Percent-encodes raw bytes, so binary input survives (no UTF-8 round trip). */
function encodeUrl(data) {
    let out = '';
    for (const byte of data) {
        const char = String.fromCharCode(byte);
        out += byte < 0x80 && URL_UNRESERVED.test(char) ? char : `%${byte.toString(16).toUpperCase().padStart(2, '0')}`;
    }
    return out;
}
/** Percent-decodes into raw bytes; every `%` needs two hex digits, `+` stays literal. */
function decodeUrl(value) {
    const bytes = Buffer.from(value, 'utf8');
    const out = [];
    for (let i = 0; i < bytes.length; i++) {
        if (bytes[i] !== 0x25) {
            out.push(bytes[i]);
            continue;
        }
        if (!isHexDigit(bytes[i + 1]) || !isHexDigit(bytes[i + 2]))
            throw new Error('Input bukan URL-encoding yang valid.');
        out.push(parseInt(String.fromCharCode(bytes[i + 1], bytes[i + 2]), 16));
        i += 2;
    }
    return Buffer.from(out);
}
export function encode(data, codec) {
    switch (codec) {
        case 'base64':
        case 'base64url':
        case 'hex':
            return data.toString(codec);
        case 'url':
            return encodeUrl(data);
    }
}
/**
 * Strict decode: malformed input throws instead of being silently truncated.
 * base64/base64url/hex ignore all whitespace (wrapped lines, PEM-style bodies).
 */
export function decode(text, codec) {
    switch (codec) {
        case 'base64':
        case 'base64url': {
            const match = (codec === 'base64' ? /^([A-Za-z0-9+/]*)(={0,2})$/ : /^([A-Za-z0-9_-]*)(={0,2})$/).exec(text.replace(/\s+/g, ''));
            if (!match)
                throw new Error(`Input bukan ${codec} yang valid.`);
            const body = match[1];
            const padding = match[2];
            const rest = body.length % 4;
            // Padding is optional, but when present it must complete the final quantum exactly.
            if (rest === 1 || (padding && padding.length !== 4 - rest))
                throw new Error(`Input bukan ${codec} yang valid.`);
            const data = Buffer.from(body, codec);
            // Unused trailing bits must be zero: only the canonical encoding is accepted.
            if (data.toString(codec).replace(/=+$/, '') !== body)
                throw new Error(`Input bukan ${codec} yang valid.`);
            return data;
        }
        case 'hex': {
            const value = text.replace(/\s+/g, '');
            if (!/^(?:[0-9a-f]{2})*$/i.test(value))
                throw new Error('Input bukan hex yang valid.');
            return Buffer.from(value, 'hex');
        }
        case 'url':
            return decodeUrl(text.trim());
    }
}
//# sourceMappingURL=codec.js.map