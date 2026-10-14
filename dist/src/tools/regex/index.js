import { UsageError } from '../../errors.js';
import { integer, parseToolArgs, printJson, readInput } from '../../io.js';
import { checkFlags } from './match.js';
import { matchInWorker } from './runner.js';
function help() {
    console.log(`dt regex [options] <pattern> [teks...]

Cari semua kecocokan regex JavaScript pada teks. Teks dari argumen setelah
pattern, --file, atau stdin (dipakai apa adanya, newline akhir ikut).
  -f, --flags <flags>     Flag regex: d i m s u v y (g selalu aktif secara internal)
  -r, --replace <templ>   Cetak juga hasil penggantian ($1, $<nama>, $&, $$)
      --first             Berhenti di kecocokan pertama (juga membatasi --replace)
      --test              Tanpa keluaran; exit 0 bila cocok, 1 bila tidak
      --timeout <ms>      Batas waktu pencocokan, 1-60000 (default: 2000)
      --json              Keluaran JSON tunggal
      --file <path>       Baca teks dari berkas
  -h, --help              Tampilkan bantuan

Setiap kecocokan: #n index=<posisi> "<teks>", lalu grup bernomor ($1..) dan
bernama; grup yang tidak ikut cocok ditampilkan -. Pattern yang diawali "-"
perlu didahului "--". Worker dibatasi 256 MB old-generation heap dan dihentikan saat timeout.
Exit: 0 sukses, 1 timeout/batas memori atau (--test) tidak cocok, 2 pattern/penggunaan salah.
Contoh: dt regex '(\\d+)-(?<b>\\w+)' "12-ab 7-z"   |   echo 'a.b' | dt regex -r '$&!' '\\.'`);
}
const quote = (value) => (value === null ? '-' : JSON.stringify(value));
function formatMatch(m, n) {
    const lines = [`#${n} index=${m.index} ${JSON.stringify(m.match)}`];
    m.groups.forEach((g, i) => lines.push(`  $${i + 1} ${quote(g)}`));
    for (const [name, g] of Object.entries(m.named))
        lines.push(`  <${name}> ${quote(g)}`);
    return lines.join('\n');
}
async function run(args) {
    const { values, positionals } = parseToolArgs(args, {
        flags: { type: 'string', short: 'f' },
        replace: { type: 'string', short: 'r' },
        first: { type: 'boolean' },
        test: { type: 'boolean' },
        timeout: { type: 'string' },
        json: { type: 'boolean' },
        file: { type: 'string' },
        help: { type: 'boolean', short: 'h' },
    });
    if (values.help) {
        help();
        return;
    }
    const [pattern, ...rest] = positionals;
    if (pattern === undefined)
        throw new UsageError('Pattern wajib diisi.');
    if (values.test && (values.json || values.replace !== undefined)) {
        throw new UsageError('--test tidak bisa digabung dengan --json atau --replace.');
    }
    const timeout = integer(values.timeout, 2000, 1, 60000, 'timeout');
    const flags = values.flags ?? '';
    let normalizedFlags;
    try {
        checkFlags(flags);
        normalizedFlags = new RegExp(pattern, flags).flags;
    }
    catch (error) {
        throw new UsageError(`Regex tidak valid: ${error.message}`, { cause: error });
    }
    const text = (await readInput(rest, values.file, 'Teks:')).toString('utf8');
    const { matches, replaced } = await matchInWorker({ pattern, flags, text, first: values.first ?? false, replace: values.replace }, timeout);
    if (values.test) {
        if (!matches.length)
            process.exitCode = 1;
        return;
    }
    if (values.json) {
        printJson({ pattern, flags: normalizedFlags, matches, ...(replaced === undefined ? {} : { replaced }) });
        return;
    }
    if (matches.length)
        console.log(matches.map((m, i) => formatMatch(m, i + 1)).join('\n'));
    else
        console.error('Tidak ada kecocokan.');
    if (replaced !== undefined)
        console.log(`Hasil ganti:\n${replaced}`);
}
export const regexTool = {
    name: 'regex',
    aliases: ['re'],
    description: 'Uji regex JavaScript: daftar kecocokan, grup, dan penggantian',
    category: 'Data',
    run,
};
//# sourceMappingURL=index.js.map