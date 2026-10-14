# color

Konversi dan analisis warna: hex, `rgb()`, `hsl()`, `hwb()`, dan nama CSS, plus luminans dan kontras WCAG 2.x.

```
dt color [opsi] <warna>
```

Warna diambil dari argumen (beberapa argumen digabung dengan spasi, jadi `rgb(255 0 0)` tanpa tanda kutip tetap
bekerja) atau dari stdin.

## Opsi

| Opsi                   | Keterangan                                                 |
| ---------------------- | ---------------------------------------------------------- |
| `-f, --format <jenis>` | Cetak hanya satu nilai: `hex`, `rgb`, `hsl`, `hwb`, `name` |
| `--contrast <warna2>`  | Rasio kontras antara `<warna>` dan `<warna2>`              |
| `--json`               | Keluaran JSON                                              |
| `-h, --help`           | Bantuan                                                    |

`--format` tidak bisa digabung dengan `--contrast` atau `--json`.

## Format masukan

- Hex: `#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa`. Tanpa `#` hanya diterima untuk 6 atau 8 digit.
- `rgb()` / `rgba()`: `rgb(255, 127, 80)`, `rgba(255,127,80,.5)`, `rgb(255 127 80 / 50%)`, `rgb(100% 50% 0%)`.
  Sintaks koma tidak boleh mencampur angka dan persen. Kanal di luar jangkauan dijepit ke 0-255.
- `hsl()` / `hsla()`: `hsl(16, 100%, 66%)`, `hsl(0.5turn 50% 50% / .3)`. Hue boleh berakhiran `deg`, `grad`, `rad`,
  `turn` (default derajat) dan dinormalkan ke 0-360. Saturasi dan lightness wajib berupa persen.
- `hwb()`: `hwb(16 31% 0%)`, hanya sintaks spasi (`/` untuk alpha).
- Nama CSS: 148 nama (tidak membedakan huruf besar/kecil) dan `transparent`.
- Alpha: angka 0-1 atau persen, dijepit ke 0-1 lalu dibulatkan sekali ke 3 desimal saat parsing.

Semua komponen numerik (hue, kanal, persen, dan alpha), termasuk hasil konversi satuannya, harus bernilai hingga
(`finite`). Nilai terlalu besar yang menghasilkan `Infinity` ditolak sebagai masukan tidak valid (kode keluar `2`),
bukan dicetak sebagai `NaN`.

## Keluaran

Tanpa `--format`, dicetak hex, rgb, hsl, hwb, nama, alpha, luminans, serta rasio kontras terhadap putih dan hitam
dengan verdikt AA/AAA untuk teks normal.

```
$ dt color coral
hex        #ff7f50
rgb        rgb(255, 127, 80)
hsl        hsl(16, 100%, 66%)
hwb        hwb(16 31% 0%)
nama       coral
alpha      1
luminans   0.3702
vs putih   2.49:1  AA gagal, AAA gagal
vs hitam   8.4:1  AA lulus, AAA lulus
```

### Pembulatan dan konvensi

- Kanal rgb adalah bilangan bulat 0-255 (dibulatkan). Masukan hsl/hwb dikonversi ke rgb lalu dibulatkan, sehingga
  `hsl(16, 100%, 66%)` menjadi `rgb(255, 128, 82)`, bukan `coral`; keluaran hsl/hwb selalu dihitung ulang dari rgb bulat.
- hsl dan hwb dicetak sebagai bilangan bulat (hue 0-359, persen 0-100).
- Alpha dibulatkan sekali ke 3 desimal saat parsing; nilai ini digunakan bersama untuk semua keluaran, pemilihan
  nama, dan pemeriksaan alpha < 1. Bila alpha < 1, hex menjadi 8 digit (`#rrggbbaa`, alpha dibulatkan ke byte),
  rgb/hsl dicetak sebagai `rgba()`/`hsla()` dan hwb dengan `/ alpha`.
  Contoh: `rgb(1 2 3 / 0.9999)` memiliki alpha `1` dan dicetak sebagai `#010203` serta `rgb(1, 2, 3)`.
- Nama: nama CSS persis bila ada (untuk sinonim dipilih yang pertama secara alfabet: `aqua`, `fuchsia`, `gray`, ...);
  bila tidak, nama terdekat dengan penanda `~` (jarak RGB kuadrat). `transparent` hanya untuk RGB hitam dengan alpha
  yang sudah dibulatkan ke `0`; misalnya `rgba(0,0,0,0.0001)` dicetak sebagai `#00000000` dan bernama `transparent`.
- Luminans relatif dan kontras memakai rumus WCAG 2.x dengan linearisasi sRGB; alpha diabaikan.
- Rasio kontras dipotong (bukan dibulatkan) ke 2 desimal agar tidak terlihat lulus padahal gagal. Ambang: teks normal
  AA 4.5, AAA 7; teks besar AA 3, AAA 4.5. Putih vs hitam = `21:1`.

## Kontras

```
$ dt color --contrast black white
Rasio kontras #ffffff vs #000000: 21:1
  Teks normal  AA (4.5): lulus   AAA (7): lulus
  Teks besar   AA (3):   lulus   AAA (4.5): lulus
```

## Keluaran JSON

`--json` mencetak satu dokumen JSON. Bentuk biasa:

- `input` (teks warna), `hex` (`#rrggbb` atau `#rrggbbaa`), `rgb` `{r, g, b}`, `hsl` `{h, s, l}`, `hwb` `{h, w, b}`
- `css` `{rgb, hsl, hwb}` (string CSS), `alpha` (0-1)
- `name` (nama CSS persis, atau `null`), `nearestName` (nama terdekat; sama dengan `name` bila persis)
- `luminance` (6 desimal)
- `contrast` `{white, black}`; masing-masing `{ratio, aa, aaa, aaLarge, aaaLarge}` (`ratio` dipotong 2 desimal,
  sisanya boolean)

Dengan `--contrast`: `foreground` `{input, hex}`, `background` `{input, hex}`, `ratio`, `aa`, `aaa`, `aaLarge`,
`aaaLarge`.

## Kode keluar

- `0` berhasil
- `2` warna atau opsi tidak valid
- `1` galat lain

[Kembali ke dokumentasi](README.md).
