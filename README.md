# dev-tools

Kumpulan alat praktis untuk pekerjaan sehari-hari, cukup dengan perintah `dt`.

## Instalasi

Pilih salah satu: skrip pemasang, kompilasi dari sumber dengan Cargo, unduh
binary siap pakai, atau gunakan Docker. Untuk `syncgit` diperlukan [Git](https://git-scm.com/).

### Skrip pemasang (Linux dan macOS)

Mengunduh binary sesuai OS dan arsitektur, memverifikasi checksum, lalu
memasangnya ke `~/.local/bin`:

```bash
curl -fsSL https://github.com/m-fadil/dev-tools/releases/latest/download/install.sh | sh
```

`releases/latest` selalu mengarah ke rilis stabil terbaru. Untuk versi tertentu,
pakai skrip dari rilis itu; versi tersebut menjadi default-nya:

```bash
curl -fsSL https://github.com/m-fadil/dev-tools/releases/download/vX.Y.Z/install.sh | sh
```

Pengaturan lewat variabel lingkungan:

| Variabel | Fungsi |
| --- | --- |
| `DT_VERSION` | Versi yang dipasang, mis. `vX.Y.Z` atau `X.Y.Z-rc.1` (juga untuk prarilis). |
| `DT_INSTALL_DIR` | Folder tujuan (default `~/.local/bin`). |

Versi juga bisa diberikan sebagai argumen: `... | sh -s -- vX.Y.Z`. Jalankan
ulang untuk memperbarui. Di Linux yang dipasang adalah build musl (statis).
Pastikan `~/.local/bin` ada di `PATH`. Di Windows, unduh arsip `.zip` dari
halaman rilis.

### Dengan Cargo

Perlu [Rust](https://rustup.rs/) 1.99 atau lebih baru:

```bash
cargo install --git https://github.com/m-fadil/dev-tools --locked
dt --help
```

Untuk memperbarui, jalankan lagi perintah yang sama. Dari clone lokal:
`cargo install --path . --locked`.

### Binary siap pakai

Setiap rilis di [GitHub Releases](https://github.com/m-fadil/dev-tools/releases)
menyertakan binary `dt` tanpa dependensi tambahan:

| Sistem | Arsitektur | Berkas |
| --- | --- | --- |
| Linux (glibc) | x64 | `dt-v<versi>-x86_64-unknown-linux-gnu.tar.gz` |
| Linux (glibc) | arm64 | `dt-v<versi>-aarch64-unknown-linux-gnu.tar.gz` |
| Linux (musl, statis) | x64 | `dt-v<versi>-x86_64-unknown-linux-musl.tar.gz` |
| Linux (musl, statis) | arm64 | `dt-v<versi>-aarch64-unknown-linux-musl.tar.gz` |
| macOS | x64 | `dt-v<versi>-x86_64-apple-darwin.tar.gz` |
| macOS | arm64 (Apple Silicon) | `dt-v<versi>-aarch64-apple-darwin.tar.gz` |
| Windows | x64 | `dt-v<versi>-x86_64-pc-windows-msvc.zip` |
| Windows | arm64 | `dt-v<versi>-aarch64-pc-windows-msvc.zip` |

Ekstrak arsip, lalu letakkan `dt` (atau `dt.exe`) di folder yang ada di `PATH`.
Berkas `SHA256SUMS` pada rilis berisi checksum semua arsip:

```bash
sha256sum --check --ignore-missing SHA256SUMS
```

Dengan [cargo-binstall](https://github.com/cargo-bins/cargo-binstall) (mengunduh
binary rilis, tanpa kompilasi):

```bash
cargo binstall --git https://github.com/m-fadil/dev-tools dev-tools
```

### Docker

Unduh image:

```bash
docker pull ghcr.io/m-fadil/dev-tools:latest
```

Buka menu atau jalankan alat langsung:

```bash
docker run --rm -it ghcr.io/m-fadil/dev-tools:latest
docker run --rm ghcr.io/m-fadil/dev-tools:latest --help
docker run --rm ghcr.io/m-fadil/dev-tools:latest hash "halo"
docker run --rm ghcr.io/m-fadil/dev-tools:latest random uuid
```

Untuk membaca file dari folder saat ini:

```bash
docker run --rm -v "$PWD:/workspace:ro" \
  ghcr.io/m-fadil/dev-tools:latest json --file /workspace/data.json
```

Untuk mengolah teks melalui pipe:

```bash
printf 'halo' | docker run --rm -i ghcr.io/m-fadil/dev-tools:latest encode
```

Di Docker, tulis nama alat langsung setelah nama image—tidak perlu menambahkan
`dt`. Gunakan `-it` untuk menu, atau `-i` untuk input dari pipe.

## Cara pakai

```bash
dt                         # buka menu dan cari alat
dt --help                  # lihat semua alat
dt --version               # versi terpasang (juga -v)
dt update                  # pasang rilis terbaru
dt hash "halo"             # buat hash
dt random uuid             # buat ID acak
dt random token --bits 256  # buat token acak
dt password                # buat password
dt json --file data.json   # rapikan JSON
dt time                    # lihat waktu sekarang
```

Untuk panduan setiap alat, jalankan `dt <nama> --help`, misalnya `dt encrypt --help`.

## Fitur

- Hash, password, ID acak (UUID, ULID, NanoID), dan enkripsi.
- Encode/decode, escape, JSON, YAML, TOML, CSV, dan pengolahan teks.
- Waktu, durasi, jadwal, warna, ukuran data, QR code, dan perhitungan.
- Pemeriksaan jaringan, gambar, file, dan sinkronisasi Git.

Contoh lengkap tersedia di [dokumentasi](docs/README.md).

Untuk menghapus instalasi: `rm ~/.local/bin/dt` (skrip pemasang) atau
`cargo uninstall dev-tools` (Cargo).
