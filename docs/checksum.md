# checksum

Hitung dan verifikasi checksum file dengan format `sha256sum`.

```bash
dt checksum a.bin b.bin              # <hex>  <path> per file
dt checksum -a sha512 a.bin > SUMS   # algoritma lain
dt checksum -c SUMS                  # verifikasi
```

## Opsi

- `-a, --algorithm <nama>`: `md5`, `sha1`, `sha224`, `sha256` (default), `sha384`,
  `sha512`, `sha3-256`, `sha3-512`, `blake2b512`. Output selalu hex.
- `-c, --check <sums>`: verifikasi file di daftar checksum. Tidak boleh disertai
  argumen file.
- `--json`: satu dokumen JSON ke stdout (lihat Keluaran JSON); ringkasan tetap ke stderr.
- `-h, --help`: bantuan.

File dibaca streaming, jadi file besar tidak dimuat ke memori.
Setiap file tetap diproses jika file lain gagal: digest yang berhasil dicetak,
sedangkan kegagalan dilaporkan ke stderr sebagai `<path>: <alasan>`. Exit akhir
bernilai 1 jika ada kegagalan.

## Mode verifikasi

Format baris: `<hex>  <path>` atau `<hex> *<path>` (penanda biner). Baris kosong
dan baris berawalan `#` diabaikan. Path relatif terhadap direktori file daftar,
bukan direktori kerja. Hex dibandingkan tanpa membedakan huruf besar/kecil.

Per file dicetak `<path>: OK`, `<path>: GAGAL`, `<path>: TIDAK ADA`, atau
`<path>: ERROR (<alasan>)` ke stdout. Error membaca file (termasuk direktori
dan path yang tidak valid) tidak menghentikan pemeriksaan file berikutnya.
Baris yang salah format dilaporkan ke stderr sebagai `baris N tidak valid`,
begitu juga ringkasan akhir dengan jumlah OK, GAGAL, TIDAK ADA, ERROR, dan
baris tidak valid. Panjang digest harus cocok dengan `--algorithm`;
daftar sha512 diverifikasi dengan `-a sha512`, kalau tidak semua barisnya tidak valid.
Daftar yang hilang atau tidak terbaca dilaporkan sebagai
`<sumsfile>: tidak dapat dibaca (<alasan>)` dan exit 1.

## Exit

0 semua cocok, 1 ada yang GAGAL, TIDAK ADA, ERROR, atau baris tidak valid (juga file
tidak terbaca, direktori, file non-reguler, atau daftar kosong), 2 penggunaan salah.

## Catatan

- Direktori dan file non-reguler (termasuk FIFO dan device seperti `/dev/zero`)
  ditolak sebelum dibaca. Symlink diikuti jika targetnya file biasa.
- `md5` dan `sha1` hanya untuk kompatibilitas, bukan keamanan.
- Checksum hanya membuktikan integritas jika daftarnya didapat dari sumber tepercaya.
- Path di daftar checksum digunakan sebagaimana diberikan: path absolut dan
  `..` diizinkan, seperti `sha256sum`; path relatif dimulai dari direktori
  file daftar. Karena daftar dapat menunjuk file di luar direktori tersebut,
  hanya verifikasi daftar checksum yang tepercaya.
- Baris bergaya BSD (`SHA256 (f) = hex`) dan nama yang di-escape dengan
  backslash tidak didukung; baris tersebut dianggap tidak valid.

## Keluaran JSON

Mode cetak: `{ algorithm, files: [{ path, digest }], errors: [{ path, message }] }`.
`files` berisi semua file yang berhasil dibaca; `errors` berisi setiap kegagalan
(kosong jika tidak ada). Dokumen tetap dicetak jika sebagian file gagal; exit 1.

Mode `--check`:
`{ algorithm, results: [{ path, status, error? }], malformed, ok, failed, missing, errors, passed }`.
`status` bernilai `ok`, `failed`, `missing`, atau `error`; hanya hasil dengan
status `error` menyertakan field `error` berisi alasan. `errors` adalah jumlah
hasil berstatus `error`, sedangkan `malformed` berisi nomor baris yang tidak valid.
`passed` bernilai `true` jika semuanya cocok. Kode exit tidak berubah
(1 bila `passed` false); ringkasan tetap dicetak ke stderr.

[Kembali ke dokumentasi](README.md).
