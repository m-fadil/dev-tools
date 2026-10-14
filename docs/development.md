# Pengembangan dan maintenance

## Struktur kode

```text
src/
  cli.ts                 Entry point, menu/help, dispatch
  errors.ts              UsageError (exit 2)
  tool.ts                Contract Tool { name, description, run(args) }
  tools/
    index.ts             Registry command
    <nama>/
      index.ts           Opsi dan prompt
      <logika>.ts        Logika operasi (mis. syncgit/sync.ts)
```

Setiap command mengelola opsi, prompt, dan implementasinya sendiri. Registry
menjadi sumber yang sama untuk menu, bantuan, dan dispatch.

## Menambahkan command

1. Buat `src/tools/<nama>/index.ts` yang mengekspor objek `Tool`.
2. Implementasikan `name`, `description`, `category` (kelompok di bantuan dan menu),
   opsional `aliases`, dan `run(args)` sesuai contract `src/tool.ts`. Nama dan alias
   harus unik (dijaga `test/registry.test.ts`). Pisahkan logika operasi dari parsing opsi dan prompt.
   Tool yang menghasilkan data terstruktur sebaiknya menyediakan `--json`: satu
   dokumen JSON dari `printJson` (`src/io.ts`) di stdout, diagnostik tetap di stderr.
3. Import objek tersebut dan daftarkan dalam array `tools` di `src/tools/index.ts`.
4. Tambahkan panduan penggunaan dalam `docs/` dan tautan di `docs/README.md`.
5. Tambahkan integration test untuk perilaku yang terlihat oleh pengguna.

## Build dan verifikasi

```bash
npm run build
npm test
npm run format        # rapikan dengan Prettier
npm run format:check  # periksa tanpa mengubah file
```

`npm test` menghapus `dist/`, lalu menjalankan build TypeScript strict dan test runner
bawaan Node.js, sehingga test yang sudah dihapus tidak ikut berjalan dari build lama.
Gunakan lingkungan sementara dan data contoh agar test terisolasi dari repo,
identitas, dan kredensial pengguna.

CI (`.github/workflows/test.yml`) menjalankan `npm ci`, `npm run format:check`, dan
`npm test` pada Node 22 dan 24 untuk setiap push ke `main` dan pull request.
Job container membangun image AMD64 dan ARM64 secara terpisah (ARM64 melalui
QEMU), lalu menjalankan bantuan CLI, bantuan syncgit, serta pemeriksaan Git,
SSH, dan user non-root. CI ini tidak memublikasikan image.

Susunan test:

- `test/cli.test.ts` — menjalankan `dist/src/cli.js` tanpa terminal, seperti cron/CI:
  bantuan, error penggunaan, dan kegagalan cepat ketika prompt membutuhkan TTY.
- `test/<nama>-args.test.ts` — unit test parsing opsi dan default (fungsi murni,
  tanpa Git atau prompt).
- `test/<nama>.test.ts` — integration test perilaku command terhadap repo sementara.

Command `dt` dari package `@m-fadil/dev-tools` yang dipasang melalui `npm link`
memakai hasil build di `dist/src/`.
Tidak perlu melakukan link ulang setelah perubahan kode; cukup build ulang.
Untuk menjalankan tanpa global link:

```bash
npm start -- --help
```

Prompt interaktif perlu diverifikasi di terminal nyata. Pengujian noninteraktif
harus memakai opsi eksplisit tanpa memerlukan input terminal.

## Rilis

Rilis dipicu oleh push tag `vX.Y.Z` pada commit sumber, bukan perubahan versi
di branch `main`. Dari working tree bersih setelah perubahan siap dirilis:

```bash
npm version patch            # atau minor / major; mengubah package.json + lock, membuat commit dan tag
git push origin main         # setelah mendapat persetujuan untuk push
git push origin "v$(node -p 'require("./package.json").version')"
```

Workflow `release` (`.github/workflows/release.yml`) checkout `${{ github.sha }}`
pada event tag, memastikan nama tag sama dengan `v<package.json version>`,
lalu menjalankan `format:check` dan `npm test`. Jika verifikasi gagal, distribusi
dan image tidak dipublikasikan. Push branch saja tidak menerbitkan rilis.

Job `image` checkout SHA event yang sama dan memublikasikan manifest
multi-arsitektur (`linux/amd64`, `linux/arm64`) ke `ghcr.io/m-fadil/dev-tools`
memakai `GITHUB_TOKEN` dengan izin `packages: write`. Metadata image berasal
dari tag: misalnya `v0.1.1` menghasilkan `0.1.1`, `0.1`, `0`, serta `latest`
untuk rilis stabil. Prerelease tidak memperbarui `latest`.
Build menyertakan provenance dan SBOM, dengan attestation di penyimpanan GitHub.
Visibilitas package GHCR diatur di GitHub; ubah menjadi public bila image harus
bisa ditarik tanpa login.

Eksekusi manual hanya berjalan bila ref yang dipilih berupa tag versi; branch
dilewati. Untuk retry setelah publikasi gagal, rerun workflow tag tersebut.
Jangan memindahkan atau force-push tag rilis. SHA event mengunci sumber tiap
run, tetapi tidak membuat tag registry immutable atau build bit-for-bit:
base image dan paket OS masih bisa berubah saat build ulang.

Setelah verifikasi, `scripts/prepare-release.mjs` menyalin `dist/src`, README,
dokumentasi, dan metadata npm ke artifact distribusi. Package distribusi tidak
memuat `scripts` atau `devDependencies`, jadi instalasi Git global tidak
menjalankan `prepare` dan tidak memerlukan compiler.

Job `distribution` memakai artifact tersebut untuk memperbarui branch `release`
pada tag stabil saja. Commit mencatat tag dan SHA sumber, mempertahankan riwayat
branch, dan memakai push biasa (bukan force-push). Izin `contents: write` hanya
diberikan ke job ini. Tag sumber tidak diubah atau dibuat ulang. Bila aturan
proteksi branch melarang push bot, sesuaikan aturan untuk branch distribusi.

Checkout kode sumber tetap memakai `prepare` untuk membangun TypeScript.
Docker memasang dependency dengan `--ignore-scripts`, membangun TypeScript
secara eksplisit di stage build, dan menyalin hasilnya ke stage runtime.

Verifikasi attestation:

```bash
gh attestation verify --owner m-fadil oci://ghcr.io/m-fadil/dev-tools:0.1.1
```

Setelah rilis baru memperbarui branch distribusi, instalasi siap pakai:

```bash
npm install -g github:m-fadil/dev-tools#release
npx --package=github:m-fadil/dev-tools#release dt --help
```

[Kembali ke dokumentasi](README.md).
