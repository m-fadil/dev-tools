# Catatan rilis

## v1.0.0

`dt` ditulis ulang dalam Rust. Instalasi dari sumber memakai
`cargo install --git https://github.com/m-fadil/dev-tools --locked`;
crate tidak diterbitkan ke crates.io. Distribusi npm dan branch distribusi
`release` dihentikan. Rilis menyediakan binary untuk delapan target Linux,
macOS, dan Windows, serta skrip pemasang `install.sh`; image Docker tetap tersedia dan membungkus binary musl.

### Perubahan yang tidak kompatibel

- `dt regex` memakai sintaks crate Rust `regex`, bukan regex JavaScript.
  Lookaround dan backreference ditolak dengan `Regex tidak valid: <galat regex>`
  (exit 2). Flag yang diterima hanya `i m s x`; flag lain menghasilkan
  `Flag tidak dikenal: <c>. Pilihan: imsx` (exit 2). Pencarian global selalu aktif,
  flag JSON menjadi huruf unik terurut, dan indeks kecocokan adalah offset byte
  UTF-8 (sebelumnya indeks UTF-16). Template penggantian memakai `$1`, `${nama}`,
  `$0`, dan `$$`, bukan `$&` atau `$<nama>`. `--timeout`, worker, dan batas heap
  worker dihapus; error timeout/batas memori worker tidak lagi berlaku.
  Lihat [panduan regex](regex.md).
- Pesan kesalahan parsing opsi berasal dari clap, bukan parser versi sebelumnya.
  Kode keluar kesalahan penggunaan tetap 2.
- `dt json` memakai `serde_json` dengan urutan key dan presisi arbitrer.
  Bilangan bulat di atas 2^53 dipertahankan persis, bukan dibulatkan.
  Validator sintaks, pesan baris/kolom, dan batas kedalaman 2000 level tetap berlaku.
  Lihat [panduan JSON](json.md).

### Toolchain dan dependensi

Build dikunci ke Rust 1.99.0 (`rust-version = "1.99"`). Rust 1.99.1 yang disebut
pada rencana awal belum tersedia dari server rustup saat implementasi (HTTP 404);
karena itu versi stabil 1.99.0 dipakai untuk build lokal dan CI.
Binary siap pakai tidak memerlukan instalasi Rust.

Crate pure-Rust `url` dan `idna` digunakan untuk mempertahankan normalisasi URL
WHATWG dan konversi domain Unicode/UTS46 dari implementasi sebelumnya.
Crate pure-Rust `jiff` menyediakan aturan zona waktu berulang untuk tanggal
di luar cakupan tabel transisi `chrono-tz`. `jiff-tzdb` dikunci ke 0.1.8
(IANA 2026c), mengikuti data zona waktu implementasi sebelumnya.
Fitur `unbounded_depth` pada `serde_json` memungkinkan parser menerima dokumen
yang telah lolos validator kedalaman 2000 level; batas pengguna tetap diberlakukan
oleh validator.

[Kembali ke dokumentasi](README.md).
