# qr

Membuat QR code dari teks, file, atau stdin. Keluaran berupa blok Unicode di terminal, teks ASCII, atau SVG.

```
dt qr [teks | --file <path>] [-l L|M|Q|H] [-m <n>] [--format unicode|ascii|svg] [--invert] [--json]
```

| Opsi                | Keterangan                                                            |
| ------------------- | --------------------------------------------------------------------- |
| `-f, --file <path>` | Kodekan isi file (tidak boleh bersama teks)                           |
| `-l, --level <L>`   | Koreksi kesalahan `L`, `M`, `Q`, atau `H` (default: `M`)              |
| `-m, --margin <n>`  | Zona tenang dalam modul, 0-8 (default: 2)                             |
| `--format <jenis>`  | `unicode` (default), `ascii`, atau `svg`                              |
| `--invert`          | Tukar polaritas unicode/ascii, untuk terminal berlatar terang         |
| `--json`            | Keluaran JSON berisi matriks modul                                    |

Tanpa teks dan `--file`, input dibaca dari stdin. Byte input dikodekan apa adanya, termasuk newline akhir dari stdin
(`echo` menambahkannya; gunakan `printf` atau `echo -n` bila tidak diinginkan). Input kosong menghasilkan kode keluar 2.

## Polaritas

Default cocok untuk terminal **gelap**: modul terang (dan zona tenang) digambar sebagai blok penuh, sehingga kode tampak
terang di atas latar gelap. Di terminal terang tambahkan `--invert` agar modul gelap yang berupa blok penuh. Format `svg`
selalu hitam di atas putih dan mengabaikan `--invert`. Pemindai modern membaca kedua polaritas, tetapi polaritas yang
salah untuk latar terminal kadang gagal dipindai.

Format unicode memakai `█ ▀ ▄` dan spasi, dua baris modul per baris teks. Format ascii memakai `##` (terisi) dan dua spasi
per modul, dengan aturan polaritas yang sama.

## Contoh

```
$ dt qr -m 1 --invert hi
 ▄▄▄▄▄▄▄  ▄▄▄▄ ▄▄▄▄▄▄▄ 
 █ ▄▄▄ █ ▄█▀▄█ █ ▄▄▄ █ 
 █ ███ █ █▀ ▄█ █ ███ █ 
 █▄▄▄▄▄█ █▀▄ █ █▄▄▄▄▄█ 
 ▄ ▄▄▄▄▄ ▀ ▀▀█ ▄▄▄▄▄   
  ▀██ ▀▄█ ▄▀▄▀▄ ▀▄▄▄▄▀ 
 ███ █ ▄  ▀ ▄ ▄ ▀█ █ ▄ 
 ▄▄▄▄▄▄▄ ▀ ▄▀█▀█▄ ▀ ▄▀ 
 █ ▄▄▄ █ █▀▀▀█▀▀▄ ▀▄ ▀ 
 █ ███ █ █▀▀▄▀▄ ▀▄▄█   
 █▄▄▄▄▄█ ▄▀█▄ ▄ ▀█▄█▄  
                       

$ dt qr --format ascii --invert -m 0 hi | head -3
##############    ########  ##############
##          ##    ####  ##  ##          ##
##  ######  ##  ####  ####  ##  ######  ##

$ dt qr --format svg -m 0 hi > hi.svg
$ head -5 hi.svg
<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 21 21" width="168" height="168" shape-rendering="crispEdges">
<rect width="21" height="21" fill="#fff"/>
<g fill="#000">
<rect x="0" y="0" width="1" height="1"/>
```

SVG berdiri sendiri (tanpa referensi eksternal): satu `<rect>` latar putih dan satu `<rect>` hitam per modul gelap,
1 satuan viewBox = 1 modul.

## Kapasitas

Input terlalu besar untuk level yang dipilih menghasilkan kode keluar 1. Kapasitas maksimum data biner (versi 40):
`L` 2953, `M` 2331, `Q` 1663, `H` 1273 byte. Teks numerik atau alfanumerik murni dapat lebih panjang.

```
$ dt qr -l H "$(printf 'a%.0s' $(seq 1274))"
Input terlalu besar: 1274 byte; kapasitas maksimum level H adalah 1273 byte.
$ echo $?
1
```

## Keluaran JSON

```json
{
  "text_bytes": 2,
  "version": 1,
  "level": "M",
  "width": 21,
  "modules": [
    "111111100111101111111",
    "100000100110101000001",
    "..."
  ]
}
```

- `text_bytes`: jumlah byte input
- `version`: versi QR 1-40; `width` = 17 + 4 × versi
- `level`: `L`, `M`, `Q`, atau `H`
- `modules`: `width` baris berisi `0`/`1` (`1` = modul gelap), tanpa zona tenang

## Kode keluar

`0` sukses, `1` gagal (input terlalu besar, file tidak terbaca), `2` penggunaan salah (input kosong, opsi tidak valid).

[Kembali ke dokumentasi](README.md).
