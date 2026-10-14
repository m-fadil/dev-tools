const NAMED_DELIMITERS = { comma: ',', semicolon: ';', tab: '\t', pipe: '|' };
const DETECT_ORDER = [',', ';', '\t', '|'];
/** Accepts a name (comma, semicolon, tab, pipe) or a single character. */
export function resolveDelimiter(value) {
    const named = NAMED_DELIMITERS[value];
    if (named !== undefined)
        return named;
    if (value.length !== 1 || value === '"' || value === '\n' || value === '\r')
        return undefined;
    return value;
}
export function stripBom(text) {
    return text.charCodeAt(0) === 0xfeff ? text.slice(1) : text;
}
/** Counts `delimiter` in the first record, quoting like the parser: a quote only opens at the start of a field. */
function countInFirstRecord(text, delimiter) {
    let count = 0;
    let quoted = false;
    let fieldStart = true;
    for (let i = 0; i < text.length; i++) {
        const c = text[i];
        if (quoted) {
            if (c !== '"')
                continue;
            if (text[i + 1] === '"')
                i++;
            else
                quoted = false;
            continue;
        }
        if (c === '\n' || c === '\r')
            break;
        if (c === '"' && fieldStart) {
            quoted = true;
            fieldStart = false;
        }
        else if (c === delimiter) {
            count++;
            fieldStart = true;
        }
        else {
            fieldStart = false;
        }
    }
    return count;
}
/** Guesses the delimiter by counting candidates outside quotes on the first record; comma on a tie or none. */
export function detectDelimiter(text) {
    const body = stripBom(text);
    let best = ',';
    let bestCount = 0;
    let tied = false;
    for (const d of DETECT_ORDER) {
        const count = countInFirstRecord(body, d);
        if (count > bestCount) {
            best = d;
            bestCount = count;
            tied = false;
        }
        else if (count > 0 && count === bestCount) {
            tied = true;
        }
    }
    return tied ? ',' : best;
}
/**
 * RFC 4180 parser. LF, CRLF and lone CR end a record; blank lines are skipped. A quote inside an
 * unquoted field is literal; anything between a closing quote and the next delimiter is an error.
 */
export function parseCsv(input, delimiter) {
    const text = stripBom(input);
    const records = [];
    const n = text.length;
    let fields = [];
    let row = 1;
    let i = 0;
    const endRecord = () => {
        records.push({ row, fields });
        fields = [];
        row++;
    };
    while (i < n) {
        const first = text[i];
        if (!fields.length && (first === '\n' || first === '\r')) {
            i += first === '\r' && text[i + 1] === '\n' ? 2 : 1;
            row++;
            continue;
        }
        let field = '';
        if (first === '"') {
            i++;
            for (;;) {
                const close = text.indexOf('"', i);
                if (close < 0)
                    throw new Error(`CSV tidak valid: tanda kutip tidak ditutup (mulai di baris ${row}).`);
                field += text.slice(i, close);
                i = close + 1;
                if (text[i] !== '"')
                    break;
                field += '"';
                i++;
            }
            const after = text[i];
            if (i < n && after !== delimiter && after !== '\n' && after !== '\r') {
                throw new Error(`CSV tidak valid pada baris ${row}: karakter ${JSON.stringify(after)} setelah tanda kutip penutup.`);
            }
        }
        else {
            const start = i;
            while (i < n && text[i] !== delimiter && text[i] !== '\n' && text[i] !== '\r')
                i++;
            field = text.slice(start, i);
        }
        fields.push(field);
        if (i >= n)
            break;
        if (text[i] === delimiter) {
            i++;
            if (i >= n)
                fields.push(''); // trailing delimiter at end of input
            else
                continue;
        }
        else {
            i += text[i] === '\r' && text[i + 1] === '\n' ? 2 : 1;
        }
        endRecord();
    }
    if (fields.length)
        endRecord();
    return records;
}
/** Serializes one record as RFC 4180; a lone empty field is quoted so the line is not mistaken for blank. */
export function toCsvLine(fields, delimiter) {
    if (fields.length === 1 && fields[0] === '')
        return '""';
    return fields
        .map((f) => (f.includes(delimiter) || /["\r\n]/.test(f) ? `"${f.replaceAll('"', '""')}"` : f))
        .join(delimiter);
}
//# sourceMappingURL=parse.js.map