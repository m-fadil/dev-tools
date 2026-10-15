# fake

Membuat data palsu untuk pengujian. Data dibuat acak untuk pengujian (lihat catatan telepon) dan tidak aman untuk keperluan kriptografi.

```
dt fake <jenis> [-c n] [--locale id|en] [--seed n] [opsi jenis] [--json]
```

Tanpa jenis, daftar jenis ditampilkan (kode keluar 0). Jenis tidak dikenal menghasilkan kode keluar 2.

`dt fake -h` menampilkan opsi khusus berkelompok di bawah jenis masing-masing.
`dt fake <jenis> -h` hanya menampilkan opsi umum dan opsi jenis tersebut,
misalnya `dt fake email -h`. Jenis tidak dikenal tetap ditolak saat meminta bantuan.

Jenis: `name`, `firstname`, `lastname`, `email`, `username`, `phone`, `company`, `address`, `city`, `country`, `ip`,
`ipv6`, `mac`, `uuid`, `date`, `datetime`, `number`, `bool`, `color`, `url`, `word`, `sentence`, `hex`.

## Opsi umum

| Opsi              | Keterangan                                                                       |
| ----------------- | -------------------------------------------------------------------------------- |
| `-c, --count <n>` | Jumlah hasil, 1-1000 (default: 1), satu per baris                                |
| `--locale <l>`    | `id` atau `en` (default: `en`)                                                   |
| `--seed <n>`      | Seed 0-4294967295 agar hasil dapat diulang (tanpa seed: acak dari `node:crypto`) |
| `--json`          | Keluaran JSON                                                                    |
| `-h, --help`      | Bantuan umum atau terfokus sesuai jenis                                          |

## Opsi khusus

### email

- `--domain <domain>`: default `example.com`, berlaku hanya untuk `email`.
- Masukkan domain saja, misalnya `mail.perusahaan.test`, tanpa `@`, URL, port,
  atau path. Domain harus memiliki setidaknya dua label yang tidak kosong;
  setiap label maksimal 63 karakter ASCII dan total maksimal 253.
- Domain diubah ke huruf kecil; domain internasional dikonversi ke ASCII
  (contoh `bücher.test` menjadi `xn--bcher-kva.test`).
- Tidak memeriksa DNS atau mengirim email. Domain nyata dapat menghasilkan
  alamat milik orang nyata; gunakan domain contoh untuk data uji.
- Nama pengguna tetap dibuat dari nama acak. Seed dan opsi yang sama menghasilkan
  hasil yang sama; mengganti domain tidak mengubah nama pengguna.

### date, datetime

- `--from`, `--to`: batas ISO 8601 tanpa pecahan detik; tanpa zona berarti UTC.
  Default 10 tahun terakhir sampai sekarang.

### number

- `--min`, `--max`: batas bilangan bulat; nilai negatif ditulis `--min=-5`.
- Default `--min` adalah 0; tanpa `--max`, batas atas adalah max(1000, `--min`).
- Selisih `--max` - `--min` maksimal 4294967295 (2^32-1); lebih besar ditolak.

### hex

- `--length <n>`: jumlah karakter hex, 1-1024 (default: 16).

Opsi khusus jenis dipakai pada jenis lain ditolak (kode keluar 2).

## Sifat data

- Email default memakai `example.com`; domain bisa diganti lewat `--domain`. URL tetap memakai domain `example.*`.
- Telepon `en` berbentuk `+1 AAA-555-01xx` (rentang fiktif NANP). Telepon `id` berbentuk `+62 8xx-xxxx-xxxx`: hanya
  mirip format nomor seluler, bisa kebetulan milik pelanggan nyata; jangan dipakai untuk menghubungi siapa pun.
- `--from`/`--to` tidak menerima pecahan detik (exit 2), agar milidetik tidak dibuang diam-diam.
- `ip` dari 192.0.2.0/24, 198.51.100.0/24, 203.0.113.0/24; `ipv6` dari 2001:db8::/32.
- `mac` bertipe unicast yang dikelola lokal. `uuid` berversi 4 (dari PRNG berseed, bukan CSPRNG; untuk UUID acak gunakan `dt random uuid`).
- `date` berformat `YYYY-MM-DD`, `datetime` berformat `YYYY-MM-DDTHH:MM:SSZ`.
- Karena default `--from`/`--to` bergantung pada waktu sekarang, beri keduanya agar `date` benar-benar reproduktif.

```
dt fake name -c 5 --locale id
dt fake number --min 10 --max 20 --seed 7
dt fake email -c 5
dt fake email --domain mail.perusahaan.test -c 5 --seed 42
dt fake email --domain example.org --json
dt fake email -h
```

## Keluaran JSON

```json
{ "type": "number", "locale": "en", "values": [412, 7, 903] }
```

- `type`: jenis yang diminta
- `locale`: `id` atau `en`
- `values`: larik hasil; `number` berupa angka, `bool` berupa boolean, jenis lain berupa string

[Kembali ke dokumentasi](README.md).
