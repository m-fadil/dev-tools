import { UsageError } from '../../errors.js';
import { choice, parseToolArgs, printJson, readInput } from '../../io.js';
import { HMAC_ALGORITHMS, decodeJwt, describeClaim, mistypedClaims, normalizeToken, parseExpiry, parseNow, signJwt, verifyHmac, } from './jwt.js';
function help() {
    console.log(`dt jwt [options] [token]
dt jwt --sign -k <secret> [options] [payload-json]

Decode JWT: tampilkan header dan payload sebagai JSON, serta waktu exp/iat/nbf
(ISO-8601 UTC beserta statusnya). Token dari argumen, --file, atau stdin;
spasi di tepi dan awalan "Bearer " dibuang.
  -k, --key <secret>      Verifikasi signature HS256/HS384/HS512 (HMAC); dengan --sign: kunci penanda tangan
      --now <waktu>       ISO-8601 (T/spasi; tanpa zona = UTC) atau epoch detik
  -f, --file <path>       Baca token (atau payload dengan --sign) dari file
      --json              Keluaran satu dokumen JSON
  -h, --help              Tampilkan bantuan

Tanpa --key, signature TIDAK diverifikasi: isi token tidak boleh dipercaya.
Dengan --key, semuanya dicetak lebih dulu, lalu exit 1 jika signature tidak
valid, alg bukan HS* (termasuk "none"), token kedaluwarsa, belum berlaku,
atau exp/nbf bukan angka. Tanpa --key, exp/nbf bukan angka memberi peringatan
di stderr; token kedaluwarsa tetap exit 0. Token rusak: exit 1.

Mode tanda tangan (-s/--sign): payload (objek JSON) dari argumen, --file, atau
stdin; tercetak hanya token kompak. --key wajib.
  -s, --sign              Buat token bertanda tangan HMAC
      --alg <alg>         HS256 (default), HS384, atau HS512
      --exp <durasi|epoch> Set exp: 30s, 15m, 1h, 7d, 2w dari sekarang, atau epoch detik
                          (menimpa exp di payload)
      --iat               Set iat = sekarang (menimpa iat di payload)
Dengan --json: decode => {header, payload, times, signature, valid}; sign => {token, header, payload}.
Contoh: dt jwt eyJhbGciOi...   |   dt jwt -k rahasia < token.txt
        dt jwt -s -k rahasia --exp 1h --iat '{"sub":"42"}'`);
}
async function sign(values, positionals, now) {
    if (values.key === undefined)
        throw new UsageError('--sign memerlukan --key <secret>.');
    const alg = choice(values.alg, Object.keys(HMAC_ALGORITHMS), 'HS256', 'alg');
    let exp;
    try {
        if (values.exp !== undefined)
            exp = parseExpiry(values.exp, now);
    }
    catch (error) {
        throw new UsageError(error.message, { cause: error });
    }
    const text = (await readInput(positionals, values.file, 'Payload JSON:')).toString('utf8').trim();
    let parsed;
    try {
        parsed = JSON.parse(text);
    }
    catch (error) {
        throw new UsageError(`Payload bukan JSON yang valid: ${error.message}`, { cause: error });
    }
    if (typeof parsed !== 'object' || parsed === null || Array.isArray(parsed)) {
        throw new UsageError('Payload harus berupa objek JSON.');
    }
    const payload = { ...parsed };
    if (values.iat)
        payload.iat = Math.floor(now);
    if (exp !== undefined)
        payload.exp = exp;
    const header = { alg, typ: 'JWT' };
    const token = signJwt(header, payload, alg, values.key);
    if (values.json)
        printJson({ token, header, payload });
    else
        console.log(token);
}
async function run(args) {
    const { values, positionals } = parseToolArgs(args, {
        key: { type: 'string', short: 'k' },
        now: { type: 'string' },
        file: { type: 'string', short: 'f' },
        sign: { type: 'boolean', short: 's' },
        alg: { type: 'string' },
        exp: { type: 'string' },
        iat: { type: 'boolean' },
        json: { type: 'boolean' },
        help: { type: 'boolean', short: 'h' },
    });
    if (values.help) {
        help();
        return;
    }
    if (values.key === '')
        throw new UsageError('Nilai --key tidak boleh kosong.');
    if (!values.sign) {
        for (const option of ['alg', 'exp', 'iat']) {
            if (values[option] !== undefined)
                throw new UsageError(`--${option} hanya berlaku dengan --sign.`);
        }
    }
    let now;
    try {
        now = values.now === undefined ? Date.now() / 1000 : parseNow(values.now);
    }
    catch (error) {
        throw new UsageError(error.message, { cause: error });
    }
    if (values.sign) {
        await sign(values, positionals, now);
        return;
    }
    const json = values.json === true;
    const token = normalizeToken((await readInput(positionals, values.file, 'Token JWT:')).toString('utf8'));
    const decoded = decodeJwt(token);
    const claims = ['exp', 'iat', 'nbf'].flatMap((name) => {
        const info = describeClaim(name, decoded.payload[name], now);
        return info ? [{ name, ...info }] : [];
    });
    const mistyped = mistypedClaims(decoded.payload);
    const claimError = mistyped.length ? `Klaim ${mistyped.join(', ')} harus berupa angka.` : undefined;
    if (!json) {
        console.log(`Header:\n${JSON.stringify(decoded.header, null, 2)}\n`);
        console.log(`Payload:\n${JSON.stringify(decoded.payload, null, 2)}`);
        if (claims.length)
            console.log(`\nWaktu:\n${claims.map((claim) => `  ${claim.line}`).join('\n')}`);
        console.log('');
    }
    const times = claims.map((c) => ({ claim: c.name, iso: c.iso, status: c.status }));
    if (values.key === undefined) {
        if (claimError)
            console.error(`Peringatan: ${claimError}`);
        if (json)
            printJson({
                header: decoded.header,
                payload: decoded.payload,
                times,
                signature: { verified: null },
                valid: null,
            });
        else
            console.log('Signature: TIDAK diverifikasi (tanpa --key)');
        return;
    }
    const alg = decoded.header.alg;
    if (typeof alg !== 'string' || !Object.hasOwn(HMAC_ALGORITHMS, alg)) {
        if (json)
            printJson({
                header: decoded.header,
                payload: decoded.payload,
                times,
                signature: { verified: false },
                valid: false,
            });
        throw new Error(`Algoritma tidak didukung untuk --key: ${typeof alg === 'string' ? alg : '(tidak ada)'}. Hanya HS256, HS384, HS512.`);
    }
    const verified = verifyHmac(decoded, alg, values.key);
    const expired = claims.find((c) => c.name === 'exp')?.invalid === true;
    const early = claims.find((c) => c.name === 'nbf')?.invalid === true;
    if (json) {
        printJson({
            header: decoded.header,
            payload: decoded.payload,
            times,
            signature: { verified, algorithm: alg },
            valid: verified && !expired && !early && !claimError,
        });
    }
    else if (verified) {
        console.log('Signature: valid');
    }
    if (!verified)
        throw new Error('Signature tidak valid');
    if (expired)
        throw new Error('Token sudah kedaluwarsa.');
    if (early)
        throw new Error('Token belum berlaku.');
    if (claimError)
        throw new Error(claimError);
}
export const jwtTool = {
    name: 'jwt',
    description: 'Decode, verifikasi, dan buat JWT HMAC (HS256/384/512)',
    category: 'Keamanan & Kripto',
    run,
};
//# sourceMappingURL=index.js.map