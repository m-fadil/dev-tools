# image

Menampilkan metadata gambar tanpa dependensi. Header diparsing manual dengan
pemeriksaan batas ketat: file terpotong atau rusak menghasilkan error yang
menyebut nama file dan alasannya, tidak pernah membaca di luar buffer.

```
dt image [options] <file...>
```

Format dikenali dari magic bytes, bukan ekstensi: PNG, JPEG, GIF (87a/89a), WebP.
Format lain: `format gambar tidak dikenal: <file>`.

Opsi:

- `--json`: keluaran JSON (lihat di bawah).
- `-h, --help`: bantuan.

## Informasi yang ditampilkan

Semua format: `format`, `mime`, `size` (byte), `width`, `height`, `aspectRatio`
(pecahan disederhanakan, mis. `16:9`).

- **PNG**: `bitDepth`, `colorType` (`grayscale`, `truecolor`, `indexed`,
  `grayscale+alpha`, `truecolor+alpha`), `interlaced`, `alpha` (tipe warna beralfa
  atau ada chunk `tRNS`), `dpi` (dari `pHYs`, hanya bila satuannya meter),
  `textCount` (jumlah semua chunk teks yang terbaca) dan `textKeys` (maksimal 100
  kunci pertama chunk `tEXt`/`zTXt`/`iTXt`). CRC setiap chunk yang terbaca
  diperiksa, termasuk chunk ancillary; CRC yang salah menggagalkan file.
  Ini pemeriksaan metadata, bukan validasi decoding PNG lengkap: keberadaan
  `IDAT` tidak diwajibkan dan panjang payload `IEND` tidak diperiksa.
- **JPEG**: `precision`, `components`, `sof` (mis. `SOF0`), `progressive`,
  `jfif` (`version`, `density` `{x, y, unit}` dengan unit `none`/`dpi`/`dpcm`),
  `exif` (`orientation`, `make`, `model`, `software`, `dateTimeOriginal`; kedua
  urutan byte TIFF didukung, tag lain diabaikan). EXIF yang rusak tidak
  menggagalkan gambar: hasilnya berupa peringatan dan field yang sempat terbaca
  tetap dilaporkan.
- **GIF**: `version`, `colorTableSize` (warna di tabel global, 0 bila tidak ada),
  `frames`, `animated`, `loopCount` (dari `NETSCAPE2.0`; `0` = tanpa batas,
  `null` = tidak ada ekstensi).
- **WebP**: `variant` (`VP8`, `VP8L`, `VP8X`), `animated`, `alpha`, `icc`, `exif`,
  `xmp` (flag dari `VP8X`; `false` untuk varian lain), `frames` (jumlah chunk
  `ANMF` pada animasi, 1 untuk gambar diam), `loopCount` (chunk `ANIM`).

Karakter kontrol pada kunci teks PNG dan string EXIF diganti dengan U+FFFD
(`�`) pada keluaran teks maupun JSON. Tag EXIF yang rusak diperingatkan per tag;
tag lain dan ExifIFD yang penunjuknya sudah terbaca tetap diproses.
Hanya file biasa yang diterima; direktori, FIFO, dan perangkat ditolak sebelum
file dibuka.

## Batas baca

Maksimal 4 MiB pertama dibaca untuk PNG/JPEG/WebP dan 64 MiB untuk GIF (agar
frame bisa dihitung). Bila file lebih besar dan pemindaian berhenti sebelum
selesai, `truncated` bernilai `true` dan sebuah peringatan dicetak (`peringatan:`
pada teks, `warnings` pada JSON). Pada GIF/WebP animasi, `framesAtLeast` menjadi
`true` dan teks menampilkan `minimal N`; `animated` GIF berisi `null` bila belum
bisa dipastikan (kurang dari 2 frame terhitung). Pemindaian yang sudah mencapai
`IEND` (PNG), `SOS`/`EOI` (JPEG) atau trailer (GIF) tidak dianggap terpotong
walaupun file lebih besar. WebP diam tidak memindai chunk untuk menghitung frame:
`frames` tetap tepat 1, tanpa peringatan batas baca akibat metadata besar.

## Beberapa file

Semua file diproses. Pada teks, tiap file dicetak sebagai bagian yang diawali
`<path>:` dan dipisah baris kosong; file yang gagal dilaporkan ke stderr
(`<path>: <alasan>`).

Exit: 0 semua berhasil, 1 ada file yang tidak ditemukan/rusak/terpotong/tidak
dikenal, 2 opsi tidak valid atau tidak ada file.

## Keluaran JSON

Satu dokumen `{ "files": [...] }`, satu entri per argumen sesuai urutan, untuk satu
file maupun banyak. Pesan kegagalan tetap dicetak ke stderr dan exit 1.

- Gagal: `{ path, ok: false, error }` (`error` sama dengan pesan di stderr).
- Berhasil: `{ path, ok: true, format, mime, size, width, height, aspectRatio, ...detail format, truncated, warnings }`
  dengan field detail seperti di atas, dalam bentuk datar (`dpi` = `{x, y}` atau
  `null`; `exif`/`jfif` = objek atau `null`; `warnings` = daftar string).

Contoh:

```
dt image foto.jpg logo.png
dt image --json anim.gif | jq '.files[0].frames'
```

[Kembali ke dokumentasi](README.md).
