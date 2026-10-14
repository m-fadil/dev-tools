import { UsageError } from '../../errors.js';
import { integer, parseToolArgs, printJson, readInput } from '../../io.js';
import { evaluate, MAX_DEPTH, MAX_LENGTH } from './evaluate.js';
import { formatNumber } from './format.js';
function help() {
    console.log(`dt math [options] <ekspresi...>

Hitung ekspresi matematika. Beberapa argumen digabung dengan spasi; tanpa argumen,
ekspresi dibaca dari stdin. Teks (argumen atau stdin) dipecah per baris menjadi
ekspresi terpisah (baris kosong dilewati), hasilnya satu baris per ekspresi. 'ans' = hasil baris sebelumnya.
  -p, --precision <n>  Jumlah desimal tetap, 0-15 (default: 12 digit signifikan)
      --scientific     Tampilkan dalam notasi ilmiah (dengan -p: desimal mantissa)
      --degrees        sin/cos/tan menerima derajat; asin/acos/atan/atan2 mengembalikan derajat
      --json           Keluaran JSON: {results: [{expression, result}]}
  -h, --help           Tampilkan bantuan

Angka: 12, 1.5, .5, 1e-3, 0xff, 1_000 (literal melampaui batas double, mis. 1e5000, = galat sintaks; 1e-5000 menjadi 0;
  heksadesimal di atas 2^53 kehilangan presisi). Operator: + - * / % ^ ** (pangkat, kanan-asosiatif,
-2^2 = -4), ! (faktorial 0-170 setelah operand), kurung. Perkalian implisit (2pi, 2(3))
tidak didukung; tulis 2*pi. % memakai tanda pembagi seperti JavaScript (-7 % 3 = -1).
Konstanta: pi e tau inf ans
Fungsi: sqrt cbrt abs sign floor ceil round trunc exp ln log(basis 10) log2 log10
  sin cos tan asin acos atan atan2 sinh cosh tanh min max pow hypot mod gcd lcm deg rad
  (round: setengah menjauhi nol; deg: radian->derajat; rad: derajat->radian)
Batas: ekspresi maks ${MAX_LENGTH} karakter, kedalaman maks ${MAX_DEPTH}.
Ekspresi diawali tanda minus perlu '--' agar tidak dibaca sebagai opsi.
Exit: 0 sukses, 1 galat hitung (bagi nol, sqrt negatif, hasil NaN/tak berhingga), 2 sintaks/penggunaan salah.
Contoh: dt math "2 * (3 + 4)^2"   |   dt math --degrees "sin(30)"   |   dt math -- -2^2`);
}
async function run(args) {
    const { values, positionals } = parseToolArgs(args, {
        precision: { type: 'string', short: 'p' },
        scientific: { type: 'boolean' },
        degrees: { type: 'boolean' },
        json: { type: 'boolean' },
        help: { type: 'boolean', short: 'h' },
    });
    if (values.help) {
        help();
        return;
    }
    if (values.json && (values.precision !== undefined || values.scientific)) {
        throw new UsageError('--json tidak dapat digabung dengan --precision atau --scientific.');
    }
    const precision = integer(values.precision, -1, 0, 15, 'precision');
    const format = { precision: precision < 0 ? undefined : precision, scientific: values.scientific ?? false };
    const text = (await readInput(positionals, undefined, 'Ekspresi:')).toString('utf8');
    const lines = text
        .split(/\r?\n/)
        .map((line, index) => ({ expression: line.trim(), number: index + 1 }))
        .filter((line) => line.expression);
    if (!lines.length)
        throw new UsageError('Ekspresi wajib diisi, mis. dt math "1 + 2".');
    const results = [];
    let ans;
    for (const { expression, number } of lines) {
        let value;
        try {
            value = evaluate(expression, { degrees: values.degrees ?? false, ans });
        }
        catch (error) {
            if (lines.length === 1 || !(error instanceof Error))
                throw error;
            const message = `baris ${number}: ${error.message}`;
            throw error instanceof UsageError
                ? new UsageError(message, { cause: error })
                : new Error(message, { cause: error });
        }
        ans = value;
        const formatted = formatNumber(value, format);
        if (values.json) {
            results.push({ expression, result: Number.isFinite(value) ? Number(formatted) : formatted });
        }
        else {
            console.log(formatted);
        }
    }
    if (values.json)
        printJson({ results });
}
export const mathTool = {
    name: 'math',
    description: 'Hitung ekspresi matematika (operator, fungsi, konstanta)',
    category: 'Matematika',
    run,
};
//# sourceMappingURL=index.js.map