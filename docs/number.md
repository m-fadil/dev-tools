# Matematika number

Alias: `num`.

```bash
dt number 255
dt number 0xFF_FF --to 10
dt number -f 2 1010 -t 16 --prefix
dt number -1 -w 16
dt number 123456789012345678901234567890 --json
echo 0b1010 | dt number
```

Mengubah bilangan bulat antar basis 2, 8, 10, dan 16 dengan ukuran bebas
(BigInt). Nilai boleh berprefix `0x` (hex), `0b` (biner), `0o` (oktal) atau
desimal polos, bertanda `-`/`+`, dan memakai pemisah `_` di antara digit.
Tanpa argumen nilai dibaca dari stdin. Pecahan ditolak.

Keluaran default: baris `Desimal`, `Hex`, `Biner`, `Oktal` (dengan prefix).
Bilangan negatif tampil dengan tanda di depan prefix, mis. `-0xff`. Nilai
negatif seperti `-5` ditulis langsung sebagai argumen.

Opsi:

- `-f, --from <auto|2|8|10|16>`: basis input; default `auto` (menurut prefix,
  tanpa prefix desimal). Dengan basis eksplisit, prefix yang sama boleh ada;
  `--from 16 0b1` membaca `0b1` sebagai digit hex.
- `-t, --to <2|8|10|16>`: cetak hanya basis itu, tanpa prefix.
- `--prefix`: dengan `--to`, sertakan prefix `0x`/`0b`/`0o`.
- `-g, --group`: kelompokkan biner per nibble (4 bit) dan hex per byte (2 digit)
  dengan `_`, diisi nol di kiri. Desimal tidak dikelompokkan.
- `-w, --width <8|16|32|64>`: tampilkan juga komplemen dua dengan lebar itu,
  diisi nol penuh. Nilai harus muat dalam rentang bertanda (-2^(w-1)..2^(w-1)-1)
  atau tak bertanda (sampai 2^w-1); jika tidak, galat. Dengan `--to`, yang
  dicetak adalah representasi komplemen dua dalam basis itu.
- `--json`: keluaran JSON. Tidak bisa digabung dengan `--to`, `--prefix`, `--group`.

Nilai negatif langsung setelah opsi yang membutuhkan nilai tetap menjadi
nilai opsi tersebut, bukan bilangan input. Misalnya `number -w -8 5` ditolak
karena lebar `-8` tidak valid; `number -5 -t 10` tetap mengonversi bilangan `-5`.

Exit: 0 sukses, 2 angka/opsi tidak valid atau nilai tidak muat dalam `--width`.

## Keluaran JSON

Angka berupa string (BigInt tak muat di number JSON).

- `input`: teks nilai.
- `base`: basis yang dipakai (2, 8, 10, atau 16).
- `negative`: `true` bila nilai < 0.
- `decimal`, `hex`, `binary`, `octal`: representasi dengan prefix (dan tanda).
- `twosComplement` (hanya dengan `--width`): `{ width, decimal, hex, binary, octal }`
  untuk pola bit tak bertanda, hex/biner/oktal diisi nol penuh.

[Kembali ke dokumentasi](README.md).
