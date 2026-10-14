# Data csv

```bash
dt csv $'nama,umur\nani,30'    # teks CSV sebagai argumen
dt csv --file data.csv --select nama,3 --limit 10
dt csv --detect -F json --file export.csv
cat data.tsv | dt csv -d tab -F csv
dt csv --no-header --stats --file raw.csv
```

Membaca CSV dan menampilkannya sebagai tabel, JSON, TSV, atau CSV. Input dari
argumen, `--file`, atau stdin.

Parser mengikuti RFC 4180: field berkutip, `""` sebagai kutip, delimiter dan
newline di dalam kutip, akhir baris CRLF/LF (CR tunggal juga diterima), newline
akhir, dan satu BOM UTF-8 di awal dibuang (BOM kedua menjadi data). Baris kosong dilewati. Kutip di tengah field tak
berkutip dianggap karakter biasa; karakter apa pun antara kutip penutup dan
delimiter berikutnya adalah error. Kutip yang tak ditutup adalah error dengan
nomor baris tempat kutip dibuka.

Nomor baris pada pesan adalah urutan rekaman di input (header = 1, baris kosong
ikut dihitung, newline di dalam kutip tidak).

Opsi:

- `-d, --delimiter <d>`: `comma` (default), `semicolon`, `tab`, `pipe`, atau satu karakter
  (bukan `"` atau newline).
- `--detect`: tebak delimiter dari rekaman pertama (`,` `;` tab `|` di luar kutip; kutip hanya membuka
  kutipan di awal field, seperti parser; yang terbanyak menang, seri atau tidak ada = koma). Hasilnya dicetak ke stderr.
  Tidak bisa dengan `-d`.
- `--no-header`: opsi boolean biasa (bukan negasi); baris pertama diperlakukan
  sebagai data dan kolom bernama `c1..cn`.
- `-F, --format <fmt>`: `table` (default), `json`, `tsv`, `csv`.
- `--max-width <n>`: lebar maksimal sel pada tabel, 1-1000 (default 40); sel yang lebih
  lebar dipotong dan diakhiri `…`.
- `--select <kolom,kolom>`: pilih dan urutkan kolom menurut nama header persis atau
  nomor mulai 1 (nama didahulukan). Kolom tak ada = error exit 1; kolom yang sama
  dua kali = kesalahan penggunaan. Nama header yang mengandung koma tidak bisa
  dipilih menurut nama; pakai nomor.
- `--limit <n>`: hanya `n` baris data pertama (setelah header).
- `--stats`: cetak jumlah baris data, jumlah kolom, nomor baris tidak rata, dan jumlah
  sel kosong (`""`) per kolom, menggantikan keluaran data. Memperhatikan `--select`,
  mengabaikan `--limit`. Sel hasil padding ikut dihitung kosong. `-F json`/`--json`
  menghasilkan JSON; format lain menghasilkan teks.
- `--strict`: baris yang jumlah kolomnya berbeda dari header (atau dari baris pertama
  dengan `--no-header`) menjadi error exit 1, bukan peringatan.
- `--json`: sama dengan `-F json`; bertentangan dengan `--format` selain `json` = salah pakai.
- `--file <path>`: baca dari berkas.

## Format keluaran

- `table`: kolom rata menurut lebar tampilan (cluster grapheme lewat `Intl.Segmenter`;
  CJK dan emoji dihitung 2 kolom), dipisah `|`, garis pemisah di bawah header.
  Newline dalam sel ditampilkan `↵`. Dengan `--no-header` nama `c1..cn` tetap dicetak.
- `json`: array objek `{ header: nilai }`; semua nilai string. Dengan `--no-header` berupa
  array array. Header duplikat tidak bisa menjadi objek (error exit 1, gunakan `--select`
  atau `--no-header`).
- `tsv`: dipisah tab; `\`, tab, LF, CR dalam sel di-escape `\\`, `\t`, `\n`, `\r`.
- `csv`: selalu delimiter koma, sel di-quote bila perlu. Tanpa baris header bila `--no-header`.

Sel pada `table` dibersihkan untuk tampilan: CR/LF menjadi `↵`, tab menjadi spasi, dan
karakter kontrol C0/C1 lainnya (termasuk ESC, DEL, U+0080-U+009F) menjadi karakter
pengganti `�` (U+FFFD), agar tidak mengirim urutan escape terminal. Format `csv`, `tsv`, dan
`json` tetap setia pada isi sel (JSON meng-escape kontrol sendiri). Bendera (regional indicator)
dan emoji berpresentasi emoji dihitung 2 kolom; selector U+FE0F memilih presentasi emoji,
sedangkan U+FE0E memilih presentasi teks.

Catatan JSON objek: urutan kunci mengikuti aturan objek JavaScript, sehingga header berupa bilangan bulat
(mis. `2,1,nama`) diurutkan naik di depan kunci lain. Untuk urutan kolom asli pakai `-F tsv`
atau `-F csv`; `--no-header` menghasilkan array array (termasuk baris header asli sebagai data).

Catatan keamanan: keluaran `csv`/`tsv` tidak disanitasi untuk spreadsheet. Sel yang diawali `=`, `+`, `-`, atau
`@` dapat dijalankan sebagai rumus di Excel/LibreOffice; periksa dulu bila datanya tidak tepercaya.

## Baris tidak rata

Baris dengan jumlah kolom berbeda dilaporkan ke stderr
(`peringatan: baris 3 punya 2 kolom, diharapkan 3`, maksimal 20 lalu ringkasan) dan
dilengkapi sel kosong. Baris yang lebih panjang dari header melebarkan tabel; kolom
tambahan bernama `c<nomor>`; bila nama itu sudah dipakai header, nomor dinaikkan sampai unik.

## Keluaran JSON

`--json` / `-F json` menulis satu dokumen ke stdout:

- data: array objek (nama kolom → string), atau array array string dengan `--no-header`;
- dengan `--stats`: `{ rows, columns, ragged, columnStats }` dengan `ragged` array nomor
  baris tidak rata dan `columnStats` array `{ index, name, empty }` (`index` mulai 1).

Peringatan dan deteksi delimiter hanya ke stderr.

Exit: 0 sukses, 1 CSV tidak valid, kolom tidak ada, header duplikat untuk JSON, atau
`--strict` dilanggar, 2 penggunaan salah.

[Kembali ke dokumentasi](README.md).
