# json

```bash
dt json '{"b":1,"a":[1,2]}'          # pretty-print, indentasi 2 spasi
dt json -m -f data.json              # satu baris
dt json --indent 4 -s -f data.json   # indentasi 4, key diurutkan
dt json --check -f data.json         # valid / error baris:kolom
curl -s https://api.example.com | dt json -q 'items[0].name' -r
```

Input dari argumen teks, `--file <path>`, atau stdin; tanpa ketiganya di terminal
muncul prompt (ke stderr).

## Opsi

- `-m`, `--minify`: keluaran satu baris tanpa spasi. Tidak bisa dengan `--indent`.
- `--indent <n>`: lebar indentasi 0-8 (default 2). `0` sama dengan minify.
- `-s`, `--sort-keys`: urutkan key objek secara rekursif (urutan array tetap).
- `--check`: hanya validasi. Cetak `valid` (exit 0), atau `invalid: baris:kolom pesan`
  ke stderr (exit 1). Input terlalu dalam juga exit 1 (lihat Catatan).
  Tidak bisa digabung dengan opsi format atau `--query`.
- `-q`, `--query <path>`: ambil sebagian nilai. Sintaks: `a.b[0].c`; awalan `.` atau
  `$.` boleh dihilangkan; key khusus lewat `["key.dengan.titik"]` (atau `['...']`).
  `$` atau `.` berarti seluruh dokumen. Nama hanya cocok pada objek dan indeks
  `[n]` hanya pada array; `n` tanpa nol di depan dan paling besar 2^53-1
  (selain itu path rusak, exit 2).
- `-r`, `--raw`: dengan `--query`, string dicetak tanpa tanda kutip. Nilai lain
  tetap dicetak sebagai JSON (mengikuti `--minify`/`--indent`).

## Exit code

0 sukses; 1 JSON tidak valid/terlalu dalam atau `path tidak ditemukan: <path>`;
2 penggunaan salah (opsi bentrok, `--indent` di luar 0-8, path query rusak).

## Catatan

- Memakai `JSON.parse`: bilangan bulat di atas 2^53 (mis. ID 64-bit) kehilangan
  presisi dan angka seperti `1.0` ditulis ulang `1`. Jangan format ulang data
  yang bergantung pada angka besar.
- Key berupa bilangan bulat (`"2"`, `"10"`) selalu diurutkan lebih dulu oleh
  JavaScript, juga tanpa `--sort-keys`.
- Key duplikat: yang terakhir dipakai. BOM di awal input diabaikan.
- Posisi error selalu dicari dengan pemindai JSON ketat (bukan dari pesan `JSON.parse`,
  yang mengutip input). Kolom dihitung per karakter UTF-16.
- Nesting maksimal 2000 level (objek atau array), berlaku untuk semua mode,
  termasuk `--check` dan format biasa. Lebih dalam dari itu (atau saat format
  gagal karena tumpukan habis) dilaporkan `JSON terlalu dalam (maks 2000 level)`,
  exit 1, tanpa keluaran ke stdout. Pemindai dan pengurutan key tidak memakai
  rekursi, sehingga tidak memunculkan error stack overflow mentah.

[Kembali ke dokumentasi](README.md).
