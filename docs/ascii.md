# ascii

Tabel ASCII dan info satu karakter.

```bash
dt ascii               # tabel 0-127
dt ascii A             # karakter
dt ascii 65            # kode desimal
dt ascii 0x0a          # hex (LF)
dt ascii é             # non-ASCII: U+00E9, UTF-8 C3 A9
```

`nilai` adalah satu karakter, kode desimal, atau hex berawalan `0x`. Satu
karakter diutamakan: `5` berarti karakter `5` (kode 53); untuk kode 0-9 pakai
hex (`0x05`). Kode sampai `0x10FFFF`; surrogate (`0xD800-0xDFFF`) dan kode di luar
rentang ditolak (exit 1). Lebih dari satu nilai, atau `--table` bersama nilai,
adalah galat penggunaan (exit 2).

## Keluaran

- Tabel (`--table`, default tanpa nilai): 4 kolom `dec hx oct chr` (desimal, hex,
  oktal, karakter atau nama kontrol seperti `NUL`, `LF`, `SP`, `DEL`).
- Lookup: desimal, hex, oktal, biner, nama + deskripsi singkat untuk karakter
  kontrol (`LF` = Line Feed: baris baru), atau karakter itu. Karakter non-ASCII
  juga menampilkan code point (`U+XXXX`) dan byte UTF-8. Karakter yang tidak
  tercetak (misalnya kontrol C1) ditandai `(tidak tercetak)`.

## Opsi

- `-t, --table`: tampilkan tabel.
- `--json`: keluaran JSON.

## Keluaran JSON

Lookup: `{ decimal, hex, octal, binary, unicode, utf8, ascii, character, name, description }`

- `hex`: `"0x41"`; `octal`: 3 digit; `binary`: minimal 8 digit; `unicode`: `"U+0041"`.
- `utf8`: array byte hex, mis. `["C3", "A9"]`; `ascii`: boolean.
- `character`: karakternya, `null` bila kontrol/tidak tercetak.
- `name`, `description`: singkatan dan deskripsi kontrol (`NUL`..`US`, `SP`, `DEL`); selain itu `null`.

Tabel: `{ "table": [ ...128 objek dengan bentuk lookup ] }`.

[Kembali ke dokumentasi](README.md).
