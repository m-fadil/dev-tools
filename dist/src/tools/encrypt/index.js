import { password as passwordPrompt } from '@inquirer/prompts';
import { UsageError } from '../../errors.js';
import { parseToolArgs, readInput, writeOutput } from '../../io.js';
import { decrypt, encrypt } from './cipher.js';
function help() {
    console.log(`dt encrypt [options] [teks]

Enkripsi AES-256-GCM (kunci dari password via scrypt); hasil base64url.
Tanpa teks/--file/stdin: prompt (terminal).
  -d, --decrypt           Dekripsi data hasil encrypt, bukan mengenkripsi
  -p, --password <teks>   Password (juga: env DEV_TOOL_PASSWORD; default: prompt)
  -f, --file <path>       Baca input dari file
  -h, --help              Tampilkan bantuan

--password terlihat di daftar proses dan history shell; di skrip, pakai
DEV_TOOL_PASSWORD. Password salah atau data diubah: exit 1.
Contoh: dt encrypt -p rahasia "halo" | dt encrypt -d -p rahasia`);
}
async function resolvePassword(option) {
    if (option === '')
        throw new UsageError('Password tidak boleh kosong.');
    const value = option ?? process.env.DEV_TOOL_PASSWORD;
    if (value)
        return value;
    if (!process.stdin.isTTY) {
        throw new UsageError('Password diperlukan: gunakan --password atau env DEV_TOOL_PASSWORD.');
    }
    const prompted = await passwordPrompt({ message: 'Password', mask: '*' }, { output: process.stderr });
    if (!prompted)
        throw new UsageError('Password tidak boleh kosong.');
    return prompted;
}
async function run(args) {
    const { values, positionals } = parseToolArgs(args, {
        decrypt: { type: 'boolean', short: 'd' },
        password: { type: 'string', short: 'p' },
        file: { type: 'string', short: 'f' },
        help: { type: 'boolean', short: 'h' },
    });
    if (values.help) {
        help();
        return;
    }
    // Resolve the password first so a stdin-backed prompt is not starved by piped input.
    const password = await resolvePassword(values.password);
    const data = await readInput(positionals, values.file, values.decrypt ? 'Data terenkripsi' : 'Teks yang akan dienkripsi');
    if (values.decrypt)
        writeOutput(decrypt(data.toString('utf8'), password));
    else
        writeOutput(`${encrypt(data, password)}\n`);
}
export const encryptTool = {
    name: 'encrypt',
    description: 'Enkripsi/dekripsi teks atau file dengan password (AES-256-GCM)',
    category: 'Keamanan & Kripto',
    run,
};
//# sourceMappingURL=index.js.map