# Pengembangan dan maintenance

## Struktur kode

```text
src/
  main.rs                Entry point, menu/help, dispatch, kode keluar
  errors.rs              CliError (Usage → 2, Failed → 1, Cancelled → 130)
  tool.rs                Struct Tool { name, aliases, description, category, run }
  io.rs                  Parsing argumen (clap), input/output, --json
  codec.rs               base64, base64url, hex, URL
  numfmt.rs              Format angka seperti JavaScript (js_number)
  random.rs              Byte acak dari OS
  tools/
    mod.rs               Registry command (TOOLS), find_tool, tools_by_category
    <nama>/
      mod.rs             Opsi (clap), prompt, run
      <logika>.rs        Logika operasi (mis. syncgit/sync.rs)
tests/
  common/mod.rs          Helper dt(), dt_in(), dt_env() — menjalankan binary tanpa terminal
  <nama>.rs              Integration test per command
```

Setiap command mengelola opsi, prompt, dan implementasinya sendiri. Registry
menjadi sumber yang sama untuk menu, bantuan, auto-complete, dan dispatch.

## Menambahkan command

1. Buat `src/tools/<nama>/mod.rs` yang mengekspor `pub static TOOL: Tool`.
2. Isi `name`, `description`, `category` (kelompok di bantuan dan menu), opsional
   `aliases`, dan `run` sesuai `src/tool.rs`. Nama dan alias harus unik (dijaga
   `tests/registry.rs`). Deklarasikan opsi dengan `clap::Parser` dan teks bantuan
   sendiri (`override_help`), lalu parse lewat `io::parse`. Pisahkan logika operasi
   dari parsing opsi dan prompt. Tool yang menghasilkan data terstruktur sebaiknya
   menyediakan `--json`: satu dokumen JSON dari `io::print_json` di stdout,
   diagnostik tetap di stderr.
3. Tambahkan `pub mod <nama>;` dan `&<nama>::TOOL` ke `src/tools/mod.rs`.
4. Tambahkan panduan penggunaan di `docs/` dan tautan di `docs/README.md`.
5. Tambahkan integration test `tests/<nama>.rs` untuk perilaku yang terlihat pengguna.

## Build dan verifikasi

```bash
cargo build
cargo fmt --check        # atau cargo fmt untuk merapikan
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
```

Versi Rust dikunci di `rust-toolchain.toml`; `rustup` memasangnya otomatis.
Gunakan lingkungan sementara (`tempfile`) dan data contoh agar test terisolasi dari
repo, identitas, dan kredensial pengguna. Test integration menjalankan binary
dengan environment kosong dan stdin berupa pipe, seperti cron/CI.

CI (`.github/workflows/test.yml`) menjalankan `cargo fmt --check`, `cargo clippy`,
dan `cargo test` di Linux, macOS, dan Windows untuk setiap push ke `main` dan pull
request. Job container membangun binary musl untuk AMD64 dan ARM64 (runner native),
membungkusnya dengan `Dockerfile`, lalu menjalankan bantuan CLI, bantuan syncgit,
serta pemeriksaan Git, SSH, dan user non-root. CI ini tidak memublikasikan image.

Prompt interaktif perlu diverifikasi di terminal nyata. Pengujian noninteraktif
harus memakai opsi eksplisit tanpa memerlukan input terminal.

## Rilis

Rilis dipicu oleh push tag `vX.Y.Z` pada commit sumber. Dari working tree bersih
setelah perubahan siap dirilis:

```bash
# ubah version di Cargo.toml, lalu perbarui Cargo.lock
cargo check
git commit -am "chore: release vX.Y.Z"
git tag vX.Y.Z
git push origin main         # setelah mendapat persetujuan untuk push
git push origin vX.Y.Z
```

Gunakan `X.Y.Z-rc.1` (di `Cargo.toml` dan tag) untuk uji coba: tag dengan `-`
ditandai prerelease dan tidak memperbarui tag image `latest`.

Workflow `release` (`.github/workflows/release.yml`) checkout `${{ github.sha }}`
pada event tag dan menjalankan job berikut:

1. `verify`: memastikan nama tag sama dengan `v<version di Cargo.toml>`, lalu
   `cargo fmt --check`, `cargo clippy`, dan `cargo test`. Jika gagal, tidak ada
   yang dipublikasikan.
2. `build`: kompilasi `--release` untuk 8 target (Linux gnu dan musl x64/arm64,
   macOS x64/arm64, Windows x64/arm64) pada runner native, uji cepat binary
   (`dt --help`, `dt hash abc`), lalu kemas menjadi
   `dt-v<versi>-<target>.tar.gz` (Windows: `.zip`) dengan folder
   `dt-v<versi>-<target>/` berisi `dt` dan `README.md`, plus berkas `.sha256`.
   Penamaan arsip ini dipakai oleh `[package.metadata.binstall]` di `Cargo.toml`;
   ubah keduanya bersamaan.
3. `publish`: membuat GitHub release (prerelease bila tag memuat `-`), melampirkan
   semua arsip, `install.sh` (salinan `install.sh` repo dengan `DEFAULT_VERSION`
   diisi tag rilis; `releases/latest/download/install.sh` melayani rilis stabil
   terbaru) dan `SHA256SUMS`, dengan attestation provenance. Bila release sudah
   ada (rerun), berkas diunggah ulang dengan `--clobber`.
4. `image`: membungkus binary musl amd64/arm64 ke image multi-arsitektur
   `ghcr.io/m-fadil/dev-tools` (`Dockerfile`, tanpa kompilasi di Docker)
   memakai `GITHUB_TOKEN` dengan izin `packages: write`. Tag image berasal dari
   tag Git: `v1.0.0` menghasilkan `1.0.0`, `1.0`, `1`, serta `latest` untuk rilis
   stabil. Build menyertakan provenance dan SBOM, dengan attestation di GitHub.
   Visibilitas package GHCR diatur di GitHub; ubah menjadi public bila image harus
   bisa ditarik tanpa login.

Eksekusi manual hanya berjalan bila ref yang dipilih berupa tag versi. Untuk retry
setelah publikasi gagal, rerun workflow tag tersebut. Jangan memindahkan atau
force-push tag rilis. SHA event mengunci sumber tiap run, tetapi tidak membuat tag
registry immutable atau build bit-for-bit: base image dan paket OS masih bisa
berubah saat build ulang.

Verifikasi attestation:

```bash
gh attestation verify --owner m-fadil oci://ghcr.io/m-fadil/dev-tools:1.0.0-rc.2
gh attestation verify --owner m-fadil dt-v1.0.0-rc.2-x86_64-unknown-linux-gnu.tar.gz
```

[Kembali ke dokumentasi](README.md).
