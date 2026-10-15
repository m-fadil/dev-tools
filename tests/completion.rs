mod common;
use common::dt;

// Fixed outputs from the legacy registry, including every name, alias and description.
const BASH: &str = r###"_dt() {
    local cur="${COMP_WORDS[COMP_CWORD]}"
    if [ "$COMP_CWORD" -eq 1 ]; then
        COMPREPLY=( $(compgen -W 'syncgit hash checksum sum encrypt encode decode random password pw jwt json time ts cron url diff env port ascii bytes case cidr color completion csv fake image lorem mac math number num regex re semver sv text --help' -- "$cur") )
    fi
}
complete -F _dt dt
"###;
const ZSH: &str = r###"#compdef dt
_dt() {
    local -a tools
    tools=(
        'syncgit:Sinkron commit dan tag antar remote Git dengan pemetaan mailmap'
        'hash:Hash atau HMAC teks/file (md5, sha1, sha2, sha3, blake2b, crc32)'
        'checksum:Hitung dan verifikasi checksum file (format sha256sum)'
        'sum:Hitung dan verifikasi checksum file (format sha256sum)'
        'encrypt:Enkripsi/dekripsi teks atau file dengan password (AES-256-GCM)'
        'encode:Encode teks/file ke base64, base64url, hex, atau URL'
        'decode:Decode base64, base64url, hex, atau URL ke teks/biner'
        'random:Buat UUID atau token acak melalui subcommand uuid/token'
        'password:Buat password acak yang aman secara kriptografis'
        'pw:Buat password acak yang aman secara kriptografis'
        'jwt:Decode, verifikasi, dan buat JWT HMAC (HS256/384/512)'
        'json:Format, minify, urutkan key, validasi, atau query JSON'
        'time:Konversi epoch/ISO-8601 ke UTC, zona waktu, dan waktu relatif'
        'ts:Konversi epoch/ISO-8601 ke UTC, zona waktu, dan waktu relatif'
        'cron:Jelaskan ekspresi cron dan tampilkan waktu eksekusi berikutnya'
        'url:Uraikan URL menjadi komponen dan parameter query'
        'diff:Bandingkan dua file (sha256, diff baris, atau offset byte beda)'
        'env:Bandingkan dua file .env (kunci dan nilai, nilai disamarkan)'
        'port:Periksa port TCP terbuka atau cari port kosong'
        'ascii:Tabel ASCII dan info karakter (desimal, hex, oktal, biner, Unicode)'
        'bytes:Konversi ukuran data (KB/MB/GB dan KiB/MiB/GiB) secara eksak'
        'case:Ubah gaya penulisan teks (camel, snake, kebab, title, dst.)'
        'cidr:Hitung subnet IPv4/IPv6\: netmask, broadcast, rentang host, contains, split'
        'color:Konversi warna (hex, rgb, hsl, hwb, nama CSS) dan kontras WCAG'
        'completion:Cetak skrip auto-complete shell (bash, zsh, fish)'
        'csv:Baca CSV\: tabel, JSON, TSV, pilih kolom, statistik'
        'fake:Buat data palsu (nama, email, alamat, IP, tanggal, dll.) untuk pengujian'
        'image:Metadata gambar (PNG, JPEG, GIF, WebP)\: ukuran, rasio, EXIF, frame'
        'lorem:Buat teks lorem ipsum (kata, kalimat, atau paragraf)'
        'mac:Parse dan konversi alamat MAC (colon, hyphen, dot, plain), flag, dan alamat acak'
        'math:Hitung ekspresi matematika (operator, fungsi, konstanta)'
        'number:Ubah bilangan antar basis 2/8/10/16 dengan ukuran bebas'
        'num:Ubah bilangan antar basis 2/8/10/16 dengan ukuran bebas'
        'regex:Uji regex JavaScript\: daftar kecocokan, grup, dan penggantian'
        're:Uji regex JavaScript\: daftar kecocokan, grup, dan penggantian'
        'semver:Parse, bandingkan, urutkan, naikkan, dan cocokkan versi SemVer dengan range'
        'sv:Parse, bandingkan, urutkan, naikkan, dan cocokkan versi SemVer dengan range'
        'text:Alat teks\: statistik, balik, slug, frekuensi kata, olah baris'
    )
    if (( CURRENT == 2 )); then
        _describe 'tool' tools
    fi
}
# Autoloaded from $fpath as _dt, zsh runs this file on the first Tab: call the completer then.
# When sourced instead, only register it.
if [[ $funcstack[1] == _dt ]]; then
    _dt "$@"
else
    compdef _dt dt
fi
"###;
const FISH: &str = r###"complete -c dt -f -n '__fish_use_subcommand' -a 'syncgit' -d 'Sinkron commit dan tag antar remote Git dengan pemetaan mailmap'
complete -c dt -f -n '__fish_use_subcommand' -a 'hash' -d 'Hash atau HMAC teks/file (md5, sha1, sha2, sha3, blake2b, crc32)'
complete -c dt -f -n '__fish_use_subcommand' -a 'checksum' -d 'Hitung dan verifikasi checksum file (format sha256sum)'
complete -c dt -f -n '__fish_use_subcommand' -a 'sum' -d 'Hitung dan verifikasi checksum file (format sha256sum)'
complete -c dt -f -n '__fish_use_subcommand' -a 'encrypt' -d 'Enkripsi/dekripsi teks atau file dengan password (AES-256-GCM)'
complete -c dt -f -n '__fish_use_subcommand' -a 'encode' -d 'Encode teks/file ke base64, base64url, hex, atau URL'
complete -c dt -f -n '__fish_use_subcommand' -a 'decode' -d 'Decode base64, base64url, hex, atau URL ke teks/biner'
complete -c dt -f -n '__fish_use_subcommand' -a 'random' -d 'Buat UUID atau token acak melalui subcommand uuid/token'
complete -c dt -f -n '__fish_use_subcommand' -a 'password' -d 'Buat password acak yang aman secara kriptografis'
complete -c dt -f -n '__fish_use_subcommand' -a 'pw' -d 'Buat password acak yang aman secara kriptografis'
complete -c dt -f -n '__fish_use_subcommand' -a 'jwt' -d 'Decode, verifikasi, dan buat JWT HMAC (HS256/384/512)'
complete -c dt -f -n '__fish_use_subcommand' -a 'json' -d 'Format, minify, urutkan key, validasi, atau query JSON'
complete -c dt -f -n '__fish_use_subcommand' -a 'time' -d 'Konversi epoch/ISO-8601 ke UTC, zona waktu, dan waktu relatif'
complete -c dt -f -n '__fish_use_subcommand' -a 'ts' -d 'Konversi epoch/ISO-8601 ke UTC, zona waktu, dan waktu relatif'
complete -c dt -f -n '__fish_use_subcommand' -a 'cron' -d 'Jelaskan ekspresi cron dan tampilkan waktu eksekusi berikutnya'
complete -c dt -f -n '__fish_use_subcommand' -a 'url' -d 'Uraikan URL menjadi komponen dan parameter query'
complete -c dt -f -n '__fish_use_subcommand' -a 'diff' -d 'Bandingkan dua file (sha256, diff baris, atau offset byte beda)'
complete -c dt -f -n '__fish_use_subcommand' -a 'env' -d 'Bandingkan dua file .env (kunci dan nilai, nilai disamarkan)'
complete -c dt -f -n '__fish_use_subcommand' -a 'port' -d 'Periksa port TCP terbuka atau cari port kosong'
complete -c dt -f -n '__fish_use_subcommand' -a 'ascii' -d 'Tabel ASCII dan info karakter (desimal, hex, oktal, biner, Unicode)'
complete -c dt -f -n '__fish_use_subcommand' -a 'bytes' -d 'Konversi ukuran data (KB/MB/GB dan KiB/MiB/GiB) secara eksak'
complete -c dt -f -n '__fish_use_subcommand' -a 'case' -d 'Ubah gaya penulisan teks (camel, snake, kebab, title, dst.)'
complete -c dt -f -n '__fish_use_subcommand' -a 'cidr' -d 'Hitung subnet IPv4/IPv6: netmask, broadcast, rentang host, contains, split'
complete -c dt -f -n '__fish_use_subcommand' -a 'color' -d 'Konversi warna (hex, rgb, hsl, hwb, nama CSS) dan kontras WCAG'
complete -c dt -f -n '__fish_use_subcommand' -a 'completion' -d 'Cetak skrip auto-complete shell (bash, zsh, fish)'
complete -c dt -f -n '__fish_use_subcommand' -a 'csv' -d 'Baca CSV: tabel, JSON, TSV, pilih kolom, statistik'
complete -c dt -f -n '__fish_use_subcommand' -a 'fake' -d 'Buat data palsu (nama, email, alamat, IP, tanggal, dll.) untuk pengujian'
complete -c dt -f -n '__fish_use_subcommand' -a 'image' -d 'Metadata gambar (PNG, JPEG, GIF, WebP): ukuran, rasio, EXIF, frame'
complete -c dt -f -n '__fish_use_subcommand' -a 'lorem' -d 'Buat teks lorem ipsum (kata, kalimat, atau paragraf)'
complete -c dt -f -n '__fish_use_subcommand' -a 'mac' -d 'Parse dan konversi alamat MAC (colon, hyphen, dot, plain), flag, dan alamat acak'
complete -c dt -f -n '__fish_use_subcommand' -a 'math' -d 'Hitung ekspresi matematika (operator, fungsi, konstanta)'
complete -c dt -f -n '__fish_use_subcommand' -a 'number' -d 'Ubah bilangan antar basis 2/8/10/16 dengan ukuran bebas'
complete -c dt -f -n '__fish_use_subcommand' -a 'num' -d 'Ubah bilangan antar basis 2/8/10/16 dengan ukuran bebas'
complete -c dt -f -n '__fish_use_subcommand' -a 'regex' -d 'Uji regex JavaScript: daftar kecocokan, grup, dan penggantian'
complete -c dt -f -n '__fish_use_subcommand' -a 're' -d 'Uji regex JavaScript: daftar kecocokan, grup, dan penggantian'
complete -c dt -f -n '__fish_use_subcommand' -a 'semver' -d 'Parse, bandingkan, urutkan, naikkan, dan cocokkan versi SemVer dengan range'
complete -c dt -f -n '__fish_use_subcommand' -a 'sv' -d 'Parse, bandingkan, urutkan, naikkan, dan cocokkan versi SemVer dengan range'
complete -c dt -f -n '__fish_use_subcommand' -a 'text' -d 'Alat teks: statistik, balik, slug, frekuensi kata, olah baris'
"###;

#[test]
fn scripts_are_byte_identical_to_legacy_registry() {
    for (shell, expected) in [("bash", BASH), ("zsh", ZSH), ("fish", FISH)] {
        let r = dt(&["completion", shell]);
        assert_eq!(r.code, 0, "{}", r.err);
        assert_eq!(r.out, expected, "{shell}");
        for name in ["syncgit", "semver", "sv", "ts"] {
            assert!(r.out.contains(name));
        }
    }
    assert_eq!(dt(&["completion", "powershell"]).code, 2);
    assert_eq!(dt(&["completion"]).code, 2);
}
#[test]
fn registers_only_dt_and_supports_zsh_autoload_and_source() {
    let bash = dt(&["completion", "bash"]).out;
    assert!(bash.starts_with("_dt() {\n"));
    assert!(bash.lines().any(|l| l == "complete -F _dt dt"));
    let zsh = dt(&["completion", "zsh"]).out;
    assert!(zsh.starts_with("#compdef dt\n_dt() {\n"));
    assert!(zsh.contains("if [[ $funcstack[1] == _dt ]]; then\n    _dt \"$@\""));
    assert!(zsh.lines().any(|l| l == "    compdef _dt dt"));
    assert!(
        dt(&["completion", "fish"])
            .out
            .lines()
            .all(|l| l.starts_with("complete -c dt "))
    );
}
