# dev-tools

Kumpulan alat praktis untuk pekerjaan sehari-hari, cukup dengan perintah `dt`.

## Instalasi

Untuk pilihan Node.js, siapkan [Node.js 24](https://nodejs.org/) dan
[Git](https://git-scm.com/). Untuk pilihan Docker, cukup siapkan Docker.

### Dari Git clone

```bash
git clone https://github.com/m-fadil/dev-tools.git
cd dev-tools
npm ci
npm link
```

### Langsung dengan npx

Tanpa memasang secara global:

```bash
npx --package=github:m-fadil/dev-tools#release dt --help
npx --package=github:m-fadil/dev-tools#release dt hash "halo"
```

### Pasang dengan npm

Pasang versi siap pakai langsung:

```bash
npm install -g github:m-fadil/dev-tools#release
dt --help
```

Untuk memperbarui, jalankan lagi perintah instalasi yang sama. Paket diambil
dari GitHub; tidak perlu mengemas atau membangunnya sendiri.

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
docker run --rm ghcr.io/m-fadil/dev-tools:latest uuid
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
dt hash "halo"             # buat hash
dt uuid                    # buat ID acak
dt password                # buat password
dt json --file data.json   # rapikan JSON
dt time                    # lihat waktu sekarang
```

Untuk panduan setiap alat, jalankan `dt <nama> --help`, misalnya `dt encrypt --help`.

## Fitur

- Hash, password, ID acak, dan enkripsi.
- Encode/decode, JSON, CSV, dan pengolahan teks.
- Waktu, jadwal, warna, ukuran data, dan perhitungan.
- Pemeriksaan jaringan, gambar, file, dan sinkronisasi Git.

Contoh lengkap tersedia di [dokumentasi](docs/README.md).

Untuk menghapus instalasi:

```bash
npm uninstall -g @m-fadil/dev-tools
```
