/** Control characters 0-31 plus SP (32) and DEL (127): abbreviation and Indonesian description. */
const CONTROL = [
    ['NUL', 'Null: karakter kosong'],
    ['SOH', 'Start of Heading: awal header'],
    ['STX', 'Start of Text: awal teks'],
    ['ETX', 'End of Text: akhir teks'],
    ['EOT', 'End of Transmission: akhir transmisi'],
    ['ENQ', 'Enquiry: permintaan respons'],
    ['ACK', 'Acknowledge: konfirmasi diterima'],
    ['BEL', 'Bell: bunyi peringatan'],
    ['BS', 'Backspace: mundur satu karakter'],
    ['HT', 'Horizontal Tab: tab horizontal'],
    ['LF', 'Line Feed: baris baru'],
    ['VT', 'Vertical Tab: tab vertikal'],
    ['FF', 'Form Feed: ganti halaman'],
    ['CR', 'Carriage Return: kembali ke awal baris'],
    ['SO', 'Shift Out: ganti set karakter'],
    ['SI', 'Shift In: kembali ke set karakter semula'],
    ['DLE', 'Data Link Escape: escape tautan data'],
    ['DC1', 'Device Control 1 (XON): lanjutkan transmisi'],
    ['DC2', 'Device Control 2'],
    ['DC3', 'Device Control 3 (XOFF): hentikan transmisi'],
    ['DC4', 'Device Control 4'],
    ['NAK', 'Negative Acknowledge: konfirmasi ditolak'],
    ['SYN', 'Synchronous Idle: sinkronisasi'],
    ['ETB', 'End of Transmission Block: akhir blok transmisi'],
    ['CAN', 'Cancel: batalkan'],
    ['EM', 'End of Medium: akhir media'],
    ['SUB', 'Substitute: pengganti karakter rusak'],
    ['ESC', 'Escape: awal urutan escape'],
    ['FS', 'File Separator: pemisah file'],
    ['GS', 'Group Separator: pemisah grup'],
    ['RS', 'Record Separator: pemisah record'],
    ['US', 'Unit Separator: pemisah unit'],
];
/** Abbreviation and description for non-printing ASCII codes, otherwise undefined. */
export function controlInfo(code) {
    if (code === 32)
        return { name: 'SP', description: 'Space: spasi' };
    if (code === 127)
        return { name: 'DEL', description: 'Delete: hapus' };
    const entry = CONTROL[code];
    return entry && { name: entry[0], description: entry[1] };
}
export function describe(code) {
    const control = controlInfo(code);
    const char = String.fromCodePoint(code);
    const printable = !control && !/[\p{C}\p{Zl}\p{Zp}]/u.test(char);
    return {
        decimal: code,
        hex: '0x' + code.toString(16).toUpperCase().padStart(2, '0'),
        octal: code.toString(8).padStart(3, '0'),
        binary: code.toString(2).padStart(8, '0'),
        unicode: 'U+' + code.toString(16).toUpperCase().padStart(4, '0'),
        utf8: Array.from(Buffer.from(char, 'utf8'), (b) => b.toString(16).toUpperCase().padStart(2, '0')),
        ascii: code < 128,
        character: printable ? char : null,
        name: control?.name ?? null,
        description: control?.description ?? null,
    };
}
/** Value as a code point: one character, decimal (65) or hex (0x41). */
export function parseValue(value) {
    let code;
    if (/^0x[0-9a-f]+$/i.test(value))
        code = parseInt(value.slice(2), 16);
    else if (Array.from(value).length === 1)
        return checked(value.codePointAt(0), value);
    else if (/^\d+$/.test(value))
        code = Number(value);
    else
        throw new Error(`Bukan satu karakter, kode desimal, atau hex (0x..): ${JSON.stringify(value)}`);
    return checked(code, value);
}
function checked(code, value) {
    if (!Number.isSafeInteger(code) || code > 0x10ffff)
        throw new Error(`Kode di luar rentang Unicode (maks 0x10FFFF): ${value}`);
    if (code >= 0xd800 && code <= 0xdfff)
        throw new Error(`Kode surrogate (0xD800-0xDFFF) bukan karakter yang valid: ${value}`);
    return code;
}
export function table() {
    return Array.from({ length: 128 }, (_, i) => describe(i));
}
/** Cell label: the character, or the control abbreviation. */
export function label(info) {
    return info.name ?? info.character ?? '';
}
//# sourceMappingURL=chars.js.map