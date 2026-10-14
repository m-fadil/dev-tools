# fake

Membuat data palsu untuk pengujian. Data dibuat acak untuk pengujian (lihat catatan telepon) dan tidak aman untuk keperluan kriptografi.

```
dt fake <jenis> [-c n] [--locale id|en] [--seed n] [opsi jenis] [--json]
```

Tanpa jenis, daftar jenis ditampilkan (kode keluar 0). Jenis tidak dikenal menghasilkan kode keluar 2.

Jenis: `name`, `firstname`, `lastname`, `email`, `username`, `phone`, `company`, `address`, `city`, `country`, `ip`,
`ipv6`, `mac`, `uuid`, `date`, `datetime`, `number`, `bool`, `color`, `url`, `word`, `sentence`, `hex`.

| Opsi              | Keterangan                                                                                                                 |
| ----------------- | -------------------------------------------------------------------------------------------------------------------------- |
| `-c, --count <n>` | Jumlah hasil, 1-1000 (default: 1), satu per baris                                                                          |
| `--locale <l>`    | `id` atau `en` (default: `en`)                                                                                             |
| `--seed <n>`      | Seed 0-4294967295 agar hasil dapat diulang (tanpa seed: acak dari `node:crypto`)                                           |
| `--from`, `--to`  | Batas `date`/`datetime`, ISO 8601 tanpa pecahan detik (default: 10 tahun terakhir sampai sekarang, UTC)                    |
| `--min`, `--max`  | Batas `number`, bilangan bulat (default: 0-1000); nilai negatif: `--min=-5`                                                |
|                   | Tanpa `--max`, batas atas = max(1000, `--min`); selisih `--max` - `--min` maks. 4294967295 (2^32-1), lebih dari itu exit 2 |
| `--length <n>`    | Panjang `hex`, 1-1024 (default: 16)                                                                                        |
| `--json`          | Keluaran JSON                                                                                                              |

Opsi khusus jenis dipakai pada jenis lain ditolak (kode keluar 2).

## Sifat data

- Email diturunkan dari nama dan hanya memakai `example.com`, `example.org`, `example.net`; URL memakai domain `example.*`.
- Telepon `en` berbentuk `+1 AAA-555-01xx` (rentang fiktif NANP). Telepon `id` berbentuk `+62 8xx-xxxx-xxxx`: hanya
  mirip format nomor seluler, bisa kebetulan milik pelanggan nyata; jangan dipakai untuk menghubungi siapa pun.
- `--from`/`--to` tidak menerima pecahan detik (exit 2), agar milidetik tidak dibuang diam-diam.
- `ip` dari 192.0.2.0/24, 198.51.100.0/24, 203.0.113.0/24; `ipv6` dari 2001:db8::/32.
- `mac` bertipe unicast yang dikelola lokal. `uuid` berversi 4 (dari PRNG berseed, bukan CSPRNG; untuk UUID aman pakai `uuid`).
- `date` berformat `YYYY-MM-DD`, `datetime` berformat `YYYY-MM-DDTHH:MM:SSZ`.
- Karena default `--from`/`--to` bergantung pada waktu sekarang, beri keduanya agar `date` benar-benar reproduktif.

```
dt fake name -c 5 --locale id
dt fake number --min 10 --max 20 --seed 7
```

## Keluaran JSON

```json
{ "type": "number", "locale": "en", "values": [412, 7, 903] }
```

- `type`: jenis yang diminta
- `locale`: `id` atau `en`
- `values`: larik hasil; `number` berupa angka, `bool` berupa boolean, jenis lain berupa string

[Kembali ke dokumentasi](README.md).
