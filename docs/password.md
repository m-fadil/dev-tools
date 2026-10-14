# password

```bash
dt password                       # 20 karakter: huruf kecil, besar, angka
dt password -s -l 32              # tambah simbol, 32 karakter
dt password -c 5 -u               # lima password tanpa karakter mirip
dt password --only hex -l 64 -e   # hex 64 karakter, entropi ke stderr
```

Password dibuat dari CSPRNG `node:crypto`; setiap pilihan karakter memakai
`crypto.randomInt` (tanpa modulo bias). Hasil ditulis ke stdout, satu per baris.

| Opsi                  | Keterangan                                 |
| --------------------- | ------------------------------------------ |
| `-l`, `--length <n>`  | Panjang, 4-1024 (default 20)               |
| `-c`, `--count <n>`   | Jumlah password, 1-1000 (default 1)        |
| `-s`, `--symbols`     | Tambah simbol `!@#$%^&*()-_=+[]{};:,.?`    |
| `--only <set>`        | `lower`, `upper`, `digits`, `alnum`, `hex` |
| `-u`, `--unambiguous` | Buang `0 O 1 l I \|` dari alfabet          |
| `-e`, `--entropy`     | Cetak `Entropi: ~N bit` ke stderr          |
| `--json`              | Keluaran JSON tunggal ke stdout            |

`--only` membatasi karakter dan tidak bisa digabung dengan `--symbols`. `alnum`
sama dengan default (huruf kecil + besar + angka); `hex` adalah `0-9a-f`.
Dengan `--unambiguous`, `hex` menjadi `2-9a-f` dan `digits` menjadi `2-9`.

Tiap jenis karakter aktif (huruf kecil, besar, angka, simbol) dijamin muncul
minimal satu kali: satu karakter per jenis ditaruh dulu, sisanya diisi dari
seluruh alfabet, lalu urutannya diacak (Fisher-Yates). `hex` dan opsi `--only`
satu jenis tidak punya jaminan tambahan. Karena itu panjang minimal 4 selalu
cukup untuk semua jenis.

Entropi dihitung sebagai `panjang * log2(ukuran alfabet)`. Ini batas atas:
jaminan satu karakter per jenis sedikit menguranginya. Default 20 karakter
sekitar 119 bit. Tanpa `--entropy` stderr kosong, jadi stdout aman dipipe.

Exit: 0 sukses, 2 penggunaan salah (nilai di luar rentang, `--only` bersama
`--symbols`, argumen tidak dikenal).

## Keluaran JSON

`{ values: string[], entropyBits?: number }`. `entropyBits` (batas atas, tanpa
pembulatan) hanya ada bersama `--entropy`; dalam mode ini baris stderr tidak dicetak.

[Kembali ke dokumentasi](README.md).
