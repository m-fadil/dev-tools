# Utilitas port

Memeriksa port TCP atau mencari port yang masih kosong. Hanya memakai soket
soket TCP bawaan sistem; tidak ada proses lain yang dijalankan.
Exit: 0 sukses, 1 gagal (port tertutup/timeout, tidak ada port kosong, galat soket), 2 penggunaan salah.

## check

```bash
dt port check 5432                      # 127.0.0.1:5432 terbuka
dt port check 443 --host example.com --timeout 3000
```

Mencoba koneksi TCP ke `--host` (default `127.0.0.1`) dengan batas waktu
`--timeout` ms (1-60000, default 1000). Hasil: `<host>:<port> terbuka` (exit 0),
`tertutup` (koneksi ditolak) atau `timeout` (exit 1). Galat soket lain, misalnya
host tidak ditemukan, dilaporkan sebagai error di stderr dengan exit 1.
Port harus bilangan bulat 1-65535. Firewall yang menjatuhkan paket tampil sebagai `timeout`, bukan `tertutup`.

## free

```bash
dt port free                            # port kosong pertama dari 3000
dt port free --from 8000 --to 8100
PORT=$(dt port free --from 4000) ./server
```

Mencari port pertama dalam `--from` (default 3000) sampai `--to` (default 65535),
keduanya 1-65535 dengan `from <= to`, yang bisa di-bind pada `--host`: port benar-benar
di-listen lalu ditutup. Jika tidak ada, exit 1. Port yang ditolak karena izin
(biasanya di bawah 1024 tanpa root) dilewati.

Catatan: port bisa dipakai proses lain antara pencarian dan pemakaian Anda (race).

## Keluaran JSON

Dengan `--json`, stdout berisi satu dokumen JSON; kode keluar tidak berubah.

- `check`: `{ host, port, status }` dengan `status` `open`, `closed`, atau `timeout`
- `free`: `{ host, port }`

Jika `free` tidak menemukan port kosong atau terjadi galat soket, stdout kosong:
tidak ada dokumen JSON; pesan error ditulis ke stderr dengan exit 1.

[Kembali ke dokumentasi](README.md).
