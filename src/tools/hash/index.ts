import type { Tool } from '../../tool.js';
import { UsageError } from '../../errors.js';
import { choice, parseToolArgs, printJson, readInput } from '../../io.js';
import { crc32Digest } from './crc32.js';
import { HASH_ALGORITHMS, HASH_ENCODINGS, hash } from './hash.js';

const ALGORITHMS = [...HASH_ALGORITHMS, 'crc32'] as const;

function help(): void {
    console.log(`dt hash [options] [teks]

Hash teks, file, atau stdin. Tanpa teks/--file/stdin: prompt (terminal).
  -a, --algorithm <nama>  ${ALGORITHMS.join(', ')} (default: sha256)
  -e, --encoding <jenis>  ${HASH_ENCODINGS.join(', ')} (default: hex)
  -k, --key <kunci>       Hitung HMAC dengan kunci ini, bukan hash biasa (tidak untuk crc32)
  -f, --file <path>       Hash isi file
      --json              Keluaran JSON: {algorithm, encoding, hmac, digest}
  -h, --help              Tampilkan bantuan

crc32 adalah checksum 32-bit (hex 8 digit), bukan hash kriptografis.
Input stdin dipakai apa adanya, termasuk newline akhir (seperti sha256sum).
Contoh: dt hash -a sha512 "halo"   |   dt hash -f dist/src/cli.js`);
}

async function run(args: string[]): Promise<void> {
    const { values, positionals } = parseToolArgs(args, {
        algorithm: { type: 'string', short: 'a' },
        encoding: { type: 'string', short: 'e' },
        key: { type: 'string', short: 'k' },
        file: { type: 'string', short: 'f' },
        json: { type: 'boolean' },
        help: { type: 'boolean', short: 'h' },
    });
    if (values.help) {
        help();
        return;
    }
    const algorithm = choice(values.algorithm, ALGORITHMS, 'sha256', 'algorithm');
    const encoding = choice(values.encoding, HASH_ENCODINGS, 'hex', 'encoding');
    if (algorithm === 'crc32' && values.key !== undefined) {
        throw new UsageError('crc32 bukan hash berkunci; --key tidak bisa dipakai dengan crc32.');
    }
    const data = await readInput(positionals, values.file, 'Teks yang akan di-hash');
    const digest = algorithm === 'crc32' ? crc32Digest(data, encoding) : hash(data, algorithm, encoding, values.key);
    if (values.json) {
        printJson({ algorithm, encoding, hmac: values.key !== undefined, digest });
        return;
    }
    console.log(digest);
}

export const hashTool: Tool = {
    name: 'hash',
    description: 'Hash atau HMAC teks/file (md5, sha1, sha2, sha3, blake2b, crc32)',
    category: 'Keamanan & Kripto',
    run,
};
