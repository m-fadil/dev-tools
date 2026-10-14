import { UsageError } from '../../errors.js';
import { choice, integer, parseToolArgs, printJson } from '../../io.js';
import { ONLY_SETS, SYMBOLS, characterClasses, entropyBits, generatePassword } from './generate.js';
function help() {
    console.log(`dt password [options]

Buat password acak dari CSPRNG (node:crypto). Default: huruf kecil, huruf besar,
dan angka; tiap jenis karakter aktif dijamin muncul minimal satu kali.
  -l, --length <n>        Panjang password, 4-1024 (default: 20)
  -c, --count <n>         Jumlah password, 1-1000 (default: 1), satu per baris
  -s, --symbols           Tambahkan simbol ${SYMBOLS}
      --only <set>        Batasi karakter: ${ONLY_SETS.join(', ')}
  -u, --unambiguous       Buang karakter yang mirip: 0 O 1 l I |
  -e, --entropy           Tampilkan estimasi entropi ke stderr
      --json              Keluaran JSON: {values, entropyBits?} (entropi masuk ke JSON, bukan stderr)
  -h, --help              Tampilkan bantuan

--only tidak bisa dipakai bersama --symbols. alnum = huruf besar/kecil + angka;
hex = 0-9a-f (tanpa jaminan jenis karakter). --length harus minimal sebanyak
jenis karakter aktif. Entropi (panjang * log2 ukuran alfabet) adalah batas
atas: jaminan satu karakter per jenis menguranginya sedikit.
Contoh: dt password -s -l 32   |   dt password -c 5 --only alnum -u -e`);
}
async function run(args) {
    const { values, positionals } = parseToolArgs(args, {
        length: { type: 'string', short: 'l' },
        count: { type: 'string', short: 'c' },
        symbols: { type: 'boolean', short: 's' },
        only: { type: 'string' },
        unambiguous: { type: 'boolean', short: 'u' },
        entropy: { type: 'boolean', short: 'e' },
        json: { type: 'boolean' },
        help: { type: 'boolean', short: 'h' },
    });
    if (values.help) {
        help();
        return;
    }
    if (positionals.length)
        throw new UsageError(`Argumen tidak dikenal: ${positionals[0]}`);
    const length = integer(values.length, 20, 4, 1024, 'length');
    const count = integer(values.count, 1, 1, 1000, 'count');
    const only = values.only === undefined ? undefined : choice(values.only, ONLY_SETS, 'alnum', 'only');
    const classes = characterClasses({
        symbols: values.symbols ?? false,
        only,
        unambiguous: values.unambiguous ?? false,
    });
    const passwords = Array.from({ length: count }, () => generatePassword(classes, length));
    if (values.json) {
        printJson(values.entropy ? { values: passwords, entropyBits: entropyBits(classes, length) } : { values: passwords });
        return;
    }
    console.log(passwords.join('\n'));
    if (values.entropy)
        console.error(`Entropi: ~${Math.round(entropyBits(classes, length))} bit`);
}
export const passwordTool = {
    name: 'password',
    description: 'Buat password acak yang aman secara kriptografis',
    category: 'Keamanan & Kripto',
    aliases: ['pw'],
    run,
};
//# sourceMappingURL=index.js.map