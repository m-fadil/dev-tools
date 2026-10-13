# dev-tools

Utilitas command-line untuk menyederhanakan pekerjaan pengembangan dan maintenance.
Repo ini menyediakan satu command, `dt`, dengan antarmuka terminal interaktif
serta opsi eksekusi langsung untuk penggunaan berulang dan otomatisasi.

Implementasi menggunakan TypeScript dan `@inquirer/prompts`. Setiap command
memiliki tanggung jawab tersendiri, sementara menu, bantuan, dan dispatch dikelola
melalui registry bersama.

## Instalasi

Memerlukan Node.js `^22.13.0 || >=23.5.0` (Node 24 direkomendasikan).
Prasyarat tambahan dijelaskan pada panduan masing-masing command.

```bash
cd /path/to/dev-tools
npm ci
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
