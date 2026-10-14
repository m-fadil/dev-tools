import { UsageError } from '../../errors.js';
import { parseToolArgs, printJson } from '../../io.js';
import type { Tool } from '../../tool.js';
import { parseRange, satisfies } from './range.js';
import { bump, checkPreid, compare, format, parseVersion, RELEASES, tryParse, type Release } from './version.js';

function help(): void {
    console.log(`dt semver <perintah> [opsi] <argumen...>   (alias: sv)

Operasi Semantic Versioning 2.0.0 (SemVer ketat: tanpa nol di depan pada angka).
Satu awalan "v" diterima dan dibuang (v1.2.3 = 1.2.3).
  parse <v>               Tampilkan major, minor, patch, prerelease, build
  valid <v>               Cetak valid/invalid; exit 0 jika valid, 1 jika tidak
  compare <a> <b>         Cetak -1, 0, atau 1 (metadata build diabaikan)
  sort <v...>             Urutkan naik
      --reverse           Urutkan turun
  bump <rilis> <v>        Naikkan versi; rilis: ${RELEASES.join(' | ')}
      --preid <id>        Tag prerelease untuk pre* (mis. beta -> 1.0.1-beta.0)
  satisfies <v> <range>   Cetak true/false; exit 0 jika cocok, 1 jika tidak
      --json              Keluaran JSON (satu dokumen) untuk semua perintah
  -h, --help              Tampilkan bantuan

Range (gaya npm): >= <= > < =, ^1.2.3, ~1.2.3, 1.2.x, 1.*, 1.2.3 - 2.3.4,
spasi = AND, || = OR. Versi prerelease hanya cocok dengan himpunan pembanding
yang memuat prerelease pada [major,minor,patch] yang sama.
Bump mengikuti npm: pre* pada versi non-prerelease menaikkan patch dulu;
prerelease menaikkan identifier numerik terakhir atau menambah .0. Metadata build dibuang.
Exit: 0 sukses, 1 valid/satisfies bernilai salah atau bump melebihi bilangan bulat aman;
2 penggunaan salah (termasuk versi tidak valid).
Contoh: dt semver bump minor 1.2.3   |   dt semver satisfies 1.4.0 "^1.2 || >=3"`);
}

function exact(positionals: string[], count: number, usage: string): void {
    if (positionals.length !== count) throw new UsageError(`Penggunaan: dt semver ${usage}`);
}

async function run(args: string[]): Promise<void> {
    const { values, positionals } = parseToolArgs(args, {
        reverse: { type: 'boolean' },
        preid: { type: 'string' },
        json: { type: 'boolean' },
        help: { type: 'boolean', short: 'h' },
    });
    if (values.help) {
        help();
        return;
    }
    const [command, ...rest] = positionals;
    if (!command) throw new UsageError('Perintah wajib diisi: parse, valid, compare, sort, bump, satisfies.');
    const json = values.json ?? false;
    if (values.reverse && command !== 'sort') throw new UsageError('--reverse hanya untuk perintah sort.');
    if (values.preid !== undefined && command !== 'bump') throw new UsageError('--preid hanya untuk perintah bump.');

    switch (command) {
        case 'parse': {
            exact(rest, 1, 'parse <versi>');
            const v = parseVersion(rest[0]!);
            // Numeric identifiers become numbers when they are exactly representable.
            const ident = (id: string) => (/^\d+$/.test(id) && Number.isSafeInteger(Number(id)) ? Number(id) : id);
            if (json) {
                printJson({
                    input: rest[0],
                    version: format(v),
                    major: v.major,
                    minor: v.minor,
                    patch: v.patch,
                    prerelease: v.prerelease.map(ident),
                    build: v.build,
                });
            } else {
                console.log(`major:      ${v.major}`);
                console.log(`minor:      ${v.minor}`);
                console.log(`patch:      ${v.patch}`);
                console.log(`prerelease: ${v.prerelease.join('.') || '-'}`);
                console.log(`build:      ${v.build.join('.') || '-'}`);
            }
            return;
        }
        case 'valid': {
            exact(rest, 1, 'valid <versi>');
            const v = tryParse(rest[0]!);
            const ok = typeof v !== 'string';
            if (json) {
                printJson({ input: rest[0], valid: ok, version: ok ? format(v) : null, error: ok ? null : v });
            } else {
                console.log(ok ? 'valid' : 'invalid');
            }
            if (!ok) process.exitCode = 1;
            return;
        }
        case 'compare': {
            exact(rest, 2, 'compare <a> <b>');
            const result = compare(parseVersion(rest[0]!), parseVersion(rest[1]!));
            if (json) printJson({ a: rest[0], b: rest[1], result });
            else console.log(result);
            return;
        }
        case 'sort': {
            if (!rest.length) throw new UsageError('Penggunaan: dt semver sort [--reverse] <versi...>');
            const reverse = values.reverse ?? false;
            const parsed = rest.map(parseVersion);
            // Array.prototype.sort is stable, so versions that differ only in build keep input order.
            parsed.sort((a, b) => (reverse ? compare(b, a) : compare(a, b)));
            const versions = parsed.map((v) => format(v));
            if (json) printJson({ reverse, versions });
            else console.log(versions.join('\n'));
            return;
        }
        case 'bump': {
            exact(rest, 2, 'bump <rilis> <versi>');
            const release = rest[0]!;
            if (!(RELEASES as readonly string[]).includes(release)) {
                throw new UsageError(`Jenis rilis tidak valid: ${release}. Pilihan: ${RELEASES.join(', ')}`);
            }
            const preid = values.preid;
            if (preid !== undefined) {
                checkPreid(preid);
                if (!release.startsWith('pre')) throw new UsageError('--preid hanya berlaku untuk rilis pre*.');
            }
            const from = parseVersion(rest[1]!);
            const result = format(bump(from, release as Release, preid));
            if (json) printJson({ release, preid: preid ?? null, from: format(from), version: result });
            else console.log(result);
            return;
        }
        case 'satisfies': {
            exact(rest, 2, 'satisfies <versi> <range>');
            const v = parseVersion(rest[0]!);
            const ok = satisfies(v, parseRange(rest[1]!));
            if (json) printJson({ version: format(v), range: rest[1], satisfies: ok });
            else console.log(ok);
            if (!ok) process.exitCode = 1;
            return;
        }
        default:
            throw new UsageError(
                `Perintah tidak dikenal: ${command}. Pilihan: parse, valid, compare, sort, bump, satisfies.`,
            );
    }
}

export const semverTool: Tool = {
    name: 'semver',
    aliases: ['sv'],
    description: 'Parse, bandingkan, urutkan, naikkan, dan cocokkan versi SemVer dengan range',
    category: 'Data',
    run,
};
