# dev-tools

Utilitas command-line untuk menyederhanakan pekerjaan pengembangan dan maintenance.
Repo ini menyediakan satu command, `dt`, dengan antarmuka terminal interaktif
serta opsi eksekusi langsung untuk penggunaan berulang dan otomatisasi.

Implementasi menggunakan TypeScript dan `@inquirer/prompts`. Setiap command
memiliki tanggung jawab tersendiri, sementara menu, bantuan, dan dispatch dikelola
melalui registry bersama.

## Instalasi

Memerlukan Node.js `^22.13.0 || >=23.5.0` (Node 24 direkomendasikan), npm, dan
`git` (npm memakai git untuk mengambil ref GitHub). Prasyarat tambahan dijelaskan
pada panduan masing-masing command.

Jalankan tanpa instalasi:

```bash
npx m-fadil/dev-tools#release            # menu interaktif, rilis terbaru
npx m-fadil/dev-tools#v0.1.0 syncgit -n  # versi tertentu
```

Atau pasang secara global:

```bash
npm install -g m-fadil/dev-tools#release   # atau #v0.1.0 untuk mengunci versi
```

Paket global npm biasa: terpasang di prefix npm (`npm prefix -g`) dengan
`node_modules` sendiri, dan `dt` masuk ke `PATH`. Jalankan ulang perintah
yang sama untuk memperbarui. Hapus dengan `npm uninstall -g @m-fadil/dev-tools`.

`release` adalah branch yang hanya berisi hasil build, satu commit per versi,
masing-masing diberi tag `vX.Y.Z`; tidak ada proses kompilasi di mesin Anda.
`main` adalah kode sumber dan tidak bisa dipasang langsung.

Bila `npm prefix -g` dimiliki root (Node sistem), awali perintah dengan `sudo`
atau pindahkan prefix sekali saja:

```bash
npm config set prefix ~/.npm-global      # lalu tambahkan ~/.npm-global/bin ke PATH
```

Dari checkout (untuk pengembangan):

```bash
cd /path/to/dev-tools
npm ci
npm run build
npm link
```

## Menjalankan

```bash
# Buka menu interaktif
dt

# Tampilkan bantuan
dt --help
```

Command dapat dijalankan langsung beserta opsinya tanpa melalui menu. Panduan
penggunaan, contoh, dan batas keselamatan tersedia di [dokumentasi](docs/README.md).

## Pengembangan

```bash
npm run build
npm test
npm run format:check
```

Build memperbarui command yang terhubung melalui `npm link`. Verifikasi memakai
TypeScript strict dan integration test dengan lingkungan sementara yang terisolasi.

Struktur kode dan panduan maintenance tersedia di
[panduan pengembangan](docs/development.md).
