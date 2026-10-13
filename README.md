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
npx 'm-fadil/dev-tools#semver:*'       # menu interaktif, tag stabil terbaru
npx m-fadil/dev-tools#v0.1.1 syncgit -n # versi tertentu, setelah tag diterbitkan
```

Atau pasang secara global:

```bash
npm pack m-fadil/dev-tools#v0.1.1      # membangun paket dari tag sumber
npm install -g ./m-fadil-dev-tools-0.1.1.tgz
```

Paket global npm biasa: terpasang di prefix npm (`npm prefix -g`) dengan
`node_modules` sendiri, dan `dt` masuk ke `PATH`. Jalankan ulang perintah
yang sama untuk memperbarui. Hapus dengan `npm uninstall -g @m-fadil/dev-tools`.

Rilis baru memakai tag `vX.Y.Z` pada commit kode sumber. npm menjalankan `prepare`
untuk membangun TypeScript sebelum mengemas paket; compiler diambil sebagai
devDependency saat persiapan, bukan prasyarat global. Jangan memakai `--ignore-scripts`
untuk instalasi dari Git. Ref `#semver:*` memilih tag stabil terbaru, bukan `main`.

Untuk instalasi global, kemas dahulu dengan `npm pack`, lalu instal tarball.
Instalasi Git langsung dengan `npm install -g` gagal pada npm 11.19 karena
persiapan dependency mewarisi mode global. Tarball sudah berisi hasil build.
Hapus file `.tgz` setelah instalasi bila tidak diperlukan.

Tag lama tetap berlaku; branch distribusi `release` tidak lagi diperbarui.

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

## Docker

Image rilis: `ghcr.io/m-fadil/dev-tools`, mendukung `linux/amd64` dan `linux/arm64`.
Image tersedia setelah workflow release berhasil memublikasikannya.

```bash
docker run --rm ghcr.io/m-fadil/dev-tools:latest --help

# Mount repo dengan UID/GID pemiliknya; -it diperlukan untuk menu/wizard.
docker run --rm -it --user "$(id -u):$(id -g)" \
  -v "$PWD:/workspace" ghcr.io/m-fadil/dev-tools:latest
```

Image menyertakan Node 24, Git, OpenSSH client, dan sertifikat CA; default user
`node` (UID 1000), direktori kerja `/workspace`. Argumen diteruskan ke `dt`.
Untuk repo yang memerlukan autentikasi, sediakan kredensial HTTPS atau SSH
agent/config/known_hosts melalui mount terpisah; kredensial host tidak otomatis
tersedia. Repo dan mailmap harus bisa dibaca/ditulis oleh user container.

Build lokal:

```bash
docker build -t dev-tools:local .
docker run --rm dev-tools:local syncgit --help
```

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
