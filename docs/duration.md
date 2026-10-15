# duration

Mengonversi dan menormalkan durasi (`1h30m`, detik, ISO-8601), serta menghitung selisih dua waktu.

```bash
dt duration 1h30m                 # ringkas, ISO-8601, detik, milidetik
dt duration 1d 2h --unit h        # 26
dt duration PT1H30M --unit m      # 90
dt duration -90m                  # durasi negatif
dt duration diff 2026-01-02T00:00:00Z 2026-01-03T06:30:00Z
```

```
dt duration [options] <durasi...>
dt duration diff [options] <a> <b>
```

| Opsi                      | Keterangan                                     |
| ------------------------- | ---------------------------------------------- |
| `-u, --unit <ms\|s\|m\|h\|d\|w>` | Cetak hanya total dalam satuan itu      |
| `--json`                  | Keluaran satu dokumen JSON                     |
| `-h, --help`              | Tampilkan bantuan                              |

## Masukan durasi

Beberapa argumen digabung dengan spasi, jadi `dt duration 1d 2h` sama dengan `dt duration "1d 2h"`.

- **Ringkas**: `1h30m`, `1d 2h`, `90m`, `1w2d`, `1.5h`, `250ms`. Satuan: `w d h m s ms` (huruf kecil). `d` = tepat 24 jam,
  `w` = 7 hari. Urutan bebas (`30m1h` = `1h30m`), tetapi satuan yang sama tidak boleh muncul dua kali (`1h1h` ditolak).
  Angka dan satuannya harus menempel (`1 h` ditolak); antarkomponen boleh dipisah spasi.
- **Angka saja**: dianggap detik (`90` = `90s`, `1.5` = `1s 500ms`).
- **ISO-8601**: `PT1H30M`, `P1DT2H`, `P2W`. Urutan wajib `W`, `D`, lalu `T` diikuti `H`, `M`, `S`; komponen tidak boleh
  berulang dan `T` tanpa komponen waktu ditolak. Huruf besar/kecil diterima, pecahan boleh dengan `.` atau `,`
  (`PT0,5H`). `W` boleh digabung dengan komponen lain (`P1W2D`).
- **Negatif**: awalan `-` berlaku untuk seluruh nilai: `dt duration -90m`, `dt duration -PT5M`. Awalan `+` diterima.
  Nilai berawalan `-` diikuti angka atau `P` dibaca sebagai nilai, bukan opsi.

Tahun dan bulan (`y`, `mo`, `M`, `P1Y`, `P1M`) ditolak karena panjangnya tidak tetap. Di mode ringkas `m` selalu menit;
di ISO-8601 `M` setelah `T` adalah menit.

Presisi maksimum 1 ms. Pecahan yang tidak jatuh tepat pada milidetik (`0.0005s`, `1.5ms`) ditolak, bukan dibulatkan.
Besar maksimum 9223372036854775807 ms (sekitar 292 juta tahun); lebih dari itu ditolak.

## Keluaran

```
$ dt duration 1d 2h
Ringkas:   1d 2h
ISO-8601:  P1DT2H
Detik:     93600
Milidetik: 93600000

$ dt duration -90m
Ringkas:   -1h 30m
ISO-8601:  -PT1H30M
Detik:     -5400
Milidetik: -5400000
```

- **Ringkas**: komponen dari terbesar ke terkecil tanpa komponen nol (`w d h m s ms`); nol = `0s`. `8d` menjadi `1w 1d`.
- **ISO-8601**: hanya `P{n}W` bila hasilnya tepat kelipatan minggu, selain itu hari (`P9D`, `P1DT2H`) agar diterima
  pengurai lain; milidetik menjadi pecahan detik (`PT0.25S`); nol = `PT0S`.
- **Detik** dan **Milidetik**: total bertanda. Semua hitungan memakai bilangan bulat, tanpa galat floating point.

### `--unit`

Mencetak hanya satu angka: total dalam satuan itu.

```
$ dt duration 1h30m --unit h
1.5
$ dt duration PT1H30M --unit m
90
```

Hasil tepat bila ekspansi desimalnya berhingga (nol di belakang dibuang). Bila tidak berhingga (mis. 1 ms dalam menit),
hasil dibulatkan ke 9 digit pecahan (setengah menjauhi nol): `dt duration 1ms --unit m` mencetak `0.000016667`.

## diff

`dt duration diff <a> <b>` menghitung durasi **b - a** (positif bila b lebih akhir).

```
$ dt duration diff 2026-01-02T10:00:00+07:00 2026-01-03T06:30:00Z
Tanda:     +
Ringkas:   1d 3h 30m
ISO-8601:  P1DT3H30M
Detik:     99000
Milidetik: 99000000
```

- Waktu berupa ISO-8601 (`2026-01-02T10:00:00+07:00`, `2026-01-02T03:00:00Z`, `2026-01-02T03:00:00.250Z`) atau epoch
  detik/milidetik (nilai di bawah 1e11 dibaca detik, selain itu milidetik). Parser sama dengan `dt time`.
- Tanpa offset, waktu dibaca sebagai UTC; tanggal saja (`2026-01-02`) adalah tengah malam UTC. Karena semua dihitung
  dalam UTC, tidak ada pengaruh DST: sehari selalu 24 jam.
- `Tanda` bernilai `+`, `-`, atau `0`. `--unit` dan `--json` bekerja seperti pada konversi.
- Epoch negatif boleh ditulis langsung: `dt duration diff -86400 0`.
- Harus tepat dua waktu.

## Keluaran JSON

Satu dokumen JSON; semua total berupa angka desimal tepat.

```
$ dt duration 250ms --json
{
  "input": "250ms",
  "sign": 1,
  "compact": "250ms",
  "iso": "PT0.25S",
  "seconds": 0.25,
  "milliseconds": 250
}
```

- Konversi: `input` (argumen yang sudah digabung), `sign` (`-1`, `0`, `1`), `compact`, `iso`, `seconds`, `milliseconds`.
- `diff`: `a` dan `b` (UTC ISO-8601, ternormalisasi) menggantikan `input`, sisanya sama.

```
$ dt duration diff 1767323045 1767323105 --json
{
  "a": "2026-01-02T03:04:05.000Z",
  "b": "2026-01-02T03:05:05.000Z",
  "sign": 1,
  "compact": "1m",
  "iso": "PT1M",
  "seconds": 60,
  "milliseconds": 60000
}
```

Dengan `--unit`, JSON menambah `unit` (nama satuan) dan `value` (angka seperti keluaran `--unit`).

## Kode keluar

0 sukses; 2 penggunaan salah atau nilai tidak valid (durasi kosong, sintaks salah, tahun/bulan, satuan berulang,
presisi di bawah 1 ms, melebihi batas, waktu `diff` tidak valid, `--unit` tidak dikenal). Kode 1 dicadangkan untuk
kegagalan saat berjalan dan tidak dipakai oleh kasus di atas.

```
$ dt duration 1y
Satuan y tidak didukung: tahun dan bulan panjangnya tidak tetap. Pakai w, d, h, m, s, ms.
```

[Kembali ke dokumentasi](README.md).
