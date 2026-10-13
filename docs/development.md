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
2. Implementasikan `name`, `description`, dan `run(args)` sesuai contract
   `src/tool.ts`. Pisahkan logika operasi dari parsing opsi dan prompt.
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

Susunan test:

- `test/cli.test.ts` — menjalankan `dist/src/cli.js` tanpa terminal, seperti cron/CI:
  bantuan, error penggunaan, dan kegagalan cepat ketika prompt membutuhkan TTY.
- `test/<nama>-args.test.ts` — unit test parsing opsi dan default (fungsi murni,
  tanpa Git atau prompt).
- `test/<nama>.test.ts` — integration test perilaku command terhadap repo sementara.

Command yang dipasang melalui `npm link` memakai hasil build di `dist/src/`.
Tidak perlu melakukan link ulang setelah perubahan kode; cukup build ulang.
Untuk menjalankan tanpa global link:

```bash
npm start -- --help
```

Prompt interaktif perlu diverifikasi di terminal nyata. Pengujian noninteraktif
harus memakai opsi eksplisit tanpa memerlukan input terminal.

## Rilis

Rilis dibuat dengan menaikkan versi di `main`:

```bash
npm version patch            # atau minor / major; mengubah package.json + lock, tanpa commit/tag (.npmrc)
git commit -am "release v$(node -p 'require("./package.json").version')"
git push
```

Workflow `release` (`.github/workflows/release.yml`) mendeteksi versi baru,
menjalankan `format:check` dan `npm test`, lalu `.github/scripts/publish-release.sh`
meng-commit hasil build ke branch `release` (satu commit per versi, pesan mencatat
commit sumber) dan memberi tag `vX.Y.Z`. Isi commit rilis hanya `dist/src`,
`package.json` tanpa `scripts`/`devDependencies`, `README.md`, dan `docs/`, sehingga
npm memasangnya dari git tanpa build. Jangan membuat tag sendiri: tag milik commit
rilis, dan versi yang sudah bertag tidak dirilis ulang. Workflow juga bisa dijalankan
manual dari tab Actions.

Pasang atau jalankan hasil rilis:

```bash
npx m-fadil/dev-tools#release
npm install -g m-fadil/dev-tools#vX.Y.Z
```

[Kembali ke dokumentasi](README.md).
